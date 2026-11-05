{
  description = "Discord Rich Presence for Xournal++ on Linux";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in {
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "xournal-rpc";
          version = "0.2.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          meta = with pkgs.lib; {
            description = "Discord Rich Presence for Xournal++ on Linux";
            homepage = "https://github.com/luqenov/xournalpp-discord-rpc";
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
