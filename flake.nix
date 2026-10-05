# FRAMEWORK FILE: `./dev update` overwrites it.
# Add packages in nix/project.nix, not here.
{
  description = "agent-devbox environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      # Baked into the image at build time (Dockerfile).
      packages = forAll (pkgs: {
        base = pkgs.buildEnv {
          name = "box-base";
          paths = import ./nix/base.nix pkgs;
        };
      });

      # Loaded at runtime by direnv (.envrc) when you enter /workspace.
      devShells = forAll (pkgs: {
        default = pkgs.mkShellNoCC {
          packages = import ./nix/project.nix pkgs;
        };
      });
    };
}
