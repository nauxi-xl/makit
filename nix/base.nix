# FRAMEWORK FILE: `./dev update` overwrites it.
# Packages every devbox needs. Project packages go in nix/project.nix.
pkgs: with pkgs; [
  bashInteractive
  git
  gh
  ripgrep
  fd
  jq
  less
  nano
  nodejs_24
  uv
  direnv
  nix-direnv
  openssh
  github-mcp-server
]
