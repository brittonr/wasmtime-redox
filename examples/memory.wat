;; Memory operations — exercises linear memory load/store.
;; Tests that the custom mmap platform works for wasm memory.
(module
  (memory (export "memory") 1)

  (func (export "store_and_load") (param i32 i32) (result i32)
    ;; Store param1 at address param0, then load it back
    local.get 0
    local.get 1
    i32.store
    local.get 0
    i32.load
  )

  (func (export "_start")
    ;; Store 42 at offset 0, verify it loads back as 42
    (if (i32.ne
      (call 1 (i32.const 0) (i32.const 42))
      (i32.const 42))
      (then unreachable)
    )
    ;; Store at a higher offset to exercise page mapping
    (if (i32.ne
      (call 1 (i32.const 65536) (i32.const 99))
      (i32.const 99))
      (then unreachable)
    )
  )
)
