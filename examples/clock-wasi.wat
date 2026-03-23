;; WASI clock test — reads monotonic clock twice and verifies t2 >= t1.
;; Tests clock_time_get and fd_write for result reporting.
(module
  (import "wasi_snapshot_preview1" "clock_time_get"
    (func $clock_time_get (param i32 i64 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  (memory (export "memory") 1)

  ;; Memory layout:
  ;;   0..8    t1 (first timestamp, u64)
  ;;   8..16   t2 (second timestamp, u64)
  ;;   16..24  iovec
  ;;   24..28  nwritten
  ;;   32+     string data

  ;; Result strings
  (data (i32.const 32) "PASS: monotonic clock ok\n")  ;; 25 bytes
  (data (i32.const 64) "FAIL: t2 < t1\n")             ;; 14 bytes

  (func (export "_start")
    ;; clock_time_get(MONOTONIC=1, precision=0, time_ptr)
    (call $clock_time_get (i32.const 1) (i64.const 0) (i32.const 0))
    drop

    ;; Read again
    (call $clock_time_get (i32.const 1) (i64.const 0) (i32.const 8))
    drop

    ;; Compare: t2 (offset 8) >= t1 (offset 0)
    (if (i64.lt_u (i64.load (i32.const 8)) (i64.load (i32.const 0)))
      (then
        ;; FAIL
        (i32.store (i32.const 16) (i32.const 64))
        (i32.store (i32.const 20) (i32.const 14))
        (call $fd_write (i32.const 1) (i32.const 16) (i32.const 1) (i32.const 24))
        drop
        unreachable
      )
      (else
        ;; PASS
        (i32.store (i32.const 16) (i32.const 32))
        (i32.store (i32.const 20) (i32.const 25))
        (call $fd_write (i32.const 1) (i32.const 16) (i32.const 1) (i32.const 24))
        drop
      )
    )
  )
)
