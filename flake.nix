{
  description = "Wasmtime WebAssembly runtime cross-compiled for Redox OS";

  inputs = {
    # Wasmtime upstream source (pinned release)
    wasmtime-src = {
      url = "github:bytecodealliance/wasmtime/v43.0.0";
      flake = false;
    };
  };

  outputs =
    { self, wasmtime-src }:
    {
      # The primary API: a function that takes the redox cross-compilation
      # environment and returns the wasmtime package. Called from the redox
      # flake as:
      #   inputs.wasmtime-redox.lib.mkWasmtime standaloneCommon
      #
      # standaloneCommon is an attrset with:
      #   pkgs, lib, rustToolchain, sysrootVendor, redoxTarget,
      #   relibc, stubLibs, vendor
      lib.mkWasmtime = import ./nix/package.nix { inherit wasmtime-src; selfSrc = self; };

      # Precompiled WASM modules (host-side, no cross-compilation needed)
      lib.mkPrecompiled =
        { pkgs }:
        import ./nix/precompile.nix { inherit pkgs; selfSrc = self; };
    };
}
