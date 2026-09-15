(module (memory 1)
 (func $memcpy (param $0 i32) (param $1 i32) (param $2 i32) (result i32)
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
)
