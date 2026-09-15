use wasmcompute::{compile, wasm::Module};

#[test]
fn float_locals_do_not_enable_memory_cache_but_nested_stores_do() {
    for store in [false, true] {
        let code = if store {
            "block local.get 0 f32.const 1.25 f32.store end"
        } else {
            "local.get 0 f32.convert_i32_s drop"
        };
        let wasm = wat::parse_str(format!(
            "(module (memory 1) (func (export \"run\") (param i32) (result i32) (local f32) {code} local.get 0 i32.load))"
        )).unwrap();
        let pack = compile(
            &Module::parse(&wasm).unwrap(),
            "test",
            true,
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            pack.files.values().any(|s| s.contains(":fmem w.a$(p).d")),
            store
        );
    }
}

#[test]
fn unsigned_or_bounds_fuse_and_prove_coordinate_arithmetic() {
    let wasm = wat::parse_str(
        r#"(module (func (export "pixel") (param i32 i32) (result i32)
      block local.get 0 local.get 1 i32.or i32.const 127 i32.gt_u br_if 0
      local.get 1 i32.const 128 i32.mul local.get 0 i32.add i32.const 10000001 i32.add return
      end i32.const -1))"#,
    )
    .unwrap();
    let m = Module::parse(&wasm).unwrap();
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    let all = pack.files.values().cloned().collect::<String>();
    assert!(!all.contains("or_nibble"));
    assert!(!pack
        .files
        .contains_key("data/test/function/rt/i32_or.mcfunction"));
    assert!(all.contains("matches 0..127"));
    assert!(all.contains("\"inputs\"") && all.contains("10000001"));
}

#[test]
fn alpha_mask_is_a_small_exact_provider() {
    let wasm = wat::parse_str(
        r#"(module (func (export "alpha") (param i32) (result i32)
      local.get 0 i32.const -16777216 i32.or))"#,
    )
    .unwrap();
    let pack = compile(
        &Module::parse(&wasm).unwrap(),
        "test",
        true,
        Default::default(),
    )
    .unwrap();
    let entry = &pack.files["data/test/function/f0/entry.mcfunction"];
    assert!(entry.contains("floor_mod") && entry.contains("16777216"));
    assert!(entry.len() < 1000);
}

#[test]
fn bitwise_is_native_for_both_word_widths() {
    for ty in ["i32", "i64"] {
        let wasm=wat::parse_str(format!(r#"(module (func (export "bits") (param {ty} {ty}) (result {ty}) local.get 0 local.get 1 {ty}.xor))"#)).unwrap();
        let pack = compile(
            &Module::parse(&wasm).unwrap(),
            "test",
            true,
            Default::default(),
        )
        .unwrap();
        assert!(!pack.files.values().any(|s| s.contains("xor_nibble")));
    }
}
