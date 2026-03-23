{
  description = "Wasmtime WebAssembly runtime cross-compiled for Redox OS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Wasmtime upstream source (pinned release)
    wasmtime-src = {
      url = "github:bytecodealliance/wasmtime/v43.0.0";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      wasmtime-src,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      # ── Library API (consumed by the Redox flake) ───────────────────

      # The primary API: a function that takes the redox cross-compilation
      # environment and returns the wasmtime package. Called from the redox
      # flake as:
      #   inputs.wasmtime-redox.lib.mkWasmtime standaloneCommon
      #
      # standaloneCommon is an attrset with:
      #   pkgs, lib, rustToolchain, sysrootVendor, redoxTarget,
      #   relibc, stubLibs, vendor
      lib.mkWasmtime = import ./nix/package.nix {
        inherit wasmtime-src;
        selfSrc = self;
      };

      # Precompiled WASM modules (host-side, no cross-compilation needed)
      lib.mkPrecompiled =
        { pkgs }:
        import ./nix/precompile.nix { inherit pkgs; selfSrc = self; };

      # ── Standalone outputs (work without Redox toolchain) ───────────

      packages = forAllSystems (system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        # Precompile examples to Pulley bytecode using host wasmtime
        precompiled = import ./nix/precompile.nix { inherit pkgs; selfSrc = self; };

        default = self.packages.${system}.precompiled;
      });

      # nix run — compile and inspect the hello.wat example
      apps = forAllSystems (system: let
        pkgs = nixpkgs.legacyPackages.${system};
        script = pkgs.writeShellScript "wasmtime-redox-demo" ''
          set -euo pipefail
          wasmtime="${pkgs.wasmtime}/bin/wasmtime"

          echo "=== wasmtime-redox: Pulley bytecode demo ==="
          echo ""
          echo "Wasmtime version: $($wasmtime --version)"
          echo "Source: ${self}/examples/hello.wat"
          echo ""

          tmpdir=$(mktemp -d)
          trap "rm -rf $tmpdir" EXIT

          echo "--- hello.wat ---"
          cat ${self}/examples/hello.wat
          echo ""

          echo "--- Compiling to Pulley bytecode ---"
          $wasmtime compile --target pulley64 ${self}/examples/hello.wat -o "$tmpdir/hello.cwasm"
          echo "Output: hello.cwasm ($(stat -c%s "$tmpdir/hello.cwasm") bytes)"
          echo ""

          echo "--- Running _start on host (native backend) ---"
          $wasmtime run ${self}/examples/hello.wat
          echo "hello.wat _start: passed (2 + 3 == 5 verified)"
          echo ""
          echo "Done. On Redox, the .cwasm runs via the Pulley interpreter."
        '';
      in {
        default = {
          type = "app";
          program = "${script}";
        };
      });

      devShells = forAllSystems (system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = pkgs.mkShell {
          packages = [
            pkgs.wasmtime
            pkgs.wabt
          ];
          shellHook = ''
            echo "wasmtime-redox devshell"
            echo "  wasmtime $(wasmtime --version)"
            echo "  wabt: wasm2wat, wat2wasm, wasm-objdump"
            echo ""
            echo "Try:"
            echo "  wasmtime run --invoke add examples/hello.wat -- 2 3"
            echo "  wasmtime compile --target pulley64 examples/hello.wat -o hello.cwasm"
          '';
        };
      });
    };
}
