use wasmcompute::{
    accurate, compile,
    wasm::{Module, Num, Ty},
};

#[test]
fn accurate_preserves_nan_global_bits_and_original_indirect_types() {
    let bytes = wat::parse_str(
        r#"(module
      (type $f (func (param f32) (result f32)))
      (type $i (func (param i32) (result i32)))
      (global f32 (f32.const nan:0x212345))
      (table 1 funcref) (elem (i32.const 0) $integer)
      (func $integer (type $i) local.get 0)
      (func (export "wrong") (type $f) local.get 0 i32.const 0 call_indirect (type $f)))"#,
    )
    .unwrap();
    let original = Module::parse(&bytes).unwrap();
    let m = accurate::lower(&original).unwrap();
    assert_eq!(m.globals[0], (Ty::I32, Num::I(0x7fa12345)));
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    assert!(!pack.files["data/test/function/table/t0.mcfunction"].contains("f0/call"));
    assert!(!m.original_types.is_empty());
}

#[test]
fn accurate_bounds_check_before_cached_memory_and_bulk_access() {
    let bytes = wat::parse_str(
        r#"(module (memory 1)
      (func (export "read") (param i32) (result i32) local.get 0 i32.load)
      (func (export "copy") (param i32 i32 i32) local.get 0 local.get 1 local.get 2 memory.copy))"#,
    )
    .unwrap();
    let original = Module::parse(&bytes).unwrap();
    let m = accurate::lower(&original).unwrap();
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    assert!(pack
        .files
        .values()
        .any(|s| s.contains("bounds_limit") && s.contains("trap_memory")));
    let fast = compile(&original, "test", true, Default::default()).unwrap();
    assert!(!fast.files.values().any(|s| s.contains("trap_memory")));
}

#[test]
fn accurate_rejects_bad_initial_segments_and_unknown_float_hosts() {
    for wat in [
        r#"(module (memory 0) (data (i32.const 0) "x"))"#,
        r#"(module (import "other" "float" (func (param f32))))"#,
    ] {
        let bytes = wat::parse_str(wat).unwrap();
        assert!(accurate::lower(&Module::parse(&bytes).unwrap()).is_err());
    }
}

#[test]
fn accurate_float_lowering_emits_only_integer_runtime() {
    let bytes = wat::parse_str(
        r#"(module (func (export "test") (param f64 f64) (result f64)
        local.get 0 local.get 1 f64.add f64.sqrt))"#,
    )
    .unwrap();
    let m = accurate::lower(&Module::parse(&bytes).unwrap()).unwrap();
    let pack = compile(&m, "test", true, Default::default()).unwrap();
    assert!(pack.warnings.is_empty());
    assert!(!pack
        .files
        .values()
        .any(|s| s.contains("compute default float")));
    assert_eq!(
        m.signature(m.exports["test"]),
        &(vec![Ty::I64, Ty::I64], vec![Ty::I64])
    );
}
