(module (import "env" "sinf" (func $sinf (param f32) (result f32)))(import "env" "fmodf" (func $fmodf (param f32 f32) (result f32))) (memory 1 2)  (func $memcpy (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
  (local $3 i32)
  (local $4 i32)
  (local $5 i32)
  (local $6 i32)
  (block $block
   (br_if $block
    (i32.eqz
     (local.get $2)
    )
   )
   (local.set $3
    (i32.and
     (local.get $2)
     (i32.const 3)
    )
   )
   (local.set $4
    (i32.const 0)
   )
   (block $block1
    (br_if $block1
     (i32.lt_u
      (local.get $2)
      (i32.const 4)
     )
    )
    (local.set $5
     (i32.and
      (local.get $2)
      (i32.const -4)
     )
    )
    (local.set $4
     (i32.const 0)
    )
    (loop $label
     (i32.store8
      (local.tee $2
       (i32.add
        (local.get $0)
        (local.get $4)
       )
      )
      (i32.load8_u
       (local.tee $6
        (i32.add
         (local.get $1)
         (local.get $4)
        )
       )
      )
     )
     (i32.store8
      (i32.add
       (local.get $2)
       (i32.const 1)
      )
      (i32.load8_u
       (i32.add
        (local.get $6)
        (i32.const 1)
       )
      )
     )
     (i32.store8
      (i32.add
       (local.get $2)
       (i32.const 2)
      )
      (i32.load8_u
       (i32.add
        (local.get $6)
        (i32.const 2)
       )
      )
     )
     (i32.store8
      (i32.add
       (local.get $2)
       (i32.const 3)
      )
      (i32.load8_u
       (i32.add
        (local.get $6)
        (i32.const 3)
       )
      )
     )
     (br_if $label
      (i32.ne
       (local.get $5)
       (local.tee $4
        (i32.add
         (local.get $4)
         (i32.const 4)
        )
       )
      )
     )
    )
   )
   (br_if $block
    (i32.eqz
     (local.get $3)
    )
   )
   (local.set $2
    (i32.add
     (local.get $1)
     (local.get $4)
    )
   )
   (local.set $4
    (i32.add
     (local.get $0)
     (local.get $4)
    )
   )
   (loop $label1
    (i32.store8
     (local.get $4)
     (i32.load8_u
      (local.get $2)
     )
    )
    (local.set $2
     (i32.add
      (local.get $2)
      (i32.const 1)
     )
    )
    (local.set $4
     (i32.add
      (local.get $4)
      (i32.const 1)
     )
    )
    (br_if $label1
     (local.tee $3
      (i32.add
       (local.get $3)
       (i32.const -1)
      )
     )
    )
   )
  )
  (local.get $0)
 )
(func (export "f32_add") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.add i32.reinterpret_f32)(func (export "f32_sub") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.sub i32.reinterpret_f32)(func (export "f32_mul") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.mul i32.reinterpret_f32)(func (export "f32_div") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.div i32.reinterpret_f32)(func (export "f32_min") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.min i32.reinterpret_f32)(func (export "f32_max") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.max i32.reinterpret_f32)(func (export "f32_copysign") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.copysign i32.reinterpret_f32)(func (export "f32_eq") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.eq)(func (export "f32_ne") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.ne)(func (export "f32_lt") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.lt)(func (export "f32_le") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.le)(func (export "f32_gt") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.gt)(func (export "f32_ge") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 f32.ge)(func (export "f32_abs") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.abs i32.reinterpret_f32)(func (export "f32_neg") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.neg i32.reinterpret_f32)(func (export "f32_sqrt") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.sqrt i32.reinterpret_f32)(func (export "f32_ceil") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.ceil i32.reinterpret_f32)(func (export "f32_floor") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.floor i32.reinterpret_f32)(func (export "f32_trunc") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.trunc i32.reinterpret_f32)(func (export "f32_nearest") (param i32) (result i32) local.get 0 f32.reinterpret_i32 f32.nearest i32.reinterpret_f32)(func (export "i32_trunc_f32_s") (param i32) (result i32) local.get 0 f32.reinterpret_i32 i32.trunc_f32_s)(func (export "i32_trunc_sat_f32_s") (param i32) (result i32) local.get 0 f32.reinterpret_i32 i32.trunc_sat_f32_s)(func (export "f32_convert_i32_s") (param i32) (result i32) local.get 0 f32.convert_i32_s i32.reinterpret_f32)(func (export "i32_trunc_f32_u") (param i32) (result i32) local.get 0 f32.reinterpret_i32 i32.trunc_f32_u)(func (export "i32_trunc_sat_f32_u") (param i32) (result i32) local.get 0 f32.reinterpret_i32 i32.trunc_sat_f32_u)(func (export "f32_convert_i32_u") (param i32) (result i32) local.get 0 f32.convert_i32_u i32.reinterpret_f32)(func (export "i64_trunc_f32_s") (param i32) (result i64) local.get 0 f32.reinterpret_i32 i64.trunc_f32_s)(func (export "i64_trunc_sat_f32_s") (param i32) (result i64) local.get 0 f32.reinterpret_i32 i64.trunc_sat_f32_s)(func (export "f32_convert_i64_s") (param i64) (result i32) local.get 0 f32.convert_i64_s i32.reinterpret_f32)(func (export "i64_trunc_f32_u") (param i32) (result i64) local.get 0 f32.reinterpret_i32 i64.trunc_f32_u)(func (export "i64_trunc_sat_f32_u") (param i32) (result i64) local.get 0 f32.reinterpret_i32 i64.trunc_sat_f32_u)(func (export "f32_convert_i64_u") (param i64) (result i32) local.get 0 f32.convert_i64_u i32.reinterpret_f32)(func (export "f64_add") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.add i64.reinterpret_f64)(func (export "f64_sub") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.sub i64.reinterpret_f64)(func (export "f64_mul") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.mul i64.reinterpret_f64)(func (export "f64_div") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.div i64.reinterpret_f64)(func (export "f64_min") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.min i64.reinterpret_f64)(func (export "f64_max") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.max i64.reinterpret_f64)(func (export "f64_copysign") (param i64 i64) (result i64) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.copysign i64.reinterpret_f64)(func (export "f64_eq") (param i64 i64) (result i32) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.eq)(func (export "f64_ne") (param i64 i64) (result i32) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.ne)(func (export "f64_lt") (param i64 i64) (result i32) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.lt)(func (export "f64_le") (param i64 i64) (result i32) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.le)(func (export "f64_gt") (param i64 i64) (result i32) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.gt)(func (export "f64_ge") (param i64 i64) (result i32) local.get 0 f64.reinterpret_i64 local.get 1 f64.reinterpret_i64 f64.ge)(func (export "f64_abs") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.abs i64.reinterpret_f64)(func (export "f64_neg") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.neg i64.reinterpret_f64)(func (export "f64_sqrt") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.sqrt i64.reinterpret_f64)(func (export "f64_ceil") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.ceil i64.reinterpret_f64)(func (export "f64_floor") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.floor i64.reinterpret_f64)(func (export "f64_trunc") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.trunc i64.reinterpret_f64)(func (export "f64_nearest") (param i64) (result i64) local.get 0 f64.reinterpret_i64 f64.nearest i64.reinterpret_f64)(func (export "i32_trunc_f64_s") (param i64) (result i32) local.get 0 f64.reinterpret_i64 i32.trunc_f64_s)(func (export "i32_trunc_sat_f64_s") (param i64) (result i32) local.get 0 f64.reinterpret_i64 i32.trunc_sat_f64_s)(func (export "f64_convert_i32_s") (param i32) (result i64) local.get 0 f64.convert_i32_s i64.reinterpret_f64)(func (export "i32_trunc_f64_u") (param i64) (result i32) local.get 0 f64.reinterpret_i64 i32.trunc_f64_u)(func (export "i32_trunc_sat_f64_u") (param i64) (result i32) local.get 0 f64.reinterpret_i64 i32.trunc_sat_f64_u)(func (export "f64_convert_i32_u") (param i32) (result i64) local.get 0 f64.convert_i32_u i64.reinterpret_f64)(func (export "i64_trunc_f64_s") (param i64) (result i64) local.get 0 f64.reinterpret_i64 i64.trunc_f64_s)(func (export "i64_trunc_sat_f64_s") (param i64) (result i64) local.get 0 f64.reinterpret_i64 i64.trunc_sat_f64_s)(func (export "f64_convert_i64_s") (param i64) (result i64) local.get 0 f64.convert_i64_s i64.reinterpret_f64)(func (export "i64_trunc_f64_u") (param i64) (result i64) local.get 0 f64.reinterpret_i64 i64.trunc_f64_u)(func (export "i64_trunc_sat_f64_u") (param i64) (result i64) local.get 0 f64.reinterpret_i64 i64.trunc_sat_f64_u)(func (export "f64_convert_i64_u") (param i64) (result i64) local.get 0 f64.convert_i64_u i64.reinterpret_f64)(func (export "i32_div_s") (param i32 i32) (result i32) local.get 0 local.get 1 i32.div_s)(func (export "i32_div_u") (param i32 i32) (result i32) local.get 0 local.get 1 i32.div_u)(func (export "i32_rem_s") (param i32 i32) (result i32) local.get 0 local.get 1 i32.rem_s)(func (export "i32_rem_u") (param i32 i32) (result i32) local.get 0 local.get 1 i32.rem_u)(func (export "i32_mul") (param i32 i32) (result i32) local.get 0 local.get 1 i32.mul)(func (export "i32_add") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add)(func (export "i32_sub") (param i32 i32) (result i32) local.get 0 local.get 1 i32.sub)(func (export "i32_shr_s") (param i32 i32) (result i32) local.get 0 local.get 1 i32.shr_s)(func (export "i32_shr_u") (param i32 i32) (result i32) local.get 0 local.get 1 i32.shr_u)(func (export "i32_shl") (param i32 i32) (result i32) local.get 0 local.get 1 i32.shl)(func (export "i32_rotl") (param i32 i32) (result i32) local.get 0 local.get 1 i32.rotl)(func (export "i32_rotr") (param i32 i32) (result i32) local.get 0 local.get 1 i32.rotr)(func (export "i64_div_s") (param i64 i64) (result i64) local.get 0 local.get 1 i64.div_s)(func (export "i64_div_u") (param i64 i64) (result i64) local.get 0 local.get 1 i64.div_u)(func (export "i64_rem_s") (param i64 i64) (result i64) local.get 0 local.get 1 i64.rem_s)(func (export "i64_rem_u") (param i64 i64) (result i64) local.get 0 local.get 1 i64.rem_u)(func (export "i64_mul") (param i64 i64) (result i64) local.get 0 local.get 1 i64.mul)(func (export "i64_add") (param i64 i64) (result i64) local.get 0 local.get 1 i64.add)(func (export "i64_sub") (param i64 i64) (result i64) local.get 0 local.get 1 i64.sub)(func (export "i64_shr_s") (param i64 i64) (result i64) local.get 0 local.get 1 i64.shr_s)(func (export "i64_shr_u") (param i64 i64) (result i64) local.get 0 local.get 1 i64.shr_u)(func (export "i64_shl") (param i64 i64) (result i64) local.get 0 local.get 1 i64.shl)(func (export "i64_rotl") (param i64 i64) (result i64) local.get 0 local.get 1 i64.rotl)(func (export "i64_rotr") (param i64 i64) (result i64) local.get 0 local.get 1 i64.rotr)(func (export "demote") (param i64) (result i32) local.get 0 f64.reinterpret_i64 f32.demote_f64 i32.reinterpret_f32)(func (export "promote") (param i32) (result i64) local.get 0 f32.reinterpret_i32 f64.promote_f32 i64.reinterpret_f64)(func (export "load") (param i32) (result i32) local.get 0 i32.load)(func (export "offset") (param i32) (result i32) local.get 0 i32.load offset=4294967295)(func (export "copy") (param i32 i32 i32) (result i32) local.get 0 local.get 1 local.get 2 memory.copy i32.const 7)(func (export "fill") (param i32 i32 i32) (result i32) local.get 0 local.get 1 local.get 2 memory.fill i32.const 7)(func (export "store") (param i32 i32) (result i32) local.get 0 local.get 1 i32.store i32.const 1)(func (export "after_store") (param ) (result i32) i32.const 65532 i32.load)(func (export "atomic_fill") (param ) (result i32) i32.const 65532 i32.const 255 i32.const 8 memory.fill i32.const 0)(func (export "after_fill") (param ) (result i32) i32.const 65532 i32.load)(func (export "atomic_copy") (param ) (result i32) i32.const 65532 i32.const 0 i32.const 8 memory.copy i32.const 0)(func (export "after_copy") (param ) (result i32) i32.const 65532 i32.load)(func (export "grow") (param i32) (result i32) local.get 0 memory.grow)(func (export "size") (param ) (result i32) memory.size)(func (export "grown_zero") (param ) (result i32) i32.const 65536 i32.load)(func (export "grown_end") (param i32) (result i32) local.get 0 i32.load)(func (export "partial_copy") (param ) (result i32) i32.const 65534 i32.const 131071 i32.const 0xaa i32.store8 i32.const 65534 i32.const 131071 i32.const 2 call $memcpy drop i32.load8_u)(func (export "partial_byte") (param ) (result i32) i32.const 65534 i32.load8_u)(func (export "empty_c_copy") (param ) (result i32) i32.const -1 i32.const -1 i32.const 0 call $memcpy)(func (export "host_sin") (param i32) (result i32) local.get 0 f32.reinterpret_i32 call $sinf i32.reinterpret_f32)(func (export "host_mod") (param i32 i32) (result i32) local.get 0 f32.reinterpret_i32 local.get 1 f32.reinterpret_i32 call $fmodf i32.reinterpret_f32))