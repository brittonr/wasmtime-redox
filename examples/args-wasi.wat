;; WASI args test — reads argument count and prints each argument.
;; Tests args_sizes_get, args_get, and fd_write together.
(module
  (import "wasi_snapshot_preview1" "args_sizes_get"
    (func $args_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "args_get"
    (func $args_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))

  (memory (export "memory") 1)

  ;; Memory layout:
  ;;   0..4    argc (result from args_sizes_get)
  ;;   4..8    argv_buf_size (result from args_sizes_get)
  ;;   8..12   nwritten (result from fd_write)
  ;;   16..80  iovec scratch (16 bytes)
  ;;   256+    argv array (pointers)
  ;;   1024+   argv_buf (string data)

  (func (export "_start")
    (local $argc i32)
    (local $i i32)
    (local $ptr i32)

    ;; Get arg sizes
    (call $args_sizes_get (i32.const 0) (i32.const 4))
    drop
    (local.set $argc (i32.load (i32.const 0)))

    ;; Get args (argv at 256, argv_buf at 1024)
    (call $args_get (i32.const 256) (i32.const 1024))
    drop

    ;; Print each arg followed by newline
    (local.set $i (i32.const 0))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $argc)))

        ;; argv[i] is a pointer to the arg string at offset 256 + i*4
        (local.set $ptr (i32.load (i32.add (i32.const 256) (i32.mul (local.get $i) (i32.const 4)))))

        ;; Find string length by scanning for null byte
        (call $print_cstr (local.get $ptr))

        ;; Write newline
        (i32.store8 (i32.const 100) (i32.const 10))  ;; '\n' at scratch offset 100
        (i32.store (i32.const 16) (i32.const 100))    ;; iov.buf
        (i32.store (i32.const 20) (i32.const 1))      ;; iov.buf_len
        (call $fd_write (i32.const 1) (i32.const 16) (i32.const 1) (i32.const 8))
        drop

        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $next
      )
    )
  )

  ;; Print a null-terminated string to stdout
  (func $print_cstr (param $ptr i32)
    (local $len i32)
    (local $p i32)
    (local.set $p (local.get $ptr))
    ;; Scan for null
    (block $end
      (loop $scan
        (br_if $end (i32.eqz (i32.load8_u (local.get $p))))
        (local.set $p (i32.add (local.get $p) (i32.const 1)))
        br $scan
      )
    )
    (local.set $len (i32.sub (local.get $p) (local.get $ptr)))
    ;; Build iovec at 16
    (i32.store (i32.const 16) (local.get $ptr))
    (i32.store (i32.const 20) (local.get $len))
    ;; fd_write(stdout, iovs=16, 1, nwritten=8)
    (call $fd_write (i32.const 1) (i32.const 16) (i32.const 1) (i32.const 8))
    drop
  )
)
