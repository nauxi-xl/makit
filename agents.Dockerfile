# syntax=docker/dockerfile:1.7
# USER FILE: `./dev update` never touches it.
#
# Installs the coding agents on top of the base image. Each block is one
# agent: delete a block to drop it, copy a block to add another. Apply
# changes with `./dev rebuild`.
#
# Installs run as the `dev` user. npm globals land in ~/.npm-global
# (the Nix store is read-only), native binaries in ~/.local/bin.

ARG BASE_IMAGE
FROM ${BASE_IMAGE}

# --- agentmemory: memory server + MCP shim ---------------------------------
# The iii engine must be exactly the version agentmemory pins (its iii-sdk
# dependency); otherwise agentmemory downloads it again on every start.
RUN npm install -g @agentmemory/agentmemory @agentmemory/mcp \
 && III_VERSION=$(node -p "require('$(npm root -g)/@agentmemory/agentmemory/package.json').dependencies['iii-sdk']") \
 && cd /tmp && curl -fsSL https://install.iii.dev/iii/main/install.sh | VERSION="$III_VERSION" sh \
 && iii --version

# --- Claude Code ------------------------------------------------------------
RUN npm install -g @anthropic-ai/claude-code

# --- Codex CLI --------------------------------------------------------------
RUN npm install -g @openai/codex

# --- Antigravity CLI (agy) --------------------------------------------------
RUN curl -fsSL https://antigravity.google/cli/install.sh | bash

# --- npm-based MCP servers (preinstalled so `npx -y` starts instantly) -------
RUN npm install -g @upstash/context7-mcp
