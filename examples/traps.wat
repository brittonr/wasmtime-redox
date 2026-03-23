;; Trap handling — tests that software trap handling works.
;; On Redox (has_native_signals = false), traps go through libcall path.
(module
  (func (export "div") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.div_u
  )

  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add
  )

  ;; _start: just verify normal operation works.
  ;; Trap testing (div by zero) is done from the runner, which catches the error.
  (func (export "_start")
    ;; 10 / 2 == 5
    (if (i32.ne (call 0 (i32.const 10) (i32.const 2)) (i32.const 5))
      (then unreachable)
    )
  )
)
