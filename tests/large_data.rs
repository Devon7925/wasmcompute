use wasmcompute::{compile, wasm::Module};

#[test]
fn large_initial_image_is_compact_and_constant_reads_are_seeded() {
    let data = "\\12\\34\\56\\78".repeat(32768);
    let wat = format!(
        r#"(module (memory 3) (data (i32.const 0) "{data}")
      (func (export "dynamic") (param i32) (result i32) local.get 0 i32.load)
      (func (export "static") (result i32) i32.const 1024 i32.load)
      (func (export "write") (param i32 i32) local.get 0 local.get 1 i32.store)
      (func (export "copy") (param i32 i32 i32) local.get 0 local.get 1 local.get 2 memory.copy))"#
    );
    let bytes = wat::parse_str(wat).unwrap();
    let module = Module::parse(&bytes).unwrap();
    let pack = compile(&module, "large", true, Default::default()).unwrap();
    let init = &pack.files["data/large/function/init.mcfunction"];
    assert!(init.lines().count() < 200);
    assert!(init.contains("scoreboard players set #m256 large_mem 2018915346"));
    assert!(!init.contains("scoreboard players set #m32767 large_mem"));
    let reader = &pack.files["data/large/function/rt/word_read.mcfunction"];
    assert!(reader.contains("matches -2147483648..2147483647"));
    assert!(reader.contains("rt/rom_read"));
    assert!(pack.files["data/large/function/rt/copy_pair.mcfunction"].contains("rt/word_read"));
}
