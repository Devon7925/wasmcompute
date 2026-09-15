//! Datapack-shaped Rust frontend. Source files keep their Minecraft function IDs.
use crate::{
    backend,
    wasm::{Module, Ty},
};
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub struct BuildOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub offline: bool,
    pub accurate: bool,
    pub copy_profile: Option<PathBuf>,
}

#[derive(Debug)]
struct Source {
    id: String,
    path: PathBuf,
    export: String,
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, PathBuf>) -> Result<()> {
    for item in fs::read_dir(dir)? {
        let item = item?;
        let ty = item.file_type()?;
        ensure!(
            !ty.is_symlink(),
            "symlinks are not supported in pack resources: {}",
            item.path().display()
        );
        if ty.is_dir() {
            walk(root, &item.path(), out)?;
        } else if ty.is_file() {
            out.insert(
                item.path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
                item.path(),
            );
        }
    }
    Ok(())
}

fn resource_id(path: &str) -> Result<Option<(String, &str)>> {
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() < 4 || parts[0] != "data" || parts[2] != "function" {
        return Ok(None);
    }
    let Some((stem, ext)) = path.rsplit_once('.') else {
        return Ok(None);
    };
    if !["rs", "mcfunction", "c", "cpp", "wasm"].contains(&ext) {
        return Ok(None);
    }
    let namespace = parts[1];
    let name = &stem["data/".len() + namespace.len() + "/function/".len()..];
    ensure!(
        !namespace.is_empty()
            && namespace
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b)),
        "invalid namespace in {path}"
    );
    ensure!(
        !name.is_empty()
            && name
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != "..")
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_./-".contains(&b)),
        "invalid function path: {path}"
    );
    Ok(Some((format!("{namespace}:{name}"), ext)))
}

fn inspect_metadata(bytes: &[u8]) -> Result<()> {
    let meta: Value = serde_json::from_slice(bytes).context("invalid pack.mcmeta")?;
    let pack = &meta["pack"];
    // This backend emits snapshot-only compute syntax. Never silently target an
    // older or unknown format while preserving incompatible resource metadata.
    let supported = |v: &Value| v == &json!(121) || v == &json!([121, 0]);
    ensure!(supported(&pack["min_format"]) && supported(&pack["max_format"]), "this compiler targets Minecraft 26.3-rc-1 only; pack min_format/max_format must both be 121 (or [121,0])");
    Ok(())
}

fn sdk_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let parent = exe.parent().unwrap();
    for p in [
        parent.join("../sdk/rust/minecraft"),
        parent.join("sdk/rust/minecraft"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/rust/minecraft"),
    ] {
        if p.join("Cargo.toml").is_file() {
            return Ok(p.canonicalize()?);
        }
    }
    bail!("bundled Rust SDK missing; keep the sdk/ directory beside the distributed bin/ directory")
}

