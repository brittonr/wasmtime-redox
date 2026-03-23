# WASI Preview 1 Design

## Context

Wasmtime upstream has a full `wasmtime-wasi` crate that implements WASI
preview 1 and preview 2. It depends on `cap-std` (capability-based filesystem),
`tokio` (async runtime), and the component model. Getting that whole stack to
cross-compile for Redox is a large effort with unclear benefit — we don't
need sandboxed filesystem capabilities or async I/O for the initial use case.

The runners (`wasm-runner`, `wasm-async-runner`) currently pass an empty
import list `&[]` to `Instance::new`. Any module with WASI imports fails.

## Goals / Non-Goals

**Goals:**
- Run wasm binaries compiled from Rust/C that use basic stdio, args, env, clocks
- Zero new crate dependencies
- Stay within the existing runner binary injection pattern
- Each WASI function independently testable with a `.wat` file

**Non-Goals:**
- Filesystem sandboxing or capability-based access
- Preview 2 / component model support
- Replacing upstream `wasmtime-wasi` — this is a minimal Redox-specific shim
- Supporting `fd_read` on stdin (requires terminal mode handling on Redox)

## Decisions

### 1. Hand-rolled shim, not wasmtime-wasi

**Choice:** Implement WASI functions directly using wasmtime's `Linker` API
and raw libc calls.

**Rationale:** `wasmtime-wasi` pulls in cap-std, tokio, and dozens of
transitive deps. The six functions we need (fd_write, args, env, clock,
proc_exit, random) are ~200 lines of straightforward libc calls. No reason
to drag in a complex dependency tree.

**Alternative:** Fork/patch wasmtime-wasi to remove tokio/cap-std deps.
Rejected — maintaining a fork of a fast-moving crate is worse than owning
a small shim.

**Implementation:** A single `runner/wasi.rs` file that exports an
`add_wasi_to_linker(linker, config)` function. Each WASI function is
registered via `linker.func_wrap("wasi_snapshot_preview1", "fd_write", ...)`.

### 2. WasiCtx in Store data

**Choice:** Store WASI state (args, env vars) in a `WasiCtx` struct carried
as the `Store<T>` data parameter.

**Rationale:** Wasmtime's `Caller<'_, T>` gives typed access to store data.
This is the standard pattern — no globals, no side channels.

**Implementation:**
```rust
struct WasiCtx {
    args: Vec<String>,
    env: Vec<(String, String)>,
}

let mut store = Store::new(&engine, WasiCtx {
    args: std::env::args().collect(),
    env: vec![],
});
```

### 3. proc_exit via trap, not process::exit

**Choice:** Implement `proc_exit` by returning a typed trap that the runner
catches, rather than calling `std::process::exit`.

**Rationale:** Calling `exit()` from inside a wasm host function skips
destructors, drops, and any cleanup the runner wants to do. A trap unwinds
cleanly through wasmtime's call stack.

**Implementation:** Define a `WasiExit(i32)` error type. The runner's
call site catches it and maps it to a process exit code.

### 4. All stubs present at link time

**Choice:** Register every `wasi_snapshot_preview1` function in the linker,
with unimplemented ones returning `errno::nosys`.

**Rationale:** Wasm modules declare their imports in the binary. If a module
imports `fd_prestat_get` (which many Rust-compiled binaries do), instantiation
fails unless the import is provided — even if the code path never executes.
Providing stubs lets these modules load.

**Implementation:** A `register_stubs(linker)` function that covers the
~30 remaining WASI functions with a generic `|| errno::nosys` closure.

### 5. Keep runner/wasi.rs in this repo, not a separate crate

**Choice:** The shim lives as a module in the runner directory, compiled
directly into the runner binaries.

**Rationale:** The shim is Redox-specific glue with no value as a standalone
crate. It's injected into the wasmtime-cli crate alongside the runners at
build time. Keeping it in one place avoids cross-crate dependency management.

**Alternative:** A separate `wasmtime-wasi-redox` crate. Rejected — adds
Cargo workspace complexity for ~300 lines of code.

## Risks / Trade-offs

**[relibc clock_gettime accuracy]** — Redox's `clock_gettime` might have
lower resolution than Linux. Mitigation: the spec only requires nanosecond
units, not nanosecond precision. Test by checking monotonicity, not exact
timing.

**[getrandom availability]** — If `getrandom(2)` isn't available in relibc,
fall back to `/dev/urandom`. Both paths are checked at build time.

**[Stub trap on critical path]** — A module that actually needs `fd_read`
will get `ENOSYS` at runtime, not a compile-time error. Mitigation:
the runner prints which WASI function returned `nosys` so the user knows
what's missing.

**[Store type change]** — Changing `Store<()>` to `Store<WasiCtx>` touches
both runners. Small blast radius since they're short files we own entirely.
