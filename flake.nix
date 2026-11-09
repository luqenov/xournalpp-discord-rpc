{
  description = "Discord Rich Presence for Xournal++ on Linux";

  inputs.nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in {
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "xournal-rpc";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "Discord Rich Presence for Xournal++ on Linux";
            homepage = "https://codeberg.org/luqenov/xournalpp-discord-rpc";
            license = licenses.mit;
            mainProgram = "xournal-rpc";
            platforms = platforms.linux;
          };
        };
      });

      overlays.default = final: prev: {
        xournal-rpc = self.packages.${final.system}.default;
      };
    };
}
