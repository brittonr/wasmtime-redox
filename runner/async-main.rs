//! Async/fiber test runner for wasmtime on Redox OS.
//!
//! Exercises wasmtime's fiber-based async support without tokio.
//! Uses a manual executor (poll loop) since Redox doesn't need tokio.
//!
//! Tests:
//!   1. Create an engine + store
//!   2. Load a module, instantiate it via new_async (fiber context switch)
//!   3. Call a function via call_async (fiber context switch)
//!
//! Usage:
//!   wasm-async-runner <file.wat|file.wasm> [func_name [arg1 arg2]]

use wasmtime::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: wasm-async-runner <file> [func [arg1 arg2]]");
        std::process::exit(2);
    }

    let file = &args[1];
    let func_name = args.get(2).map(|s| s.as_str()).unwrap_or("_start");
    let func_args: Vec<i32> = args[3..].iter().filter_map(|a| a.parse().ok()).collect();

    let engine = Engine::default();
    let module = Module::from_file(&engine, file)?;

    let mut store = Store::new(&engine, ());

    // Instantiate via async (exercises fiber context switching)
    let instance = block_on(Instance::new_async(&mut store, &module, &[]))?;

    if func_args.len() == 2 {
        let f = instance.get_typed_func::<(i32, i32), i32>(&mut store, func_name)?;
        let result = block_on(f.call_async(&mut store, (func_args[0], func_args[1])))?;
        println!("{result}");
    } else if func_args.len() == 1 {
        let f = instance.get_typed_func::<(i32,), i32>(&mut store, func_name)?;
        let result = block_on(f.call_async(&mut store, (func_args[0],)))?;
        println!("{result}");
    } else {
        // Try () -> () first (_start), fall back to () -> i32
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
