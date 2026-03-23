//! Minimal wasmtime runner for testing on Redox OS.
//!
//! Exercises the wasmtime runtime API with optional WASI support.
//! Injected into the wasmtime-cli crate as a [[bin]] during cross-compilation.
//!
//! Usage:
//!   wasm-runner <file.wat|file.wasm|file.cwasm> [--invoke <func> [args...]]
//!   wasm-runner <file> --invoke add 2 3
//!   wasm-runner <file>                          # calls _start
//!   wasm-runner <file> --roundtrip              # compile → serialize → deserialize → run

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
        println!("usage: wasm-runner <file> [--invoke <func> [args...]] [--roundtrip] [--no-wasi]");
        std::process::exit(2);
    }

    let file = &args[1];
    let roundtrip = args.iter().any(|a| a == "--roundtrip");
    let no_wasi = args.iter().any(|a| a == "--no-wasi");

    // Parse --invoke <func> [args...]
    let (func_name, func_args) = if let Some(pos) = args.iter().position(|a| a == "--invoke") {
        let name = args.get(pos + 1).cloned().unwrap_or_else(|| "_start".into());
        let vals: Vec<i32> = args[pos + 2..]
            .iter()
            .take_while(|a| a.as_str() != "--roundtrip")
            .filter_map(|a| a.parse().ok())
            .collect();
        (name, vals)
    } else {
        ("_start".to_string(), vec![])
    };

    let engine = Engine::default();

    let module = if file.ends_with(".cwasm") {
        unsafe { Module::deserialize_file(&engine, file)? }
    } else if roundtrip {
        // Compile → serialize → deserialize (tests cwasm roundtrip)
        let original = Module::from_file(&engine, file)?;
        let bytes = original.serialize()?;
        unsafe { Module::deserialize(&engine, &bytes)? }
    } else {
        Module::from_file(&engine, file)?
    };

    // Set up WASI context with the runner's own args forwarded
    let mut ctx = wasi::WasiCtx::new();
    ctx.args = std::env::args().collect();

    let mut store = Store::new(&engine, ctx);

    let mut linker = Linker::new(&engine);
    if !no_wasi {
        wasi::add_wasi_to_linker(&mut linker)?;
    }
    let instance = linker.instantiate(&mut store, &module)?;

    call_func(&mut store, &instance, &func_name, &func_args)
}

fn call_func(store: &mut Store<wasi::WasiCtx>, instance: &Instance, name: &str, args: &[i32]) -> Result<()> {
    match args.len() {
        0 => {
            match instance.get_typed_func::<(), ()>(&mut *store, name) {
                Ok(f) => {
                    f.call(&mut *store, ())?;
                    println!("OK");
                }
                Err(_) => {
                    let f = instance.get_typed_func::<(), i32>(&mut *store, name)?;
                    let result = f.call(&mut *store, ())?;
                    println!("{result}");
                }
            }
        }
        1 => {
            let f = instance.get_typed_func::<(i32,), i32>(&mut *store, name)?;
            let result = f.call(&mut *store, (args[0],))?;
            println!("{result}");
        }
        2 => {
            let f = instance.get_typed_func::<(i32, i32), i32>(&mut *store, name)?;
            let result = f.call(&mut *store, (args[0], args[1]))?;
            println!("{result}");
        }
        _ => {
            println!("ERROR: unsupported arg count (max 2 i32 args)");
            std::process::exit(2);
        }
    }
    Ok(())
}
