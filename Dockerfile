# syntax=docker/dockerfile:1.7
# FRAMEWORK FILE: `./dev update` overwrites it. Put your own changes in
# agents.Dockerfile (agents) or nix/project.nix (packages).
#
# Base layer: a minimal Debian with single-user Nix. Framework packages
# (node, git, gh, ...) come from nix/base.nix and are baked in at build time.
FROM debian:trixie-slim

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=1000

ENV LANG=C.UTF-8 \
    USER=${USERNAME}

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl xz-utils procps \
 && rm -rf /var/lib/apt/lists/* \
 && echo 'export PATH="$HOME/.local/bin:$HOME/.npm-global/bin:$HOME/.nix-profile/bin:/opt/box/bin:$PATH"' \
      > /etc/profile.d/box-path.sh \
 && groupadd --gid "${USER_GID}" "${USERNAME}" \
 && useradd --uid "${USER_UID}" --gid "${USER_GID}" --create-home --shell /bin/bash "${USERNAME}" \
 && mkdir -m 0755 /nix /nix-cache /workspace \
 && chown "${USERNAME}:${USERNAME}" /nix /nix-cache /workspace

USER ${USERNAME}
WORKDIR /home/${USERNAME}

RUN curl -fsSL https://nixos.org/nix/install \
    | sh -s -- --no-daemon --no-channel-add --no-modify-profile

ENV PATH=/home/${USERNAME}/.local/bin:/home/${USERNAME}/.npm-global/bin:/home/${USERNAME}/.nix-profile/bin:/opt/box/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    NPM_CONFIG_PREFIX=/home/${USERNAME}/.npm-global \
    CLAUDE_CONFIG_DIR=/home/${USERNAME}/.claude \
    HISTFILE=/home/${USERNAME}/.box-state/bash_history

COPY --chown=${USERNAME}:${USERNAME} .box/runtime/nix/nix.conf /home/${USERNAME}/.config/nix/nix.conf
COPY --chown=${USERNAME}:${USERNAME} flake.nix flake.lock /opt/box/flake/
COPY --chown=${USERNAME}:${USERNAME} nix/base.nix /opt/box/flake/nix/base.nix
RUN nix profile install /opt/box/flake#base \
 && nix store optimise

COPY --chown=${USERNAME}:${USERNAME} .box/runtime/ /opt/box/
RUN chmod +x /opt/box/bin/* \
 && mkdir -p ~/.npm-global ~/.local/bin ~/.cache ~/.vscode-server ~/.box-state ~/.box-ssh \
             ~/.claude ~/.codex ~/.gemini ~/.config/gh ~/.config/direnv ~/.config/git \
 && cp /opt/box/home/direnvrc ~/.config/direnv/direnvrc \
 && cp /opt/box/home/gitconfig ~/.config/git/config \
 && cat /opt/box/home/bashrc >> ~/.bashrc \
 && sed -i '1i [ -n "${SSH_CONNECTION:-}" ] && [ -r /tmp/box-env.sh ] && . /tmp/box-env.sh' ~/.bashrc

WORKDIR /workspace
CMD ["/opt/box/bin/entrypoint"]
