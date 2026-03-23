;; GCD computation — exercises loops, branches, and local variables.
;; More complex control flow than hello.wat.
(module
  (func $gcd (export "gcd") (param i32 i32) (result i32)
    (local i32)
    block
      block
        local.get 0
        br_if 0
        local.get 1
        local.set 2
        br 1
      end
      loop
        local.get 1
        local.get 0
        local.tee 2
        i32.rem_u
        local.set 0
        local.get 2
        local.set 1
        local.get 0
        br_if 0
      end
    end
    local.get 2
  )

  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add
  )

  (func (export "_start")
    ;; gcd(6, 27) == 3
    (if (i32.ne (call $gcd (i32.const 6) (i32.const 27)) (i32.const 3))
      (then unreachable)
    )
  )
)
