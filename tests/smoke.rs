use wasmcompute::{compile, wasm::Module};

#[test]
fn complete_forward_copy_pattern_uses_guarded_bulk_copy() {
    let bytes = include_bytes!("../src/patterns/forward-copy.wasm");
    let mut m = Module::parse(bytes).unwrap();
    m.exports.insert("arbitrary_name".into(), 0);
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    assert!(pack
        .files
        .contains_key("data/test/function/rt/memory_copy.mcfunction"));
    // Unoptimized mode retains the reference loop.
    let reference = compile(&m, "test", false, Default::default()).unwrap();
    assert!(!reference
        .files
        .contains_key("data/test/function/rt/memory_copy.mcfunction"));
}

#[test]
fn arithmetic_and_control_compile() {
    let bytes = wat::parse_str(
        r#"(module
      (func (export "sum") (param i32) (result i32) (local i32)
        block loop local.get 0 i32.eqz br_if 1
          local.get 1 local.get 0 i32.add local.set 1
          local.get 0 i32.const 1 i32.sub local.set 0 br 0
        end end local.get 1)
      (func (export "float") (param f32) (result f32)
        local.get 0 f32.const 1.5 f32.mul f32.const 0.25 f32.add))"#,
    )
    .unwrap();
    let m = Module::parse(&bytes).unwrap();
    let pack = compile(&m, "smoke", true, Default::default()).unwrap();
    assert!(pack
        .files
        .values()
        .any(|s| s.contains("set compute default float")));
    assert!(pack
        .files
        .values()
        .any(|s| s.contains("return run function")));
}

#[test]
fn malformed_input_rejected() {
    assert!(Module::parse(b"not wasm").is_err());
}

#[test]
fn unknown_import_is_error() {
    let bytes =
        wat::parse_str(r#"(module (import "env" "missing" (func)) (func (export "run") call 0))"#)
            .unwrap();
    let m = Module::parse(&bytes).unwrap();
    assert!(compile(&m, "x", true, Default::default()).is_err());
}

#[test]
fn byte_load_arithmetic_is_one_provider_tree() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
      (func (export "f") (result i32)
        i32.const 64 i32.load8_u i32.const 2 i32.mul
        i32.const 65 i32.load8_u i32.add))"#,
    )
    .unwrap();
    let module = Module::parse(&bytes).unwrap();
    let pack = compile(&module, "test", true, Default::default()).unwrap();
    let entry = &pack.files["data/test/function/f0/entry.mcfunction"];
    assert_eq!(entry.matches("compute default integer").count(), 1);
    assert!(
        entry.contains("#m16")
            && entry.contains("\"type\":\"mul\"")
            && entry.contains("\"type\":\"add\"")
    );
}

#[test]
fn unbounded_integer_addition_keeps_wrapping_semantics() {
    let bytes = wat::parse_str(
        r#"(module (func (export "f") (param i32 i32) (result i32)
      local.get 0 local.get 1 i32.add))"#,
    )
    .unwrap();
    let module = Module::parse(&bytes).unwrap();
    let pack = compile(&module, "test", true, Default::default()).unwrap();
    let entry = &pack.files["data/test/function/f0/entry.mcfunction"];
    assert!(entry.contains(" += "));
    assert!(!entry.contains("compute default integer"));
}

#[test]
fn wrapper_inlining_exposes_constant_address_across_branch() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
      (func $read (param i32) (result i32) local.get 0 i32.load)
      (func (export "f") (param i32) (result i32) (local i32)
       i32.const 4096 local.set 1
       local.get 0 if nop end
       local.get 1 call $read))"#,
    )
    .unwrap();
    let m = Module::parse(&bytes).unwrap();
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    assert!(!pack
        .files
        .contains_key("data/test/function/f0/call.mcfunction"));
    assert!(!pack.files.values().any(|s| s.contains("$(p)")));
    assert!(pack.files.values().any(|s| s.contains("#m1024")));
}

#[test]
fn readonly_branch_reuses_dynamic_load() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
      (func (export "f") (param i32 i32) (result i32) (local i32)
       local.get 0 i32.load local.set 2
       local.get 1 if nop end
       local.get 0 i32.load local.get 2 i32.add))"#,
    )
    .unwrap();
    let m = Module::parse(&bytes).unwrap();
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    let code: String = pack
        .files
        .iter()
        .filter(|(p, _)| p.starts_with("data/test/function/f0/"))
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(code.matches("function test:local/f0/load32").count(), 1);
}

#[test]
fn memory_macro_caches_are_separate_and_small() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
      (func (export "a") (param i32) (result i32) local.get 0 i32.load)
      (func (export "b") (param i32) (result i32) local.get 0 i32.load))"#,
    )
    .unwrap();
    let m = Module::parse(&bytes).unwrap();
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    for fid in 0..2 {
        let text = &pack.files[&format!("data/test/function/local/f{fid}/word_read.mcfunction")];
        assert_eq!(text, "$return run scoreboard players get #m$(p) test_mem\n");
    }
}

#[test]
fn legacy_nbt_memory_remains_selectable() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
        (func (export "read") (param i32) (result i32) local.get 0 i32.load))"#,
    )
    .unwrap();
    let m = Module::parse(&bytes).unwrap();
    let pack = wasmcompute::backend::compile_with_memory(
        &m,
        "test",
        true,
        Default::default(),
        false,
        false,
    )
    .unwrap();
    assert_eq!(
        pack.files["data/test/function/local/f0/word_read.mcfunction"],
        "$return run data get storage test:mem w.a$(p)\n"
    );
    assert!(!pack.files["data/test/function/init.mcfunction"].contains("test_mem"));
}

#[test]
fn constant_float_addresses_do_not_use_dynamic_memory_macros() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
        (func (export "read") (result f32) i32.const 64 f32.load)
        (func (export "write") (param f32) i32.const 64 local.get 0 f32.store))"#,
    )
    .unwrap();
    let m = Module::parse(&bytes).unwrap();
    for score_memory in [false, true] {
        let pack = wasmcompute::backend::compile_with_memory(
            &m,
            "test",
            true,
            Default::default(),
            false,
            score_memory,
        )
        .unwrap();
        assert!(pack
            .files
            .contains_key("data/test/function/rt/load_f32_at_16.mcfunction"));
        assert!(!pack.files.values().any(|s| s.contains("$(p)")));
    }
}
