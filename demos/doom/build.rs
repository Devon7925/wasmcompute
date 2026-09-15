use std::{env,fs,path::PathBuf,process::Command};
fn run(c:&mut Command){assert!(c.status().expect("launch C toolchain").success(),"failed: {c:?}");}
fn main(){
 let root=PathBuf::from(env!("CARGO_MANIFEST_DIR"));let out=PathBuf::from(env::var_os("OUT_DIR").unwrap());
 let wad=PathBuf::from(env::var_os("DOOM_WAD").expect("set DOOM_WAD to a Doom shareware IWAD"));
 let bytes=fs::read(&wad).expect("read IWAD");assert!(&bytes[..4]==b"IWAD");fs::write(out.join("doom.wad"),bytes).unwrap();
 let mut objects=Vec::new();
 for name in include_str!("engine-sources.txt").lines(){
  let mut source=root.join("engine").join(name);
  if name=="wasmdoom.c" {
   // The immutable embedded IWAD can be bound directly; no staging copy or
   // extra memory.grow is necessary. Engine gameplay is unchanged.
   let text=fs::read_to_string(&source).unwrap().replace("static int wd_wad_len = 0;","static int wd_wad_len = 0;\nvoid doom_bind_wad(const unsigned char *data,int len){wd_wad_buf=(unsigned char*)data;wd_wad_len=len;}");
   source=out.join(name);fs::write(&source,text).unwrap();
  }
  let object=out.join(name.replace(".c",".o"));
  run(Command::new(env::var_os("CLANG").unwrap_or_else(||"clang".into())).args(["--target=wasm32","-O3","-std=c17","-DNORMALUNIX","-DLINUX","-Wno-everything","-fno-sanitize=undefined","-nostdlib","-I"]).arg(root.join("engine")).arg("-include").arg(root.join("engine/wd_libc.h")).arg("-c").arg(source).arg("-o").arg(&object));
  objects.push(object);
 }
 run(Command::new(env::var_os("LLVM_AR").unwrap_or_else(||"llvm-ar".into())).arg("crs").arg(out.join("libdoom.a")).args(objects));
 println!("cargo:rustc-link-search=native={}",out.display());println!("cargo:rustc-link-lib=static=doom");
 println!("cargo:rerun-if-changed=engine");println!("cargo:rerun-if-changed=engine-sources.txt");println!("cargo:rerun-if-changed={}",wad.display());
 for key in ["DOOM_WAD","CLANG","LLVM_AR"]{println!("cargo:rerun-if-env-changed={key}");}
}
