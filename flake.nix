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
        rust-bin = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
          targets = [ "x86_64-unknown-linux-musl" ];
        };
      in
      {
        devShells.default = with pkgs; mkShell {
          buildInputs = [
            postgresql
            sqlx-cli
            cargo-edit  # for `cargo upgrade`
            rust-analyzer

            openssl
            pkg-config
            rust-bin

            # use nightly to check for unused deps:
            # rust-bin.nightly.latest.default
            # cargo-udeps
          ];

          DATABASE_URL = "postgres://postgres:password@localhost:5432/adventus";
          shellHook = "${pkgs.bash}/bin/bash ./scripts/init_db.sh";
        };
      }
    );
}
