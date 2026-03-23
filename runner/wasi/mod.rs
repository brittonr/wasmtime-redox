//! Minimal WASI preview 1 shim for Redox OS.
//!
//! Implements the subset of wasi_snapshot_preview1 needed to run basic
//! CLI programs: fd_write (stdout/stderr), args, env, clock, proc_exit,
//! random_get. Everything else stubs with ENOSYS.
//!
//! Zero external crate dependencies — uses std and raw libc via relibc.

use wasmtime::*;

// ── WASI errno constants ─────────────────────────────────────────────

const ERRNO_SUCCESS: i32 = 0;
const ERRNO_BADF: i32 = 8;
const ERRNO_INVAL: i32 = 28;
const ERRNO_NOSYS: i32 = 52;

// ── WASI clock IDs ───────────────────────────────────────────────────

const CLOCKID_REALTIME: i32 = 0;
const CLOCKID_MONOTONIC: i32 = 1;

// ── Host state ───────────────────────────────────────────────────────

/// WASI context stored in the wasmtime Store.
pub struct WasiCtx {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl WasiCtx {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            env: Vec::new(),
        }
    }
}

// ── proc_exit handling ───────────────────────────────────────────────

/// Typed exit code from proc_exit. The runner catches this and maps it
/// to std::process::exit.
#[derive(Debug)]
pub struct WasiExit(pub i32);

impl std::fmt::Display for WasiExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "proc_exit({})", self.0)
    }
}

impl std::error::Error for WasiExit {}

/// Check if a wasmtime error wraps a WASI proc_exit. Returns the exit code if so.
pub fn wasi_exit_code(err: &Error) -> Option<i32> {
    err.downcast_ref::<WasiExit>().map(|e| e.0)
}

// ── Guest memory helpers ─────────────────────────────────────────────

fn guest_memory(caller: &mut Caller<'_, WasiCtx>) -> Result<Memory> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| Error::msg("missing guest \"memory\" export"))
}

