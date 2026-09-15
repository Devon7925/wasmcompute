use std::{env, fs, path::PathBuf, process::Command};
fn run(c: &mut Command) {
    assert!(
        c.status().expect("launch C++ toolchain").success(),
        "failed: {c:?}"
    );
}
fn main() {
    println!("cargo:rustc-check-cfg=cfg(celeste_blocks)");
    println!("cargo:rerun-if-env-changed=CELESTE_BLOCKS");
    if env::var("CELESTE_BLOCKS").as_deref() == Ok("1") {
        println!("cargo:rustc-cfg=celeste_blocks");
    }
    println!("cargo:rustc-check-cfg=cfg(celeste_atlas)");
    println!("cargo:rerun-if-env-changed=CELESTE_RASTER");
    let raster_mode = env::var("CELESTE_RASTER").unwrap_or_else(|_| "cached".into());
    assert!(["reference", "rects", "atlas", "cached"].contains(&raster_mode.as_str()));
    if raster_mode == "atlas" || raster_mode == "cached" {
        println!("cargo:rustc-cfg=celeste_atlas");
    }
    println!("cargo:rustc-check-cfg=cfg(celeste_map_cache)");
    if raster_mode == "cached" {
        println!("cargo:rustc-cfg=celeste_map_cache");
    }
    println!("cargo:rerun-if-env-changed=CELESTE_RASTER_TEST");
    let raster_test = env::var("CELESTE_RASTER_TEST").as_deref() == Ok("1");
    let source = PathBuf::from(
        env::var_os("CELESTE_SOURCE").expect("set CELESTE_SOURCE to the pinned cceleste checkout"),
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut assets = String::new();
    for name in ["gfx", "font"] {
        let path = source.join(format!("data/{name}.bmp"));
        let b = fs::read(&path).unwrap();
        let u32at = |i| u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as usize;
        let (offset, w, h) = (u32at(10), u32at(18), u32at(22));
        let depth = b[28] as usize;
        assert!(w == 128 && h >= 64 && (depth == 1 || depth == 4));
        let stride = ((w * depth + 31) / 32) * 4;
        assets.push_str(&format!(
            "static const unsigned char {name}_pixels[8192]={{"
        ));
        let mut raw_pixels = Vec::new();
        for y in 0..64 {
            for x in 0..w {
                let byte = b[offset + (h - 1 - y) * stride + x * depth / 8];
                let value = (byte >> (8 - depth - (x * depth % 8))) & ((1 << depth) - 1);
                raw_pixels.push(value);
                assets.push_str(&format!("{value},"));
            }
        }
        assets.push_str("};\n");
        fs::write(out.join(format!("{name}.bin")), raw_pixels).unwrap();
        println!("cargo:rerun-if-changed={}", path.display());
    }
    fs::write(out.join("assets.h"), assets).unwrap();
    // Extract the reference rasterizer, excluding its platform main loop and
    // alternate celeste2.c API. Optional batching replaces drawing algorithms;
    // the game code and compiler optimization flags remain unchanged.
    let mc = fs::read_to_string(source.join("mcmain.c"))
        .unwrap()
        .replace("\r\n", "\n");
    let sdl = fs::read_to_string(source.join("mc_sdl_compat.h"))
        .unwrap()
        .replace("\r\n", "\n");
    let raster = &mc[mc.find("static int gettileflag(int, int);").unwrap()
        ..mc.find("\nvoid P8music(").unwrap()];
    let raster_owned=raster.replace("palette[a] = base_palette[b];", "SetPalette(a,b);");
    let raster=raster_owned.as_str();
    let line = &mc[mc
        .find("static int gettileflag(int tile, int flag) {")
        .unwrap()..mc.find("\n#if SDL_MAJOR_VERSION >= 2\n//SDL2:").unwrap()];
    let primitives = &sdl[sdl.find("void SDL_SetPixel_screen_palette(").unwrap()
        ..sdl.find("\n#define SDL_INIT_AUDIO").unwrap()];
    let raster = if raster_mode == "atlas" || raster_mode == "cached" {
        let split = raster.find("//lots of code from").unwrap();
        let reference = raster[..split]
            .replace("void _blitter(", "void reference_blitter(")
            .replace("void _blitter_masked(", "void reference_blitter_masked(");
        format!(
            "{reference}\n#include \"batched-blit.inc\"\n{}",
            &raster[split..]
        )
    } else {
        raster.to_string()
    };
    let raster = if raster_mode == "cached" {
        let start = raster.find("case CELESTE_P8_MAP:").unwrap();
        let mut map = raster[start..].to_string();
        map=map.replacen("int mask = INT_ARG();", "int mask = INT_ARG();\nint targetx=tx-camera_x,targety=ty-camera_y;\nint cached=direct_map_begin(mx,my,mw,mh,mask);\nif(cached==0){direct_map_end(targetx,targety);break;}\nif(cached==1){tx=camera_x;ty=camera_y;}",1);
        let end = map.find("} break;").unwrap();
        map.insert_str(end, "if(cached==1)direct_map_end(targetx,targety);\n");
        format!("extern \"C\" int direct_map_begin(int,int,int,int,int);\nextern \"C\" void direct_map_end(int,int);\n{}{}",&raster[..start],map)
    } else {
        raster
    };
    fs::write(out.join("upstream-raster.inc"), format!("{raster}\n{line}")).unwrap();
    let primitives = if raster_mode != "reference" {
        let split = primitives
            .find("void SDL_FillRect_screen_palette(")
            .unwrap();
        format!("{}\n#include \"batched-rect.inc\"", &primitives[..split])
    } else {
        primitives.to_string()
    };
    fs::write(out.join("upstream-primitives.inc"), primitives).unwrap();
    let libc = root.join("../celeste-common/libc.inc");
    fs::copy(&libc, out.join("libc.inc")).unwrap();
    let mut cmd = Command::new(env::var_os("CLANGXX").unwrap_or_else(|| "clang++".into()));
    cmd.args([
        "--target=wasm32",
        "-O3",
        "-std=c++20",
        "-nostdlib",
        "-fno-exceptions",
        "-fno-rtti",
        "-fno-builtin",
        "-fno-threadsafe-statics",
        "-Wno-deprecated",
        "-Wno-macro-redefined",
        "-c",
    ])
    .arg(root.join("raster.cpp"))
    .arg("-I")
    .arg(&root)
    .arg("-I")
    .arg(root.join("../celeste-common/include"))
    .arg("-I")
    .arg(&out)
    .arg(format!(
        "-DCELESTE_SOURCE=\"{}\"",
        source
            .join("celeste.c")
            .to_string_lossy()
            .replace('\\', "/")
    ))
    .arg(format!(
        "-DCELESTE_TILEMAP=\"{}\"",
        source
            .join("tilemap.h")
            .to_string_lossy()
            .replace('\\', "/")
    ))
    .arg("-o")
    .arg(out.join("celeste.o"));
    if env::var("CELESTE_FIXED").as_deref() == Ok("1") {
        cmd.arg("-DCELESTE_P8_FIXEDP");
    }
    if raster_test {
        cmd.arg("-DCELESTE_RASTER_TEST");
    }
    run(&mut cmd);
    run(
        Command::new(env::var_os("LLVM_AR").unwrap_or_else(|| "llvm-ar".into()))
            .arg("crs")
            .arg(out.join("libceleste.a"))
            .arg(out.join("celeste.o")),
    );
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=celeste");
    for p in [
        root.join("raster.cpp"),
        root.join("batched-rect.inc"),
        root.join("batched-blit.inc"),
        source.join("celeste.c"),
        source.join("tilemap.h"),
        libc,
    ] {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    for p in [source.join("mcmain.c"), source.join("mc_sdl_compat.h")] {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    for v in ["CELESTE_SOURCE", "CELESTE_FIXED", "CLANGXX", "LLVM_AR"] {
        println!("cargo:rerun-if-env-changed={v}");
    }
}
