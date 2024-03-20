{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustc # rust deps
    pkgs.cargo # rust deps
    pkgs.clippy # rust deps
    pkgs.rustPlatform.rust-src

    pkgs.cargo-shuttle # infrastructure

    pkgs.darwin.apple_sdk.frameworks.SystemConfiguration # for shuttle

    pkgs.nixpkgs-fmt # for nix formatting

    pkgs.sqlx-cli # for test database
    pkgs.docker # for test database
    pkgs.postgresql # for test database

    pkgs.shfmt # vscode shell formatting dep
    pkgs.libiconv # shuttle dep
    pkgs.yt-dlp # songbird youtube downloader dep
  ];

  shellHook = ''
    echo "Development environment loaded."
  '';
}
