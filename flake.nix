{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11"; };

  outputs = { self, nixpkgs, ... }:
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
    } // { # system-generic stuff (don't need to worry about supportedSystems)
      # Adventus is deployed using this NixOS module. See the NixOS test container further down for 
      # how it is installed.
      nixosModule = { pkgs, config, lib, ... }:
        with lib;
        let cfg = config.services.adventus;
        in {
          options.services.adventus = {
            enable = mkEnableOption "Enable Adventus Discord bot";

            # TODO: This is not a secure solution. For a silly Discord bot, it's fine for now. Do 
            # this properly at some point, perhaps using an environment file.
            discordToken = mkOption { type = types.str; };
          };

          config = mkIf cfg.enable {
            systemd.services.adventus = {
              wantedBy = [ "multi-user.target" ];
              wants = [ "network-online.target" ];
              after = [ "network-online.target" ];

              serviceConfig = {
                Restart = "always";
                ExecStart = "${
                    self.packages.${pkgs.stdenv.hostPlatform.system}.default
                  }/bin/adventus";
                DynamicUser = "yes";
                StateDirectory = "adventus";
              };

              # TODO: Duplicates runtime deps listed above. Abstract this.
              path = with pkgs; [ openssl ffmpeg libopus ];

              environment = {
                DISCORD_TOKEN = cfg.discordToken;
                RUST_LOG = "info";
                RUST_BACKTRACE = "1";
                # Use the state directory provided by systemd.
                INTROS_DIR = "/var/lib/adventus/intros";
              };
            };
          };
        };

      # WIP: This NixOS container is used to test the Adventus module before we deploy it.
      nixosConfigurations.container = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          ({ ... }: {
            boot.isContainer = true;

            # We need access to outside network to test the bot. See 
            # https://nixos.org/manual/nixos/stable/#sec-container-networking.
            networking.nat.enable = true;
            networking.nat.internalInterfaces = [ "ve-+" ];
            networking.nat.externalInterface = "eth0";

            # Discord bots require WebSockets so we must enable HTTP ports.
            networking.firewall.allowedTCPPorts = [ 80 443 ];

            # services.adventus = {
            #   enable = true;
            #   discordToken = "";
            # };
          })
        ];
      };
    };
}
