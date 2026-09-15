use wasmcompute::{
    accurate, compile,
    wasm::{Module, Num, Ty},
};

#[test]
fn fast_nonfinite_constants_are_explicitly_approximate_and_accurate_bits_survive() {
    let bytes = wat::parse_str(
        r#"(module
      (global f64 (f64.const inf))
      (global f64 (f64.const -inf))
      (global f32 (f32.const nan:0x212345))
      (func (export "value") (result f64) f64.const inf))"#,
    )
    .unwrap();
    let original = Module::parse(&bytes).unwrap();
    let fast = compile(&original, "fast", true, Default::default()).unwrap();
    assert!(fast
        .warnings
        .iter()
        .any(|s| s.contains("maps NaN constants to zero")));
    let exact = accurate::lower(&original).unwrap();
    assert_eq!(exact.globals[0], (Ty::I64, Num::I(0x7ff0000000000000)));
    assert_eq!(
        exact.globals[1],
        (Ty::I64, Num::I(0xfff0000000000000u64 as i64))
    );
    assert_eq!(exact.globals[2], (Ty::I32, Num::I(0x7fa12345)));
    let pack = compile(&exact, "exact", true, Default::default()).unwrap();
    assert!(!pack
        .warnings
        .iter()
        .any(|s| s.contains("maps NaN constants to zero")));
}
