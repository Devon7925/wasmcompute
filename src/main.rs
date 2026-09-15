use anyhow::Result;
use clap::Parser;
use std::{path::PathBuf, time::Instant};

#[derive(Parser)]
#[clap(
    version,
    about = "WebAssembly to native Minecraft 26.3 command functions"
)]
struct Args {
    /// Preserve supported Wasm scalar semantics with software floats and checks.
    #[clap(long)]
    accurate: bool,
    input: PathBuf,
    #[clap(short, long)]
    output: PathBuf,
    #[clap(long, default_value = "wc")]
    namespace: String,
    #[clap(short = 'O', default_value = "1")]
    opt: u8,
    #[clap(long)]
    bindings: Option<PathBuf>,
    /// Experimental integer word write-back buffer; workload dependent.
    #[clap(long)]
    word_buffer: bool,
    /// Use the previous NBT canonical memory representation.
    #[clap(long)]
    nbt_memory: bool,
    /// JSON array of guarded {src,dst,len} bulk-copy specializations.
    #[clap(long)]
    copy_profile: Option<PathBuf>,
}

#[derive(Parser)]
#[clap(version, about = "Build a datapack with Rust source functions")]
struct BuildArgs {
    /// Preserve supported Wasm scalar semantics with software floats and checks.
    #[clap(long)]
    accurate: bool,
    input: PathBuf,
    #[clap(short, long)]
    output: PathBuf,
    #[clap(long)]
    offline: bool,
    /// Optional guarded bulk-copy profile for this built Wasm module.
    #[clap(long)]
    copy_profile: Option<PathBuf>,
}

fn main() -> Result<()> {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("build")) {
        let args = BuildArgs::parse_from(
            std::iter::once(std::ffi::OsString::from("wasmcompute build"))
                .chain(std::env::args_os().skip(2)),
        );
        let report = wasmcompute::sourcepack::build(wasmcompute::sourcepack::BuildOptions {
            input: args.input,
            output: args.output,
            offline: args.offline,
            accurate: args.accurate,
            copy_profile: args.copy_profile,
        })?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let args = Args::parse();
    let start = Instant::now();
    let bytes = std::fs::read(&args.input)?;
    let module = wasmcompute::wasm::Module::parse(&bytes)?;
    let module = if args.accurate {
        wasmcompute::accurate::lower(&module)?
    } else {
        module
    };
    let bindings = if let Some(p) = args.bindings {
        serde_json::from_slice(&std::fs::read(p)?)?
    } else {
        Default::default()
    };
    let copy_profile = if let Some(p) = args.copy_profile {
        serde_json::from_slice(&std::fs::read(p)?)?
    } else {
        vec![]
    };
    let pack = wasmcompute::backend::compile_with_profile(
        &module,
        &args.namespace,
        args.opt != 0,
        bindings,
        args.word_buffer,
        !args.nbt_memory,
        copy_profile,
    )?;
    pack.write(&args.output)?;
    println!(
        "{}",
        serde_json::json!({"output":args.output,"compile_ms":start.elapsed().as_secs_f64()*1000.,"files":pack.files.len(),"commands":pack.command_count(),"warnings":pack.warnings})
    );
    Ok(())
}
