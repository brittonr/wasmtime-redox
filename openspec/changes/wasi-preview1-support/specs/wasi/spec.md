# WASI Preview 1 Shim Specification

## Purpose

Defines the WASI `wasi_snapshot_preview1` functions implemented by the
Redox shim, their behavior, and how they map to host syscalls via relibc.

## Requirements

### Requirement: fd_write on stdout and stderr

The shim MUST implement `fd_write(fd, iovs, iovs_len, nwritten)` for
file descriptors 1 (stdout) and 2 (stderr). It MUST write the gathered
iovec buffers to the host file descriptor and store the total bytes
written at `nwritten`. It MUST return `errno::success` on success.

It MUST return `errno::badf` for any file descriptor other than 1 or 2.

#### Scenario: Hello world print

- GIVEN a wasm module that calls `fd_write(1, iovs, 1, nwritten)`
- WHEN the iov points to the bytes "hello\n"
- THEN "hello\n" appears on host stdout and `nwritten` contains 6

#### Scenario: Invalid file descriptor

- GIVEN a wasm module that calls `fd_write(42, iovs, 1, nwritten)`
- WHEN the shim handles the call
- THEN it returns `errno::badf` without writing anything

### Requirement: args_get and args_sizes_get

The shim MUST implement `args_sizes_get(argc, argv_buf_size)` and
`args_get(argv, argv_buf)`. The argument list MUST be configurable by
the host before instantiation. Each argument MUST be null-terminated
in `argv_buf`.

#### Scenario: Pass two arguments

- GIVEN the host configures args as `["program", "file.txt"]`
- WHEN the module calls `args_sizes_get`
- THEN `argc` is 2 and `argv_buf_size` is the sum of lengths + null terminators

### Requirement: environ_get and environ_sizes_get

The shim MUST implement `environ_sizes_get(environc, environ_buf_size)` and
`environ_get(environ, environ_buf)`. Environment variables MUST be
configurable by the host. Each entry MUST be in `KEY=VALUE\0` format.

#### Scenario: Empty environment

- GIVEN the host configures zero environment variables
- WHEN the module calls `environ_sizes_get`
- THEN `environc` is 0 and `environ_buf_size` is 0

### Requirement: clock_time_get

The shim MUST implement `clock_time_get(id, precision, time)`. It MUST
support `clockid::monotonic` and `clockid::realtime`. The returned time
MUST be in nanoseconds.

It SHOULD use `clock_gettime(2)` via relibc.

It MUST return `errno::inval` for unsupported clock IDs.

#### Scenario: Monotonic clock is monotonic

- GIVEN a module that calls `clock_time_get(monotonic)` twice
- WHEN both calls succeed
- THEN the second value is greater than or equal to the first

### Requirement: proc_exit

The shim MUST implement `proc_exit(rval)`. It MUST terminate the wasm
execution and propagate `rval` as the process exit code.

It SHOULD use wasmtime's trap mechanism rather than calling `std::process::exit`
directly, so the host can clean up.

#### Scenario: Exit with code 0

- GIVEN a module that calls `proc_exit(0)`
- WHEN the host runner handles the trap
- THEN the process exits with code 0

#### Scenario: Exit with non-zero code

- GIVEN a module that calls `proc_exit(42)`
- WHEN the host runner handles the trap
- THEN the process exits with code 42

### Requirement: random_get

The shim MUST implement `random_get(buf, buf_len)`. It MUST fill the
buffer with cryptographically suitable random bytes.

It SHOULD use `getrandom(2)` or read from `/dev/urandom` via relibc.

#### Scenario: Fill 32 bytes

- GIVEN a module that calls `random_get(buf, 32)`
- WHEN the call succeeds
- THEN 32 bytes are written to buf and the return is `errno::success`

### Requirement: Stub functions return ENOSYS

All unimplemented `wasi_snapshot_preview1` functions MUST be present as
imports (so modules that declare them can instantiate) and MUST return
`errno::nosys` when called.

The full set includes: `fd_read`, `fd_seek`, `fd_tell`, `fd_close`,
`fd_fdstat_get`, `fd_fdstat_set_flags`, `fd_prestat_get`,
`fd_prestat_dir_name`, `path_open`, `path_create_directory`,
`path_remove_directory`, `path_rename`, `path_unlink_file`,
`path_readlink`, `path_filestat_get`, `path_filestat_set_times`,
`fd_filestat_get`, `fd_filestat_set_times`, `fd_readdir`,
`fd_advise`, `fd_allocate`, `fd_datasync`, `fd_sync`,
`fd_pread`, `fd_pwrite`, `fd_renumber`, `poll_oneoff`,
`sched_yield`, `sock_recv`, `sock_send`, `sock_shutdown`,
`sock_accept`.

#### Scenario: Module imports fd_read but never calls it

- GIVEN a module that declares `fd_read` as an import
- WHEN the module is instantiated
- THEN instantiation succeeds (the stub is provided)

#### Scenario: Module calls an unimplemented function

- GIVEN a module that calls `fd_read`
- WHEN the shim handles the call
- THEN it returns `errno::nosys`