fn run(cmd: &mut Command, context: &str) -> Result<Vec<u8>> {
    let out = cmd
        .output()
        .with_context(|| format!("could not start {context}"))?;
    ensure!(
        out.status.success(),
        "{context} failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(out.stdout)
}

fn rust_module(root: &Path, sources: &[Source], offline: bool) -> Result<Vec<u8>> {
    let generated = root.join("target/wasmcompute/entry");
    fs::create_dir_all(generated.join("src"))?;
    // TOML basic string escaping equals JSON escaping for these path strings.
    let quoted = |p: &Path| {
        let path = p.to_string_lossy();
        let path = if let Some(unc) = path.strip_prefix("\\\\?\\UNC\\") {
            format!("//{unc}")
        } else {
            path.strip_prefix("\\\\?\\").unwrap_or(&path).to_string()
        };
        serde_json::to_string(&path.replace('\\', "/")).unwrap()
    };
    let mut manifest = format!("[package]\nname = \"wasmcompute_entry\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n[workspace]\n[lib]\ncrate-type = [\"cdylib\"]\n[dependencies]\nminecraft = {{ path = {} }}\n", quoted(&sdk_path()?));
    let mut standard_library = false;
    if root.join("Cargo.toml").is_file() {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"));
        if offline {
            cmd.arg("--offline");
        }
        let metadata: Value = serde_json::from_slice(&run(&mut cmd, "Cargo helper metadata")?)?;
        let packages = metadata["packages"]
            .as_array()
            .context("Cargo returned no packages")?;
        let package = packages
            .iter()
            .find(|p| {
                p["manifest_path"]
                    .as_str()
                    .and_then(|s| Path::new(s).canonicalize().ok())
                    == root.join("Cargo.toml").canonicalize().ok()
            })
            .context(
                "source pack Cargo.toml must define a library package, not just a workspace",
            )?;
        ensure!(
            package["targets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["kind"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|k| k == "lib" || k == "rlib")),
            "source pack Cargo.toml must define a library for shared helpers"
        );
        let name = package["name"].as_str().unwrap();
        standard_library = package["metadata"]["wasmcompute"]["std"]
            .as_bool()
            .unwrap_or(false);
        ensure!(
            name != "minecraft",
            "helper package name minecraft conflicts with the bundled SDK"
        );
        manifest.push_str(&format!(
            "{} = {{ path = {} }}\n",
            serde_json::to_string(name)?,
            quoted(root)
        ));
        // Seed resolution with the author's lockfile; Cargo adds only generated
        // entry/SDK packages. Dependencies keep normal Cargo resolution rules.
        if root.join("Cargo.lock").is_file() {
            fs::copy(root.join("Cargo.lock"), generated.join("Cargo.lock"))?;
        }
    }
    manifest.push_str(
        "[profile.release]\npanic = \"abort\"\nlto = true\nopt-level = 3\ncodegen-units = 1\n",
    );
    fs::write(generated.join("Cargo.toml"), manifest)?;
    let mut glue = if standard_library {
        String::new()
    } else {
        String::from("#![no_std]\n#[panic_handler]\nfn panic(_: &core::panic::PanicInfo) -> ! { core::arch::wasm32::unreachable() }\n")
    };
    for source in sources {
        glue.push_str(&format!("#[path = {}]\nmod source_{};\n#[no_mangle]\npub extern \"C\" fn {}() -> i32 {{ source_{}::main() }}\n", quoted(&source.path), source.export, source.export, source.export));
    }
    fs::write(generated.join("src/lib.rs"), glue)?;
    let target = if let Some(custom) = std::env::var_os("CARGO_TARGET_DIR") {
        std::path::absolute(PathBuf::from(custom))?
    } else {
        let default = root.join("target/wasmcompute/cargo");
        if cfg!(windows) && default.as_os_str().len() > 140 {
            // MSVC host build scripts/proc macros can fail on long nested Cargo
            // paths even when Rust itself uses the extended Windows path form.
            let hash = root
                .to_string_lossy()
                .bytes()
                .fold(0xcbf29ce484222325u64, |h, b| {
                    (h ^ b as u64).wrapping_mul(0x100000001b3)
                });
            let short = std::env::temp_dir().join(format!("wc-{hash:016x}"));
            eprintln!("Using short Cargo artifact directory: {}", short.display());
            short
        } else {
            default
        }
    };
    let mut cmd = Command::new("cargo");
    cmd.current_dir(root)
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "-j2",
            "--manifest-path",
        ])
        .arg(generated.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target);
    // Export the ABI stack pointer so a trapping nested callback can unwind its
    // stack without rolling back guest globals/memory and command side effects.
    cmd.env("CARGO_ENCODED_RUSTFLAGS", "-C\x1ftarget-feature=-reference-types,-multivalue\x1f-C\x1flink-arg=--export=__stack_pointer");
    if offline {
        cmd.arg("--offline");
    }
    run(&mut cmd, "Rust source functions (see source paths below)")?;
    let bytes = fs::read(target.join("wasm32-unknown-unknown/release/wasmcompute_entry.wasm"))
        .context("Cargo produced no Wasm module")?;
    fs::write(generated.join("module.wasm"), &bytes)?;
    Ok(bytes)
}

