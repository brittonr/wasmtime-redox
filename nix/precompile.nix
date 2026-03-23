# Precompile WASM modules to Pulley bytecode (.cwasm) for Redox
#
# Uses the host wasmtime to compile .wat/.wasm files to .cwasm
# targeting the Pulley interpreter.

{ pkgs, selfSrc }:

let
  wasmtime = pkgs.wasmtime;
  helloWat = "${selfSrc}/examples/hello.wat";
in
pkgs.stdenv.mkDerivation {
  pname = "wasmtime-precompiled-modules";
  version = "0.1.0";

  dontUnpack = true;

  nativeBuildInputs = [ wasmtime ];

  buildPhase = ''
    export HOME=$(mktemp -d)
    echo "Precompiling WASM modules with wasmtime $(wasmtime --version)"
    echo "Target: pulley64"

    mkdir -p modules
    wasmtime compile --target pulley64 ${helloWat} -o modules/hello.cwasm
    echo "hello.cwasm: $(ls -la modules/hello.cwasm | awk '{print $5}') bytes"
  '';

  installPhase = ''
    mkdir -p $out/share/wasm
    cp modules/*.cwasm $out/share/wasm/
    cp ${helloWat} $out/share/wasm/hello.wat
  '';

  meta = {
    description = "Precompiled WASM modules (Pulley bytecode) for Redox";
  };
}
