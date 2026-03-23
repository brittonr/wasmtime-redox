//! Minimal wasmtime runner for testing on Redox OS.
//!
//! Exercises the wasmtime runtime API without WASI or tokio.
//! Injected into the wasmtime-cli crate as a [[bin]] during cross-compilation.
//!
//! Usage:
//!   wasm-runner <file.wat|file.wasm|file.cwasm> [--invoke <func> [args...]]
//!   wasm-runner <file> --invoke add 2 3
//!   wasm-runner <file>                          # calls _start

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
        eprintln!("usage: wasm-runner <file> [--invoke <func> [args...]]");
        std::process::exit(2);
    }

    let file = &args[1];

    // Parse --invoke <func> [args...]
    let (func_name, func_args) = if args.len() >= 4 && args[2] == "--invoke" {
        let name = args[3].clone();
        let vals: Vec<i32> = args[4..]
            .iter()
            .filter_map(|a| a.parse().ok())
            .collect();
        (name, vals)
    } else {
        ("_start".to_string(), vec![])
    };

    let engine = Engine::default();
    let module = if file.ends_with(".cwasm") {
        unsafe { Module::deserialize_file(&engine, file)? }
    } else {
        Module::from_file(&engine, file)?
    };

    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    if func_args.is_empty() {
        // Call as () -> () (typical for _start)
        match instance.get_typed_func::<(), ()>(&mut store, &func_name) {
            Ok(f) => {
                f.call(&mut store, ())?;
                println!("OK");
            }
            Err(_) => {
                // Try () -> i32
                let f = instance.get_typed_func::<(), i32>(&mut store, &func_name)?;
                let result = f.call(&mut store, ())?;
                println!("{result}");
            }
        }
    } else if func_args.len() == 2 {
        let f = instance.get_typed_func::<(i32, i32), i32>(&mut store, &func_name)?;
        let result = f.call(&mut store, (func_args[0], func_args[1]))?;
        println!("{result}");
    } else if func_args.len() == 1 {
        let f = instance.get_typed_func::<(i32,), i32>(&mut store, &func_name)?;
        let result = f.call(&mut store, (func_args[0],))?;
        println!("{result}");
    } else {
        eprintln!("error: unsupported arg count (max 2 i32 args)");
        std::process::exit(2);
    }

    Ok(())
}