fn add_function(files: &mut BTreeMap<String, String>, ns: &str, path: &str, lines: Vec<String>) {
    files.insert(
        format!("data/{ns}/function/{path}.mcfunction"),
        lines.join("\n") + "\n",
    );
}

/// Save only invocation scratch. Linear memory, page count and guest globals
/// remain shared. The external frame stack is separate from Wasm call frames.
fn public_boundary(
    pack: &mut backend::Pack,
    ns: &str,
    module: &Module<'_>,
    sources: &[Source],
    stack_global: Option<u32>,
) {
    let mut scores = BTreeSet::new();
    for (p, text) in &pack.files {
        if !p.ends_with(".mcfunction") {
            continue;
        }
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let tokens: Vec<_> = line.split_whitespace().collect();
            for pair in tokens.windows(2) {
                if pair[1] == ns && pair[0].starts_with('#') {
                    let name = &pair[0][1..];
                    let is_global = name
                        .strip_prefix('g')
                        .map(|s| s.strip_suffix('h').unwrap_or(s))
                        .map_or(false, |s| {
                            !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
                        });
                    if !is_global && name != "pages" {
                        scores.insert(name.to_string());
                    }
                }
            }
        }
    }
    let mut save = vec![];
    let mut restore = vec![];
    for (i, score) in scores.iter().enumerate() {
        save.push(format!("execute store result storage {ns}:vm external[0].scores.s{i} int 1 run scoreboard players get #{score} {ns}"));
        restore.push(format!("execute store result score #{score} {ns} run data get storage {ns}:vm external[0].scores.s{i}"));
    }
    for field in ["r", "host", "macro", "flush", "frames"] {
        save.push(format!(
            "data modify storage {ns}:vm external[0].{field} set from storage {ns}:vm {field}"
        ));
    }
    // Keep mutations to Wasm floating globals when replacing the caller's r.
    for (i, (ty, _)) in module
        .globals
        .iter()
        .enumerate()
        .filter(|(_, (t, _))| t.float())
    {
        let _ = ty;
        restore.push(format!(
            "data modify storage {ns}:vm external[0].r.g{i} set from storage {ns}:vm r.g{i}"
        ));
    }
    for field in ["r", "host", "macro", "flush", "frames"] {
        restore.push(format!(
            "data modify storage {ns}:vm {field} set from storage {ns}:vm external[0].{field}"
        ));
    }
    add_function(&mut pack.files, ns, "public/save", save);
    add_function(&mut pack.files, ns, "public/restore", restore);
    add_function(
        &mut pack.files,
        ns,
        "public/load",
        vec![
            format!("function {ns}:init"),
            format!("data modify storage {ns}:vm external set value []"),
            format!("scoreboard players set #public_depth {ns} 0"),
            format!("scoreboard players set #public_ready {ns} 1"),
        ],
    );
    for source in sources {
        let (public_ns, path) = source.id.split_once(':').unwrap();
        // Most invocations do not interrupt an existing guest call. Keep their
        // result and saved ABI stack pointer in scores, avoiding repeated NBT
        // mutations on the large VM compound. Nested calls retain the full
        // external-frame path below and never touch #public_stack.
        let mut root_entry = vec![];
        if let Some(g) = stack_global {
            root_entry.push(format!(
                "scoreboard players operation #public_stack {ns} = #g{g} {ns}"
            ));
        }
        root_entry.extend([
            format!("scoreboard players set #public_depth {ns} 1"),
            format!("function {ns}:export/{}", source.export),
        ]);
        if let Some(g) = stack_global {
            root_entry.push(format!(
                "scoreboard players operation #g{g} {ns} = #public_stack {ns}"
            ));
        }
        root_entry.extend([
            format!("scoreboard players set #public_depth {ns} 0"),
            format!("scoreboard players operation #public_result {ns} = #ret0 {ns}"),
            format!("scoreboard players operation #public_trap {ns} = #trap {ns}"),
            format!("execute unless score #public_trap {ns} matches 0 run return run function {ns}:public/trap_{}",source.export),
            format!("return run scoreboard players get #public_result {ns}"),
        ]);
        add_function(
            &mut pack.files,
            ns,
            &format!("public/root_{}", source.export),
            root_entry,
        );
        add_function(&mut pack.files, ns, &format!("public/trap_{}",source.export), vec![
            format!("data modify storage {ns}:diagnostics last_trap set value {{function:{},code:0}}",serde_json::to_string(&source.id).unwrap()),
            format!("execute store result storage {ns}:diagnostics last_trap.code int 1 run scoreboard players get #public_trap {ns}"),
            "return fail".into(),
        ]);
        let mut entry = vec![
            format!(
                "execute unless score #public_ready {ns} matches 1 run function {ns}:public/load"
            ),
            format!("execute if score #public_depth {ns} matches 0 run return run function {ns}:public/root_{}",source.export),
            format!("data modify storage {ns}:vm external prepend value {{scores:{{}}}}"),
            format!(
                "execute if score #public_depth {ns} matches 1.. run function {ns}:public/save"
            ),
        ];
        if let Some(g) = stack_global {
            entry.push(format!("execute store result storage {ns}:vm external[0].stack int 1 run scoreboard players get #g{g} {ns}"));
        }
        entry.extend([
            format!("scoreboard players add #public_depth {ns} 1"),
            format!("function {ns}:export/{}", source.export),
            format!("execute store result storage {ns}:vm external[0].result int 1 run scoreboard players get #ret0 {ns}"),
            format!("execute store result storage {ns}:vm external[0].trap int 1 run scoreboard players get #trap {ns}"),
            format!("execute if score #public_depth {ns} matches 2.. run function {ns}:public/restore"),
        ]);
        if let Some(g) = stack_global {
            entry.push(format!("execute store result score #g{g} {ns} run data get storage {ns}:vm external[0].stack"));
        }
        entry.extend([
            format!("scoreboard players remove #public_depth {ns} 1"),
            format!("execute store result score #public_result {ns} run data get storage {ns}:vm external[0].result"),
            format!("execute store result score #public_trap {ns} run data get storage {ns}:vm external[0].trap"),
            format!("data remove storage {ns}:vm external[0]"),
            format!("execute unless score #public_trap {ns} matches 0 run return run function {ns}:public/trap_{}",source.export),
            format!("return run scoreboard players get #public_result {ns}"),
        ]);
        add_function(&mut pack.files, public_ns, path, entry);
    }
}

