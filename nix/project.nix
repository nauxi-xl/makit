# USER FILE: `./dev update` never touches it.
#
# Packages for this project. They load automatically (direnv) when you
# open a shell in /workspace; no rebuild needed. Search names at
# https://search.nixos.org/packages
#
# Note: Nix flakes only see files tracked by git, so `git add` new .nix files.
pkgs: with pkgs; [
  # Makit is written in Rust.
  cargo
  rustc
  clippy
  rustfmt
  # rustc links through `cc`; also the C toolchain for Makit's own tests.
  gcc
]
