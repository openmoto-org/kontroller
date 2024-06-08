{
  description = "openmoto kontroller devenv";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    esp-dev = {
      url = "github:mirrexagon/nixpkgs-esp-dev";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, esp-dev, ... }:
    {
      overlays.default = import ./nix/overlay.nix;
    } // flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [
            fenix.overlays.default
            esp-dev.overlays.default
            self.overlays.default
          ];

          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rustToolchain = with fenix.packages.${system}; combine [
            pkgs.rust-esp
            pkgs.rust-src-esp
          ];
        in
        {
          devShells.default = with pkgs; mkShellNoCC {
            packages = [
              nil
              nixpkgs-fmt
            ];

            buildInputs = [
              openssl
              pkg-config
              ldproxy
              esp-idf-esp32s3-with-clang
              rustToolchain
              cargo-generate
              cargo-espflash
              platformio
            ] ++ lib.optional stdenv.isDarwin [ libiconv ];

            shellHook = ''
              unset IDF_PATH
              unset IDF_TOOLS_PATH
              unset IDF_PYTHON_CHECK_CONSTRAINTS
              unset IDF_PYTHON_ENV_PATH

              export PLATFORMIO_CORE_DIR=$PWD/.platformio

              # NOTE: this is installed by nixpkgs-esp-dev, but not given as part
              # of the available packages and thus being able to be referenced.
              export CLANG_PATH="$(dirname $(which clang))"
              export LIBCLANG_PATH="$CLANG_PATH/../lib"
              export LIBCLANG_BIN_PATH="$CLANG_PATH/../lib"
            '';
          };
        }
      );
}
