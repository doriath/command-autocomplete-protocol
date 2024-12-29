{
  description = "Flake for Command Autocompletion Server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        manifest = (pkgs.lib.importTOML ./crates/cas/Cargo.toml).package;
        rust = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.recurseIntoAttrs (
          pkgs.makeRustPlatform {
            rustc = rust;
            cargo = rust;
          }
        );
        cas = rustPlatform.buildRustPackage {
          name = manifest.name;
          version = manifest.version;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          src = pkgs.lib.cleanSource ./.;
          nativeBuildInputs = [ pkgs.pkg-config ];
        };
      in
      rec {
        packages = flake-utils.lib.flattenTree { cas = cas; };

        defaultPackage = packages.cas;

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.rust-analyzer
            rust
          ];
        };
      }
    );
}
