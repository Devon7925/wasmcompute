(module
  (memory 1)
  (func (export "read") (param i32) (result i32)
    block
      i32.const 0 local.get 0 f32.convert_i32_s f32.store
    end
    ;; Copy a dirty float word, then overwrite the original through integer memory.
    i32.const 4 i32.const 0 i32.const 4 memory.copy
    i32.const 0 i32.const 42 i32.store
    i32.const 4 i32.load)
  (func (export "overwrite") (param i32) (result i32)
    i32.const 0 f32.const 1.25 f32.store
    i32.const 0 local.get 0 i32.store
    i32.const 0 f32.load i32.reinterpret_f32))
