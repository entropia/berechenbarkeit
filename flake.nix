{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  inputs.devenv.url = "github:cachix/devenv";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } {
    imports = [
      inputs.devenv.flakeModule
    ];
    systems = inputs.nixpkgs.lib.systems.flakeExposed;
    perSystem = { config, pkgs, inputs', self', system, ... }: {
      devenv.shells.default = {
        packages = with pkgs; [
          pkg-config
          postgresql.lib
          openssl.dev
          sqlx-cli
        ];
        services.postgres = {
          enable = true;
          package = pkgs.postgresql_15;
          initialDatabases = [
            { name = "berechenbarkeit"; }
          ];
        };
      };
    };
  };
}