fn read_u32(mem: &Memory, store: &impl AsContext, offset: u32) -> Result<u32> {
    let data = mem.data(store);
    let bytes = data
        .get(offset as usize..offset as usize + 4)
        .ok_or_else(|| Error::msg(format!("oob read at {offset}")))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn write_u32(mem: &Memory, store: &mut impl AsContextMut, offset: u32, val: u32) -> Result<()> {
    let data = mem.data_mut(store);
    let slot = data
        .get_mut(offset as usize..offset as usize + 4)
        .ok_or_else(|| Error::msg(format!("oob write at {offset}")))?;
    slot.copy_from_slice(&val.to_le_bytes());
    Ok(())
}

fn write_u64(mem: &Memory, store: &mut impl AsContextMut, offset: u32, val: u64) -> Result<()> {
    let data = mem.data_mut(store);
    let slot = data
        .get_mut(offset as usize..offset as usize + 8)
        .ok_or_else(|| Error::msg(format!("oob write at {offset}")))?;
    slot.copy_from_slice(&val.to_le_bytes());
    Ok(())
}

// ── Linker registration ──────────────────────────────────────────────

/// Register all wasi_snapshot_preview1 functions on the linker.
/// Modules that don't import WASI are unaffected — the linker only
/// provides imports the module actually declares.
pub fn add_wasi_to_linker(linker: &mut Linker<WasiCtx>) -> Result<()> {
    let m = "wasi_snapshot_preview1";

    // ── fd_write ─────────────────────────────────────────────────
    // (fd, iovs_ptr, iovs_len, nwritten_ptr) -> errno
    linker.func_wrap(m, "fd_write", |mut caller: Caller<'_, WasiCtx>,
        fd: i32, iovs_ptr: i32, iovs_len: i32, nwritten_ptr: i32| -> Result<i32>
    {
        if fd != 1 && fd != 2 {
            return Ok(ERRNO_BADF);
        }
        let mem = guest_memory(&mut caller)?;
        let mut total: u32 = 0;
        for i in 0..iovs_len as u32 {
            let base = iovs_ptr as u32 + i * 8;
            let buf_ptr = read_u32(&mem, &caller, base)?;
            let buf_len = read_u32(&mem, &caller, base + 4)?;
            let data = mem.data(&caller);
            let slice = data
                .get(buf_ptr as usize..buf_ptr as usize + buf_len as usize)
                .ok_or_else(|| Error::msg("fd_write: iov buf oob"))?;
            use std::io::Write;
            if fd == 1 {
                std::io::stdout().write_all(slice).map_err(Error::new)?;
            } else {
                std::io::stderr().write_all(slice).map_err(Error::new)?;
            }
            total += buf_len;
        }
        write_u32(&mem, &mut caller, nwritten_ptr as u32, total)?;
        Ok(ERRNO_SUCCESS)
    })?;

    // ── fd_read (stub) ───────────────────────────────────────────
    linker.func_wrap(m, "fd_read", |_: Caller<'_, WasiCtx>,
        _fd: i32, _iovs: i32, _iovs_len: i32, _nread: i32| -> i32
    {
        ERRNO_NOSYS
    })?;

    // ── fd_close ─────────────────────────────────────────────────
    // Return success for stdio fds, badf for everything else.
    linker.func_wrap(m, "fd_close", |_: Caller<'_, WasiCtx>, fd: i32| -> i32 {
        if fd <= 2 { ERRNO_SUCCESS } else { ERRNO_BADF }
    })?;

    // ── fd_seek ──────────────────────────────────────────────────
    linker.func_wrap(m, "fd_seek", |_: Caller<'_, WasiCtx>,
        _fd: i32, _offset: i64, _whence: i32, _newoffset: i32| -> i32
    {
        ERRNO_NOSYS
    })?;

    // ── fd_tell ──────────────────────────────────────────────────
    linker.func_wrap(m, "fd_tell", |_: Caller<'_, WasiCtx>,
        _fd: i32, _offset: i32| -> i32
    {
        ERRNO_NOSYS
    })?;

    // ── fd_fdstat_get ────────────────────────────────────────────
    // Many Rust binaries call this on stdio fds at startup.
    // fdstat layout (24 bytes):
    //   offset 0:  fs_filetype  (u8)
    //   offset 2:  fs_flags     (u16)
    //   offset 8:  fs_rights_base       (u64)
    //   offset 16: fs_rights_inheriting (u64)
    linker.func_wrap(m, "fd_fdstat_get", |mut caller: Caller<'_, WasiCtx>,
        fd: i32, buf: i32| -> Result<i32>
    {
        if fd > 2 { return Ok(ERRNO_BADF); }
        let mem = guest_memory(&mut caller)?;
        let data = mem.data_mut(&mut caller);
        let slot = data
            .get_mut(buf as usize..buf as usize + 24)
            .ok_or_else(|| Error::msg("fd_fdstat_get: buf oob"))?;
        slot.fill(0);
        slot[0] = 2; // CHARACTER_DEVICE
        // Leave rights at 0 — most libc impls don't check them.
        Ok(ERRNO_SUCCESS)
    })?;

    // ── fd_fdstat_set_flags ──────────────────────────────────────
    linker.func_wrap(m, "fd_fdstat_set_flags", |_: Caller<'_, WasiCtx>,
        _fd: i32, _flags: i32| -> i32
    {
        ERRNO_NOSYS
    })?;

    // ── fd_prestat_get ───────────────────────────────────────────
    // Rust wasm binaries probe preopened dirs by calling this with
    // increasing fd numbers. Return BADF to signal "no more preopens".
    linker.func_wrap(m, "fd_prestat_get", |_: Caller<'_, WasiCtx>,
        _fd: i32, _buf: i32| -> i32
    {
        ERRNO_BADF
    })?;

    // ── fd_prestat_dir_name ──────────────────────────────────────
    linker.func_wrap(m, "fd_prestat_dir_name", |_: Caller<'_, WasiCtx>,
        _fd: i32, _path: i32, _path_len: i32| -> i32
    {
        ERRNO_NOSYS
    })?;

    // ── args_sizes_get ───────────────────────────────────────────
    linker.func_wrap(m, "args_sizes_get", |mut caller: Caller<'_, WasiCtx>,
        argc_ptr: i32, argv_buf_size_ptr: i32| -> Result<i32>
    {
        let argc = caller.data().args.len() as u32;
        let buf_size: u32 = caller.data().args.iter()
            .map(|a| a.len() as u32 + 1)
            .sum();
        let mem = guest_memory(&mut caller)?;
        write_u32(&mem, &mut caller, argc_ptr as u32, argc)?;
        write_u32(&mem, &mut caller, argv_buf_size_ptr as u32, buf_size)?;
        Ok(ERRNO_SUCCESS)
    })?;

    // ── args_get ─────────────────────────────────────────────────
    linker.func_wrap(m, "args_get", |mut caller: Caller<'_, WasiCtx>,
        argv_ptr: i32, argv_buf_ptr: i32| -> Result<i32>
    {
        let args: Vec<String> = caller.data().args.clone();
        let mem = guest_memory(&mut caller)?;
        let mut buf_off = argv_buf_ptr as u32;
        for (i, arg) in args.iter().enumerate() {
            write_u32(&mem, &mut caller, argv_ptr as u32 + (i as u32) * 4, buf_off)?;
            let bytes = arg.as_bytes();
            let data = mem.data_mut(&mut caller);
            let slot = data
                .get_mut(buf_off as usize..buf_off as usize + bytes.len() + 1)
                .ok_or_else(|| Error::msg("args_get: buf overflow"))?;
            slot[..bytes.len()].copy_from_slice(bytes);
            slot[bytes.len()] = 0;
            buf_off += bytes.len() as u32 + 1;
        }
        Ok(ERRNO_SUCCESS)
    })?;

    // ── environ_sizes_get ────────────────────────────────────────
    linker.func_wrap(m, "environ_sizes_get", |mut caller: Caller<'_, WasiCtx>,
        environc_ptr: i32, environ_buf_size_ptr: i32| -> Result<i32>
    {
        let count = caller.data().env.len() as u32;
        let buf_size: u32 = caller.data().env.iter()
            .map(|(k, v)| k.len() as u32 + 1 + v.len() as u32 + 1) // KEY=VALUE\0
            .sum();
        let mem = guest_memory(&mut caller)?;
        write_u32(&mem, &mut caller, environc_ptr as u32, count)?;
        write_u32(&mem, &mut caller, environ_buf_size_ptr as u32, buf_size)?;
        Ok(ERRNO_SUCCESS)
    })?;

    // ── environ_get ──────────────────────────────────────────────
    linker.func_wrap(m, "environ_get", |mut caller: Caller<'_, WasiCtx>,
        environ_ptr: i32, environ_buf_ptr: i32| -> Result<i32>
    {
        let env: Vec<(String, String)> = caller.data().env.clone();
        let mem = guest_memory(&mut caller)?;
        let mut buf_off = environ_buf_ptr as u32;
        for (i, (k, v)) in env.iter().enumerate() {
            write_u32(&mem, &mut caller, environ_ptr as u32 + (i as u32) * 4, buf_off)?;
            let entry = format!("{k}={v}");
            let bytes = entry.as_bytes();
            let data = mem.data_mut(&mut caller);
            let slot = data
                .get_mut(buf_off as usize..buf_off as usize + bytes.len() + 1)
                .ok_or_else(|| Error::msg("environ_get: buf overflow"))?;
            slot[..bytes.len()].copy_from_slice(bytes);
            slot[bytes.len()] = 0;
            buf_off += bytes.len() as u32 + 1;
        }
        Ok(ERRNO_SUCCESS)
    })?;

    // ── clock_time_get ───────────────────────────────────────────
    linker.func_wrap(m, "clock_time_get", |mut caller: Caller<'_, WasiCtx>,
        id: i32, _precision: i64, time_ptr: i32| -> Result<i32>
    {
        let nanos: u64 = match id {
            CLOCKID_REALTIME => {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0)
            }
            CLOCKID_MONOTONIC => {
                use std::sync::OnceLock;
                use std::time::Instant;
                static START: OnceLock<Instant> = OnceLock::new();
                START.get_or_init(Instant::now).elapsed().as_nanos() as u64
            }
            _ => return Ok(ERRNO_INVAL),
        };
        let mem = guest_memory(&mut caller)?;
        write_u64(&mem, &mut caller, time_ptr as u32, nanos)?;
        Ok(ERRNO_SUCCESS)
    })?;

    // ── proc_exit ────────────────────────────────────────────────
    linker.func_wrap(m, "proc_exit", |_: Caller<'_, WasiCtx>, code: i32| -> Result<()> {
        Err(Error::new(WasiExit(code)))
    })?;

    // ── random_get ───────────────────────────────────────────────
    linker.func_wrap(m, "random_get", |mut caller: Caller<'_, WasiCtx>,
        buf_ptr: i32, buf_len: i32| -> Result<i32>
    {
        let mem = guest_memory(&mut caller)?;
        let data = mem.data_mut(&mut caller);
        let slot = data
            .get_mut(buf_ptr as usize..buf_ptr as usize + buf_len as usize)
            .ok_or_else(|| Error::msg("random_get: buf oob"))?;
        use std::io::Read;
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(slot))
            .map_err(Error::new)?;
        Ok(ERRNO_SUCCESS)
    })?;

    // ── sched_yield ──────────────────────────────────────────────
    linker.func_wrap(m, "sched_yield", |_: Caller<'_, WasiCtx>| -> i32 {
        std::thread::yield_now();
        ERRNO_SUCCESS
    })?;

    // ── Remaining stubs (ENOSYS) ─────────────────────────────────
    register_stubs(linker)?;

    Ok(())
}