fn inject_load(files: &mut BTreeMap<String, Vec<u8>>, ns: &str) -> Result<()> {
    let key = "data/minecraft/tags/function/load.json";
    let mut tag: Value = if let Some(bytes) = files.get(key) {
        serde_json::from_slice(bytes).context("invalid minecraft:load tag")?
    } else {
        json!({"values":[]})
    };
    tag.get_mut("values")
        .and_then(Value::as_array_mut)
        .context("minecraft:load tag values must be an array")?
        .insert(0, json!(format!("{ns}:public/load")));
    files.insert(key.into(), serde_json::to_vec_pretty(&tag)?);
    Ok(())
}

fn safe_output_path(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.is_empty()
            && relative
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains(['\\', ':'])),
        "invalid owned output path: {relative}"
    );
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        if let Ok(meta) = fs::symlink_metadata(&path) {
            ensure!(
                !meta.file_type().is_symlink(),
                "symlink in output: {}",
                path.display()
            );
        }
    }
    Ok(path)
}

fn write_output(root: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let mut previous = BTreeSet::new();
    if root.exists() && root.read_dir()?.next().is_some() {
        let old: Value = serde_json::from_slice(
            &fs::read(root.join("wasmcompute-build.json"))
                .context("refusing to overwrite non-source-build output")?,
        )?;
        for value in old["owned_files"]
            .as_array()
            .context("invalid previous build ownership")?
        {
            let p = value.as_str().context("invalid owned path")?;
            safe_output_path(root, p)?;
            previous.insert(p.to_string());
        }
        previous.insert("wasmcompute-build.json".into());
    }
    // Preflight every path before writing anything; do not overwrite unrelated
    // files added by the user after a prior build.
    for name in files.keys() {
        let path = safe_output_path(root, name)?;
        ensure!(
            !path.exists() || previous.contains(name),
            "output contains an unowned file: {name}"
        );
    }
    fs::create_dir_all(root)?;
    for (name, bytes) in files
        .iter()
        .filter(|(name, _)| name.as_str() != "wasmcompute-build.json")
    {
        let path = safe_output_path(root, name)?;
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, bytes)?;
    }
    for name in previous.iter().filter(|n| !files.contains_key(*n)) {
        let path = safe_output_path(root, name)?;
        if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    fs::write(
        root.join("wasmcompute-build.json"),
        &files["wasmcompute-build.json"],
    )?;
    Ok(())
}

