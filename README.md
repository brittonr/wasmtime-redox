# wasmtime-redox

Wasmtime WebAssembly runtime cross-compiled for [Redox OS](https://www.redox-os.org/).

Uses Wasmtime's **Pulley interpreter** backend and **custom platform API** to
run on Redox without depending on `ucontext_t`, `sigaltstack`, or signal-based
trap handling.

## Quick start

```bash
# Run the demo (compiles hello.wat to Pulley bytecode, runs it)
nix run github:brittonr/wasmtime-redox

# Build precompiled Pulley bytecode modules
nix build github:brittonr/wasmtime-redox

# Enter a devshell with wasmtime and wabt tools
nix develop github:brittonr/wasmtime-redox
wasmtime compile --target pulley64 examples/hello.wat -o hello.cwasm
```

## Architecture

Redox is `cfg(unix)` but lacks the signal context types that Wasmtime's unix
sys module needs. Two small patches route Redox to the `custom` sys module:

1. **`sys/mod.rs`** — `cfg(target_os = "redox")` before the `cfg(unix)` catch-all
2. **`build.rs`** — exclude Redox from `supported_os` so `has_native_signals = false`

The `custom` module requires external C symbols for virtual memory and TLS.
These are implemented in `platform/wasmtime-platform-redox.c` using Redox's
standard POSIX mmap/mprotect/munmap via relibc.

With `has_native_signals = false`, the Pulley interpreter handles all traps
(bounds checks, division by zero, unreachable) in software. No signal handler
registration is needed.

## What works

- `wasmtime compile` — compile `.wat`/`.wasm` to Pulley bytecode (`.cwasm`)
- Cranelift code generation targeting Pulley
- Runtime module loading and execution via Pulley interpreter
- WASI preview 1 (stdout, args, env, clocks, random — see below)
- 15 MB statically linked binary, 33s build

## WASI support

The runners include a minimal WASI preview 1 shim (`runner/wasi.rs`) that
implements the most common host imports without pulling in `wasmtime-wasi`,
`cap-std`, or `tokio`. Uses raw libc calls through relibc.

| WASI function | Status | Implementation |
|---------------|--------|----------------|
| `fd_write` (stdout/stderr) | ✅ | Direct write via `std::io::stdout/stderr` |
| `args_get` / `args_sizes_get` | ✅ | Forwarded from host process args |
| `environ_get` / `environ_sizes_get` | ✅ | Configurable env vars |
| `clock_time_get` | ✅ | Monotonic + realtime via `std::time` |
| `proc_exit` | ✅ | Typed trap caught by runner |
| `random_get` | ✅ | `/dev/urandom` |
| `sched_yield` | ✅ | `std::thread::yield_now` |
| `fd_fdstat_get` | ✅ | Returns CHARACTER_DEVICE for stdio fds |
| `fd_prestat_get` | ✅ | Returns BADF (no preopened dirs) |
| `fd_close` (stdio) | ✅ | Returns success for fds 0–2 |
| `fd_read`, filesystem, sockets | ⬚ | Stubs return ENOSYS |

Modules that import stubbed functions can still instantiate — they only fail
if the code path is actually reached at runtime.

## Tested on Redox

All execution paths are tested in a Redox VM via `wasm-runner` and
`wasm-async-runner` (minimal binaries injected into the wasmtime build):

| Capability | Status | Notes |
|------------|--------|-------|
| Pulley compilation | ✅ | `wasmtime compile --target pulley64` |
| Native x86_64 compilation | ✅ | `wasmtime compile` (default target) |
| Native execution (libcall traps) | ✅ | `wasm-runner` loads .wat, Engine::default() compiles to x86_64 |
| Linear memory (mmap platform) | ✅ | store/load across page boundaries |
| Control flow (loops, branches) | ✅ | GCD computation with `--invoke` |
| Async/fiber function calls | ✅ | `wasm-async-runner` with typed args, fiber context switching |
| Precompiled cwasm roundtrip | ✅ | `--roundtrip` flag: compile → serialize → deserialize → run |
| WASI fd_write | ✅ | `hello-wasi.wat` — iovec write to stdout, nwritten verified |
| WASI args | ✅ | `args-wasi.wat` — args_sizes_get returns errno 0 |
| WASI clock | ✅ | `clock-wasi.wat` — monotonic clock t2 ≥ t1 |
| Async void calls | ❌ | `()→()` fiber calls crash (SIGSEGV); typed returns work |

On x86_64 Redox, `Engine::default()` compiles to **native x86_64** via
Cranelift (not Pulley) because `has_host_compiler_backend = true`. Traps use
the libcall path (`has_native_signals = false`) — no `ucontext_t` or signal
handlers needed.

## Not built

- **`wasmtime run` CLI** — the upstream `run` feature bundles tokio + WASI as
  compile-time deps. The runners provide WASI via a lightweight shim instead.

## Usage from the Redox flake

The cross-compiled Redox binary requires the full Redox toolchain (relibc,
cross-compiler, sysroot). Consume it from your Redox flake:

```nix
# flake.nix inputs
wasmtime-redox = {
  url = "github:brittonr/wasmtime-redox";
  flake = true;
};

# In packages.nix
wasmtime-redox = inputs.wasmtime-redox.lib.mkWasmtime standaloneCommon;
```

## Files

| Path | Description |
|------|-------------|
| `patches/0001-redox-custom-platform.patch` | Routes Redox to custom sys module |
| `platform/wasmtime-platform-redox.c` | mmap/mprotect/tls symbols for Redox |
| `runner/wasi/mod.rs` | WASI preview 1 shim (fd_write, args, env, clock, random) |
| `runner/main.rs` | Sync runner with WASI + Linker |
| `runner/async-main.rs` | Async/fiber runner with WASI + Linker |
| `nix/package.nix` | Nix cross-compilation package |
| `nix/precompile.nix` | Host-side .wat → .cwasm precompilation |
| `examples/hello.wat` | Pure compute module (i32.add arithmetic) |
| `examples/hello-wasi.wat` | WASI test: fd_write to stdout |
| `examples/args-wasi.wat` | WASI test: read and print args |
| `examples/clock-wasi.wat` | WASI test: monotonic clock ordering |

## Wasmtime version

Pinned to **v43.0.0** via flake input. The vendor hash and patches are
version-specific — bumping requires updating both.

## License

Apache-2.0
