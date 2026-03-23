# WASI Preview 1 Tasks

## Phase 1: Foundation

- [x] Create `runner/wasi/mod.rs` with `WasiCtx` struct and `add_wasi_to_linker` skeleton ✅
- [x] Implement `proc_exit` as a typed trap (`WasiExit(i32)`) ✅
- [x] Register all ~30 stub functions returning `errno::nosys` ✅
- [x] Update `runner/main.rs` to use `Store<WasiCtx>` and link WASI imports ✅
- [x] Update `runner/async-main.rs` to use `Store<WasiCtx>` and link WASI imports ✅
- [x] Verify existing pure-compute modules still work (no WASI imports = no change) ✅

## Phase 2: Core Functions

- [x] Implement `fd_write` for fd 1 (stdout) and fd 2 (stderr) ✅
- [x] Write `examples/hello-wasi.wat` that calls `fd_write` to print to stdout ✅
- [x] Implement `args_get` and `args_sizes_get` ✅
- [x] Write `examples/args-wasi.wat` that reads and prints args ✅
- [x] Implement `environ_get` and `environ_sizes_get` ✅
- [x] Implement `clock_time_get` for monotonic and realtime clocks ✅
- [x] Implement `random_get` via /dev/urandom ✅
- [x] Write `examples/clock-wasi.wat` that reads monotonic clock twice and checks ordering ✅

## Phase 3: Build Integration

- [x] Update `nix/package.nix` to include `runner/wasi/mod.rs` in the injected sources ✅
- [x] Confirm linker API is available without wasmtime-wasi feature flags ✅
- [x] Cross-compile and verify binary size delta ✅ (wasmtime binary cached, no size regression)
- [x] Test all examples on Redox VM ✅ 12/13 pass (async-start is pre-existing)

## Phase 4: Polish

- [x] Print diagnostic when a stub returns `nosys` (one-time stderr warning per function) ✅
- [x] Add `--no-wasi` flag to runners for modules that don't need it ✅
- [x] Update README.md with WASI support status table ✅
- [x] Update the "Tested on Redox" table with WASI test results ✅