fn resolve_output(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        ensure!(
            path.is_dir() && !fs::symlink_metadata(path)?.file_type().is_symlink(),
            "output must be a regular directory"
        );
        return Ok(path.canonicalize()?);
    }
    let parent = path.parent().context("output needs a parent directory")?;
    let name = path.file_name().context("invalid output path")?;
    Ok(resolve_output(parent)?.join(name))
}

pub fn build(options: BuildOptions) -> Result<Value> {
    let root = options
        .input
        .canonicalize()
        .context("source pack not found")?;
    let output = resolve_output(&std::path::absolute(&options.output)?)?;
    ensure!(
        !output.starts_with(&root),
        "output must be outside the source pack"
    );
    let mut paths = BTreeMap::new();
    if root.join("data").exists() {
        walk(&root, &root.join("data"), &mut paths)?;
    }
    for p in ["pack.mcmeta", "pack.png"] {
        if root.join(p).is_file() {
            paths.insert(p.into(), root.join(p));
        }
    }
    let mut files = BTreeMap::new();
    let mut sources = vec![];
    let mut identities = BTreeSet::new();
    for (path, source) in &paths {
        if let Some((id, ext)) = resource_id(path)? {
            ensure!(
                identities.insert(id.clone()),
                "multiple files define function {id}"
            );
            if ext != "mcfunction" {
                ensure!(ext == "rs", "frontend .{ext} is not implemented yet: {path}; use the standalone Wasm compiler for prebuilt modules");
                sources.push(Source {
                    id,
                    path: source.canonicalize()?,
                    export: format!("e{}", sources.len()),
                });
                continue;
            }
        }
        files.insert(path.clone(), fs::read(source)?);
    }
    inspect_metadata(
        files
            .get("pack.mcmeta")
            .context("source pack needs pack.mcmeta")?,
    )?;
    if sources.is_empty() {
        let report = json!({"schema":1,"compiler":env!("CARGO_PKG_VERSION"),"target":"26.3-rc-1","entries":{},"vanilla_semantics":"native","owned_files":files.keys().collect::<Vec<_>>()});
        files.insert(
            "wasmcompute-build.json".into(),
            serde_json::to_vec_pretty(&report)?,
        );
        write_output(&output, &files)?;
        return Ok(json!({"output":output,"functions":0,"files":files.len()}));
    }
    let bytes = rust_module(&root, &sources, options.offline)?;
    // Stable private identity from public IDs and guest bytes. Distinct builds
    // defining identical public IDs must not share private registers/memory.
    // This is namespace allocation,
    // not authentication; payload/function integrity is a separate mod concern.
    let mut hash = 0xcbf29ce484222325u64;
    for id in &identities {
        for b in id.bytes().chain([0]) {
            hash = (hash ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    for b in &bytes {
        hash = (hash ^ *b as u64).wrapping_mul(0x100000001b3);
    }
    if options.accurate {
        for b in b"accurate-v1" {
            hash = (hash ^ *b as u64).wrapping_mul(0x100000001b3);
        }
    }
    let ns = format!("w{:011x}", hash & 0x7ffffffffff);
    ensure!(
        !paths.keys().any(|p| p.starts_with(&format!("data/{ns}/"))),
        "private namespace collision: {ns}"
    );
    let module = Module::parse(&bytes).context(
        "unsupported Wasm produced by Rust; see generated project under target/wasmcompute/entry",
    )?;
    for source in &sources {
        let fid = *module
            .exports
            .get(&source.export)
            .with_context(|| format!("missing export for {}", source.id))?;
        ensure!(
            module.signature(fid) == &(vec![], vec![Ty::I32]),
            "{} must have pub fn main() -> i32",
            source.id
        );
    }
    let mut stack_global = None;
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let wasmparser::Payload::ExportSection(section) = payload? {
            for export in section {
                let export = export?;
                if export.name == "__stack_pointer"
                    && export.kind == wasmparser::ExternalKind::Global
                {
                    stack_global = Some(export.index);
                }
            }
        }
    }
    let copy_profile = options
        .copy_profile
        .as_ref()
        .map(|p| -> Result<Vec<backend::CopyRegion>> { Ok(serde_json::from_slice(&fs::read(p)?)?) })
        .transpose()?
        .unwrap_or_default();
    let module = if options.accurate {
        crate::accurate::lower(&module)?
    } else {
        module
    };
    let mut pack = backend::compile_with_profile(
        &module,
        &ns,
        true,
        Default::default(),
        false,
        true,
        copy_profile,
    )
    .context("compiling source pack to mcfunctions")?;
    public_boundary(&mut pack, &ns, &module, &sources, stack_global);
    let warnings = pack.warnings.clone();
    for (p, s) in pack.files {
        if p == "pack.mcmeta" {
            continue;
        }
        let p = if p == "wasmcompute.json" {
            format!("wasmcompute/{ns}/backend.json")
        } else {
            p
        };
        ensure!(!files.contains_key(&p), "generated output collision: {p}");
        files.insert(p, s.into_bytes());
    }
    inject_load(&mut files, &ns)?;
    use sha2::{Digest, Sha256};
    let module_hash = format!("{:x}", Sha256::digest(&bytes));
    files.insert(format!("wasmcompute/{ns}/module.wasm"), bytes);
    let entries: BTreeMap<_,_> = sources.iter().map(|s| (s.id.clone(), json!({"export":s.export,"source":s.path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\',"/")}))).collect();
    let mut accelerated_entries = serde_json::Map::new();
    for s in &sources {
        let path = format!(
            "data/{}/function/{}.mcfunction",
            s.id.split_once(':').unwrap().0,
            s.id.split_once(':').unwrap().1
        );
        accelerated_entries.insert(
            s.id.clone(),
            json!({"export":s.export,"sha256":format!("{:x}",Sha256::digest(&files[&path]))}),
        );
    }
    let init_path = format!("data/{ns}/function/public/load.mcfunction");
    accelerated_entries.insert(
        format!("{ns}:public/load"),
        json!({"init":true,"sha256":format!("{:x}",Sha256::digest(&files[&init_path]))}),
    );
    files.insert("wasmcompute-accelerator.json".into(),serde_json::to_vec_pretty(&json!({"schema":1,"target":"26.3-rc-1","namespace":ns,"module":format!("wasmcompute/{ns}/module.wasm"),"sha256":module_hash,"entries":accelerated_entries}))?);
    let report = json!({"schema":1,"compiler":env!("CARGO_PKG_VERSION"),"target":"26.3-rc-1","namespace":ns,"vanilla_semantics":if options.accurate {"accurate"} else {"fast"},"wasm_payload":"unmodified source Wasm; accurate Fabric accelerator metadata included","entries":entries,"warnings":warnings,"load_tag":"private initialization prepended; authored values retain order","owned_files":files.keys().collect::<Vec<_>>()});
    files.insert(
        "wasmcompute-build.json".into(),
        serde_json::to_vec_pretty(&report)?,
    );
    write_output(&output, &files)?;
    Ok(
        json!({"output":output,"functions":sources.len(),"files":files.len(),"namespace":ns,"warnings":warnings}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "wasmcompute-source-test-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn owned_files(path: &str) -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            (path.into(), b"generated".to_vec()),
            (
                "wasmcompute-build.json".into(),
                serde_json::to_vec(&json!({"owned_files":[path]})).unwrap(),
            ),
        ])
    }
    #[test]
    fn rebuild_removes_stale_generated_files_preserves_user_files() {
        let dir = TestDir::new();
        write_output(&dir.0, &owned_files("data/test/function/old.mcfunction")).unwrap();
        fs::write(dir.0.join("user.txt"), b"keep").unwrap();
        write_output(&dir.0, &owned_files("data/test/function/new.mcfunction")).unwrap();
        assert!(!dir.0.join("data/test/function/old.mcfunction").exists());
        assert_eq!(fs::read(dir.0.join("user.txt")).unwrap(), b"keep");
        assert!(dir.0.join("data/test/function/new.mcfunction").is_file());
    }
    #[test]
    fn refuses_unowned_overwrite_before_writing() {
        let dir = TestDir::new();
        write_output(&dir.0, &owned_files("old.txt")).unwrap();
        fs::write(dir.0.join("user.txt"), b"keep").unwrap();
        assert!(write_output(&dir.0, &owned_files("user.txt")).is_err());
        assert!(dir.0.join("old.txt").is_file());
        assert_eq!(fs::read(dir.0.join("user.txt")).unwrap(), b"keep");
    }
    #[test]
    fn validates_full_target_format() {
        for v in [
            json!(120),
            json!([121, 1]),
            json!([121]),
            json!([121, 0, 0]),
            json!("121"),
        ] {
            assert!(inspect_metadata(
                &serde_json::to_vec(&json!({"pack":{"min_format":v,"max_format":[121,0]}}))
                    .unwrap()
            )
            .is_err());
        }
    }
    #[test]
    fn resolves_not_yet_created_output_inside_source() {
        let dir = TestDir::new();
        assert!(resolve_output(&dir.0.join("not-created/dist"))
            .unwrap()
            .starts_with(dir.0.canonicalize().unwrap()));
    }
    #[test]
    fn function_identity() {
        assert_eq!(
            resource_id("data/demo/function/config/load.rs").unwrap(),
            Some(("demo:config/load".into(), "rs"))
        );
        assert!(resource_id("data/demo/function/Bad.rs").is_err());
        assert!(resource_id("data/demo/function/../oops.rs").is_err());
        assert!(resource_id("data/demo/advancement/a.json")
            .unwrap()
            .is_none());
    }
    #[test]
    fn load_tag_preserves_authored_semantics() {
        let mut files = BTreeMap::from([(
            "data/minecraft/tags/function/load.json".into(),
            br#"{"replace":true,"values":["demo:load",{"id":"optional:load","required":false}]}"#
                .to_vec(),
        )]);
        inject_load(&mut files, "wtest").unwrap();
        let tag: Value =
            serde_json::from_slice(&files["data/minecraft/tags/function/load.json"]).unwrap();
        assert_eq!(
            tag,
            json!({"replace":true,"values":["wtest:public/load","demo:load",{"id":"optional:load","required":false}]})
        );
    }
    #[test]
    fn rejects_output_traversal() {
        for p in ["../secret", "a/../../b", "C:/x", "a\\b", "/root"] {
            assert!(safe_output_path(Path::new("out"), p).is_err());
        }
    }
}
