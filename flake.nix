{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11"; };

  outputs = { nixpkgs, ... }:
    let
      # boilerplate for cross-platform builds/shells
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = nixpkgs.legacyPackages;

      # function to create package for each supported system
      makePackage = system:
        let
          pkgs = pkgsFor.${system};
          manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        in pkgs.rustPlatform.buildRustPackage {
          pname = manifest.name;
          version = manifest.version;
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          # build-time dependencies
          nativeBuildInputs = with pkgs;
            [ pkg-config ] ++ lib.optionals pkgs.stdenv.isDarwin
            [ pkgs.darwin.apple_sdk.frameworks.SystemConfiguration ];
          # run-time dependencies
          buildInputs = with pkgs; [ openssl ffmpeg libopus ];
        };

      # function to create dev shell for each supported system
      makeDevShell = system:
        let
          pkgs = pkgsFor.${system};
          rustPackage = makePackage system;
        in pkgs.mkShell {
          inputsFrom = [ rustPackage ];
          buildInputs = with pkgs; [ rustc cargo rust-analyzer cargo-edit ];
        };
    in {
      packages = forAllSystems (system: { default = makePackage system; });
      devShells = forAllSystems (system: { default = makeDevShell system; });
    };
}
