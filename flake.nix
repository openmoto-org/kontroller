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

  outputs = { nixpkgs, flake-utils, fenix, esp-dev, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [
            fenix.overlays.default
            esp-dev.overlays.default
          ];

          pkgs = import nixpkgs {
            inherit system overlays;
          };

          rustToolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-Fj+OcOTyexYiW/3M1X1YkNJ/tnuHStX/meU7MHC3AxY=";
          };
        in
        {
          devShells.default = with pkgs; mkShell {
            packages = [
              nil
              nixpkgs-fmt
            ];

            buildInputs = [
              openssl
              pkg-config
              rustToolchain
              cargo-generate
              cargo-espflash
              ldproxy

              # esp-idf-esp32c3
            ] ++ lib.optional stdenv.isDarwin [ libiconv ];
          };
        }
      );
}
