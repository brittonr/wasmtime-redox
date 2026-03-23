//! Async/fiber test runner for wasmtime on Redox OS.
//!
//! Exercises wasmtime's fiber-based async support with optional WASI.
//! Uses a manual executor (poll loop) since Redox doesn't need tokio.
//!
//! Usage:
//!   wasm-async-runner <file.wat|file.wasm> [func_name [arg1 arg2]]

use wasmtime::*;

#[path = "wasi/mod.rs"]
mod wasi;

fn main() {
    if let Err(e) = run() {
        if let Some(code) = wasi::wasi_exit_code(&e) {
            std::process::exit(code);
        }
        // Print errors to STDOUT so ion shell captures them
        println!("ERROR: {e:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("usage: wasm-async-runner <file> [func [arg1 arg2]]");
        std::process::exit(2);
    }

    let file = &args[1];
    let no_wasi = args.iter().any(|a| a == "--no-wasi");
    // Filter out --no-wasi before parsing positional args
    let positional: Vec<&String> = args[2..].iter().filter(|a| a.as_str() != "--no-wasi").collect();
    let func_name = positional.first().map(|s| s.as_str()).unwrap_or("_start");
    let func_args: Vec<i32> = positional[1..].iter().filter_map(|a| a.parse().ok()).collect();

    // Use Pulley interpreter for async execution. Cranelift's native x86_64
    // trampoline for void functions crashes on fiber stacks with libcall traps
    // (has_native_signals = false). Pulley handles traps in software, avoiding
    // the interaction entirely.
    let mut config = Config::new();
    config.target("pulley64")?;
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, file)?;

    // Set up WASI context
    let mut ctx = wasi::WasiCtx::new();
    ctx.args = std::env::args().collect();

    let mut store = Store::new(&engine, ctx);

    let mut linker = Linker::new(&engine);
    if !no_wasi {
        wasi::add_wasi_to_linker(&mut linker)?;
    }

    // Async instantiation — exercises fiber context switching
    let instance = block_on(linker.instantiate_async(&mut store, &module))?;

    match func_args.len() {
        0 => {
            match instance.get_typed_func::<(), ()>(&mut store, func_name) {
                Ok(f) => {
                    block_on(f.call_async(&mut store, ()))?;
                    println!("OK");
                }
                Err(_) => {
                    let f = instance.get_typed_func::<(), i32>(&mut store, func_name)?;
                    let result = block_on(f.call_async(&mut store, ()))?;
                    println!("{result}");
                }
            }
        }
        1 => {
            let f = instance.get_typed_func::<(i32,), i32>(&mut store, func_name)?;
            let result = block_on(f.call_async(&mut store, (func_args[0],)))?;
            println!("{result}");
        }
        2 => {
            let f = instance.get_typed_func::<(i32, i32), i32>(&mut store, func_name)?;
            let result = block_on(f.call_async(&mut store, (func_args[0], func_args[1])))?;
            println!("{result}");
        }
        _ => {
            println!("ERROR: unsupported arg count (max 2 i32 args)");
            std::process::exit(2);
        }
    }

    Ok(())
}

/// Drive a Future to completion by polling in a tight loop.
///
/// Wasmtime's async uses fibers (cooperative stack switching), not actual I/O.
/// Futures resolve within a finite number of polls without needing a real
/// async runtime like tokio.
fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    fn noop_raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker { noop_raw_waker() }
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(std::ptr::null(), vtable)
    }

    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(Box::new(future)) };

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(result) => return result,
            Poll::Pending => std::hint::spin_loop(),
        }
    }
}
