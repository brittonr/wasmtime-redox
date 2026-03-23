;; WASI hello world — writes "hello wasi\n" to stdout via fd_write.
;; Tests the fd_write shim and iovec memory layout.
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  (memory (export "memory") 1)

  ;; String data at offset 16
  (data (i32.const 16) "hello wasi\n")

  (func (export "_start")
    ;; Build iovec at offset 0: { buf_ptr: 16, buf_len: 11 }
    (i32.store (i32.const 0) (i32.const 16))   ;; iov.buf = 16
    (i32.store (i32.const 4) (i32.const 11))   ;; iov.buf_len = 11

    ;; fd_write(stdout=1, iovs=0, iovs_len=1, nwritten=8)
    (call $fd_write
      (i32.const 1)   ;; fd: stdout
      (i32.const 0)   ;; iovs pointer
      (i32.const 1)   ;; iovs count
      (i32.const 8))  ;; nwritten pointer
    drop

    ;; Verify nwritten == 11
    (if (i32.ne (i32.load (i32.const 8)) (i32.const 11))
      (then unreachable))
  )
)
