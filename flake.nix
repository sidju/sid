{
  description = "sid-llvm development shell (Rust + LLVM 18)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          llvmPkgs = pkgs.llvmPackages_18;
          llvmBin = llvmPkgs.llvm.out or llvmPkgs.llvm;
          llvmDev = llvmPkgs.llvm.dev or llvmPkgs.llvm;
          llvmLib = llvmPkgs.libllvm.lib or llvmPkgs.libllvm;
          llvmPrefix = pkgs.symlinkJoin {
            name = "llvm-18-prefix";
            paths = [ llvmBin llvmDev llvmLib ];
          };
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              rust-analyzer
              llvmPkgs.llvm
              llvmPkgs.libllvm
              llvmPkgs.clang
              llvmPkgs.lld
              bash
              pkg-config
              libffi
              libxml2
              just
            ];

            LLVM_SYS_180_PREFIX = "${llvmPrefix}";
            LLVM_CONFIG_PATH = "${llvmPrefix}/bin/llvm-config";
            LIBCLANG_PATH = "${llvmLib}/lib";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ llvmPkgs.libllvm pkgs.libffi pkgs.libxml2 ];

            shellHook = ''
              export PATH="${llvmPrefix}/bin:$PATH"
              echo "sid-llvm dev shell"
              echo "  LLVM_SYS_180_PREFIX=$LLVM_SYS_180_PREFIX"
            '';
          };
        });
    };
}
