{
  description = "take-note Rust CLI";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/6b5e5b7a6631f065bf6908986990b37d845f847f";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      lib = nixpkgs.lib;
      forAllSystems = lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "take-note";
            version = "2.3.7";

            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = {
              description = "A CLI for creating and managing weekly and daily markdown notes";
              homepage = "https://github.com/wsinned/take-note-rs";
              license = lib.licenses.mit;
              mainProgram = "take-note";
              platforms = systems;
            };
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/take-note";
        };
      });

      checks = forAllSystems (system: {
        default = self.packages.${system}.default;
      });
    };
}