/// Print a one-time warning when a stub is called, then return ENOSYS.
macro_rules! stub {
    ($linker:expr, $name:expr, | $($p:ident : $t:ty),* | $ret:ty) => {
        $linker.func_wrap("wasi_snapshot_preview1", $name, |_: Caller<'_, WasiCtx>, $($p : $t),*| -> $ret {
            use std::sync::Once;
            static WARN: Once = Once::new();
            WARN.call_once(|| {
                eprintln!("wasi: {}() not implemented (ENOSYS)", $name);
            });
            ERRNO_NOSYS
        })?
    };
}

fn register_stubs(linker: &mut Linker<WasiCtx>) -> Result<()> {
    // path operations
    stub!(linker, "path_open", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i64, _g: i64, _h: i32, _i: i32| i32);
    stub!(linker, "path_create_directory", |_a: i32, _b: i32, _c: i32| i32);
    stub!(linker, "path_remove_directory", |_a: i32, _b: i32, _c: i32| i32);
    stub!(linker, "path_rename", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32| i32);
    stub!(linker, "path_unlink_file", |_a: i32, _b: i32, _c: i32| i32);
    stub!(linker, "path_readlink", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32| i32);
    stub!(linker, "path_filestat_get", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32| i32);
    stub!(linker, "path_filestat_set_times", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i64, _f: i64, _g: i32| i32);
    stub!(linker, "path_symlink", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32| i32);
    stub!(linker, "path_link", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32, _g: i32| i32);

    // fd operations
    stub!(linker, "fd_filestat_get", |_a: i32, _b: i32| i32);
    stub!(linker, "fd_filestat_set_times", |_a: i32, _b: i64, _c: i64, _d: i32| i32);
    stub!(linker, "fd_filestat_set_size", |_a: i32, _b: i64| i32);
    stub!(linker, "fd_readdir", |_a: i32, _b: i32, _c: i32, _d: i64, _e: i32| i32);
    stub!(linker, "fd_advise", |_a: i32, _b: i64, _c: i64, _d: i32| i32);
    stub!(linker, "fd_allocate", |_a: i32, _b: i64, _c: i64| i32);
    stub!(linker, "fd_datasync", |_a: i32| i32);
    stub!(linker, "fd_sync", |_a: i32| i32);
    stub!(linker, "fd_pread", |_a: i32, _b: i32, _c: i32, _d: i64, _e: i32| i32);
    stub!(linker, "fd_pwrite", |_a: i32, _b: i32, _c: i32, _d: i64, _e: i32| i32);
    stub!(linker, "fd_renumber", |_a: i32, _b: i32| i32);

    // poll
    stub!(linker, "poll_oneoff", |_a: i32, _b: i32, _c: i32, _d: i32| i32);

    // sockets
    stub!(linker, "sock_recv", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32, _f: i32| i32);
    stub!(linker, "sock_send", |_a: i32, _b: i32, _c: i32, _d: i32, _e: i32| i32);
    stub!(linker, "sock_shutdown", |_a: i32, _b: i32| i32);
    stub!(linker, "sock_accept", |_a: i32, _b: i32, _c: i32| i32);

    Ok(())
}
