# Returns a function: args -> derivation
#
# Called from the redox flake as:
#   inputs.wasmtime-redox.lib.mkWasmtime {
#     inherit mkUserspace pkgs lib redoxTarget relibc;
#   }
#
# mkUserspace is the redox cross-compilation builder (from mk-userspace.nix).
# The caller constructs it and passes it in — no circular dependency.

{ wasmtime-src, selfSrc }:

{
  mkUserspace,
  pkgs,
  lib,
  redoxTarget,
  relibc,
  ...
}:

let
  redoxPlatformPatch = "${selfSrc}/patches/0001-redox-custom-platform.patch";
  platformC = "${selfSrc}/platform/wasmtime-platform-redox.c";

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
mkUserspace.mkBinary {
  pname = "wasmtime";
  version = "43.0.0";
  src = wasmtime-src;
  binaryName = "wasmtime";

  vendorHash = "sha256-WJz7J9FHGRqni889o2omkC1K+rwZ8iM6SarT1nd1p0Q=";

  cargoBuildFlags = builtins.concatStringsSep " " [
    "--no-default-features"
    "-F compile,cranelift,pulley,wat,logging"
    "-F wasmtime/custom-virtual-memory"
    "-F wasmtime/runtime"
  ];

  preConfigure = ''
    patch -p1 < ${redoxPlatformPatch}
  '';

  postConfigure = ''
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

  meta = with lib; {
    description = "WebAssembly runtime for Redox OS (Pulley interpreter)";
    homepage = "https://wasmtime.dev";
    license = licenses.asl20;
    mainProgram = "wasmtime";
  };
}
