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
- 15 MB statically linked binary, 33s build

## What doesn't (yet)

- `wasmtime run` — requires WASI + tokio (not ported to Redox)
- Native JIT execution — requires `ucontext_t` / kernel signal context support
- Async/fiber support — requires mmap'd stack switching

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
| `nix/package.nix` | Nix cross-compilation package |
| `nix/precompile.nix` | Host-side .wat → .cwasm precompilation |
| `examples/hello.wat` | Minimal test module (i32.add arithmetic) |

## Wasmtime version

Pinned to **v43.0.0** via flake input. The vendor hash and patches are
version-specific — bumping requires updating both.

## License

Apache-2.0
