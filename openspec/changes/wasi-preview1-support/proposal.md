# WASI Preview 1 Support for Redox

## Why

The current runners (`wasm-runner`, `wasm-async-runner`) only execute pure
compute modules with no host imports. Any wasm module that calls `fd_write`
to print to stdout, reads environment variables, or accesses the clock fails
at instantiation because the imports aren't provided.

Most real-world wasm binaries — anything compiled from C/Rust with a standard
library — expect at least `wasi_snapshot_preview1`. Without it, wasmtime on
Redox is limited to hand-crafted `.wat` files.

## What Changes

Add a minimal WASI preview 1 implementation that satisfies the most common
host imports using Redox's POSIX-compatible syscalls through relibc. Not the
full `wasmtime-wasi` crate (which pulls in cap-std, tokio, and the component
model), but a hand-rolled shim linked directly into the runner binaries.

- **WASI shim library** — a new Rust module implementing `wasi_snapshot_preview1`
  functions as wasmtime `Func` imports. Uses raw libc calls, not cap-std.
- **Updated runners** — `wasm-runner` and `wasm-async-runner` link the shim
  and provide WASI imports during instantiation.
- **Test modules** — `.wat` and compiled-from-Rust `.wasm` files that exercise
  the supported WASI calls.
- **Nix build changes** — the shim compiles as part of the runner binaries,
  no new crate dependencies.

## Capabilities

### New Capabilities
- `wasi-stdout`: wasm modules can write to stdout/stderr via `fd_write`
- `wasi-args`: wasm modules can read command-line arguments via `args_get`/`args_sizes_get`
- `wasi-env`: wasm modules can read environment variables via `environ_get`/`environ_sizes_get`
- `wasi-clock`: wasm modules can read monotonic/realtime clocks via `clock_time_get`
- `wasi-exit`: wasm modules can exit with a code via `proc_exit`
- `wasi-random`: wasm modules can get random bytes via `random_get`

### Deferred Capabilities
- `wasi-fs`: filesystem access (`path_open`, `fd_read` on files, directory ops)
- `wasi-net`: socket operations (not in preview 1 anyway)
- `wasi-poll`: `poll_oneoff` for timer/fd readiness

## Impact

- **Files**: new `runner/wasi.rs` module, updated `runner/main.rs` and `runner/async-main.rs`, new test `.wat` files in `examples/`
- **APIs**: no public API change — the runners gain WASI support transparently
- **Dependencies**: zero new crate dependencies. Uses `libc` types already available through relibc.
- **Testing**: each WASI function gets a `.wat` test module run on Redox via the existing VM test flow
- **Binary size**: expect ~5–10 KB growth from the shim functions

## Scope Boundary

In scope: the six capabilities listed above (`fd_write` on stdout/stderr,
args, env, clocks, proc_exit, random_get). These cover "hello world" through
basic CLI tools.

Out of scope: filesystem access, `poll_oneoff`, `fd_seek`/`fd_tell`,
`fd_read` on stdin, directory operations, symlinks. These can follow as a
separate change once the foundation works.
