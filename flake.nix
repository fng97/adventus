{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            postgresql

            cargo-edit  # for `cargo upgrade`

            openssl
            pkg-config
            rust-bin.stable.latest.default

            # use nightly to check for unused deps:
            # rust-bin.nightly.latest.default
            # cargo-udeps
          ];
        };
      }
    );
}
