(module
  (memory 1)
  (func (export "read") (param i32) (result i32)
    i32.const 0 local.get 0 i32.store
    i32.const 0 f32.load i32.reinterpret_f32))
