# Returns a function: standaloneCommon -> derivation
#
# Called from the redox flake as:
#   inputs.wasmtime-redox.lib.mkWasmtime standaloneCommon
#
# standaloneCommon contains: pkgs, lib, rustToolchain, sysrootVendor,
# redoxTarget, relibc, stubLibs, vendor (and possibly others via ...).

{ wasmtime-src, selfSrc }:

{
  pkgs,
  lib,
  rustToolchain,
  sysrootVendor,
  redoxTarget,
  relibc,
  stubLibs,
  vendor,
  ...
}:

let
  mkUserspace = import ./mk-userspace.nix {
    inherit
      pkgs
      lib
      rustToolchain
      sysrootVendor
      redoxTarget
      relibc
      stubLibs
      vendor
      ;
  };

  redoxPlatformPatch = "${selfSrc}/patches/0001-redox-custom-platform.patch";
  platformC = "${selfSrc}/platform/wasmtime-platform-redox.c";
  runnerSrc = "${selfSrc}/runner/main.rs";
  asyncRunnerSrc = "${selfSrc}/runner/async-main.rs";
  wasiSrc = "${selfSrc}/runner/wasi/mod.rs";

  wasmtimePlatformLib = pkgs.stdenv.mkDerivation {
    pname = "wasmtime-platform-redox-lib";
    version = "43.0.0";
    dontUnpack = true;
    nativeBuildInputs = [ pkgs.llvmPackages.clang-unwrapped pkgs.llvmPackages.bintools ];
    buildPhase = ''
      ${pkgs.llvmPackages.clang-unwrapped}/bin/clang \
        --target=${redoxTarget} \
        --sysroot=${relibc}/${redoxTarget} \
        -I${relibc}/${redoxTarget}/include \
        -D__redox__ \
        -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 \
        -c ${platformC} -o wasmtime-platform-redox.o
      llvm-ar crs libwasmtime-platform-redox.a wasmtime-platform-redox.o
    '';
    installPhase = ''
      mkdir -p $out/lib
      cp libwasmtime-platform-redox.a $out/lib/
    '';
  };

in
mkUserspace.mkPackage {
  pname = "wasmtime";
  version = "43.0.0";
  src = wasmtime-src;

  vendorHash = "sha256-WJz7J9FHGRqni889o2omkC1K+rwZ8iM6SarT1nd1p0Q=";

  cargoBuildFlags = builtins.concatStringsSep " " [
    "--no-default-features"
    "-F compile,cranelift,pulley,wat,logging"
    "-F wasmtime/custom-virtual-memory"
    "-F wasmtime/runtime"
    "-F wasmtime/async"
    "--bin wasmtime"
    "--bin wasm-runner"
    "--bin wasm-async-runner"
  ];

  preConfigure = ''
    patch -p1 < ${redoxPlatformPatch}
  '';

  postConfigure = ''
    # Inject runner binaries and WASI shim into the wasmtime-cli crate
    cp ${runnerSrc} src/bin/wasm-runner.rs
    cp ${asyncRunnerSrc} src/bin/wasm-async-runner.rs
    mkdir -p src/bin/wasi
    cp ${wasiSrc} src/bin/wasi/mod.rs

    cat >> Cargo.toml << 'EXTRA_BINS'

[[bin]]
name = "wasm-runner"
path = "src/bin/wasm-runner.rs"
doc = false

[[bin]]
name = "wasm-async-runner"
path = "src/bin/wasm-async-runner.rs"
doc = false
EXTRA_BINS

    # Link the custom platform library
    cat >> crates/wasmtime/build.rs << 'BUILD_RS_APPEND'

// Added by wasmtime-redox: link the custom platform library
fn main_redox_platform() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("redox") {
        if let Ok(lib_dir) = std::env::var("WASMTIME_PLATFORM_LIB_DIR") {
            println!("cargo:rustc-link-search=native={lib_dir}");
            println!("cargo:rustc-link-lib=static=wasmtime-platform-redox");
        }
    }
}
BUILD_RS_APPEND

    sed -i '/custom_cfg("default_target_pulley"/a\    main_redox_platform();' crates/wasmtime/build.rs
  '';

  preBuild = ''
    export WASMTIME_PLATFORM_LIB_DIR="${wasmtimePlatformLib}/lib"
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/share/wasm
    cp target/${redoxTarget}/release/wasmtime $out/bin/
    cp target/${redoxTarget}/release/wasm-runner $out/bin/
    cp target/${redoxTarget}/release/wasm-async-runner $out/bin/

    # Install test WAT files for on-device testing
    cp ${selfSrc}/examples/*.wat $out/share/wasm/
    runHook postInstall
  '';

  meta = with lib; {
    description = "WebAssembly runtime for Redox OS (Pulley interpreter + native x86_64)";
    homepage = "https://wasmtime.dev";
    license = licenses.asl20;
    mainProgram = "wasmtime";
  };
}
