#!/usr/bin/env bash
# Smoke test for the template itself (CI and local). Run after ./dev up.
set -euo pipefail
cd "$(dirname "$0")/.."

in_box() { docker compose -f .box/compose.yml exec -T dev bash -lc "$1"; }
check() { echo "--- $1"; in_box "$2"; }

check "claude"          'claude --version'
check "codex"           'codex --version'
check "agy"             'agy --version'
check "nix devShell"    'cd /workspace && nix develop -c true'
check "agentmemory"     'curl -fsS http://agentmemory:3111/agentmemory/livez'
check "claude mcp"      'jq -e ".mcpServers | has(\"agentmemory\") and has(\"context7\") and has(\"github\")" /workspace/.mcp.json'
check "codex mcp"       'grep -q "^\[mcp_servers.github\]" ~/.codex/config.toml && ! grep -q localhost:3111 ~/.codex/config.toml'
check "agy mcp"         'jq -e ".mcpServers.github" ~/.gemini/config/mcp_config.json'
check "nix cache sync"  'box-nix-sync && ls /nix-cache/*.narinfo >/dev/null'

echo "--- Reopen in Container reuses the ./dev container"
before=$(docker compose -f .box/compose.yml ps -q dev)
after=$(npx -y @devcontainers/cli up --workspace-folder . | tail -n1 | jq -r .containerId)
echo "dev=$before devcontainer=$after"
[ "${after:0:12}" = "${before:0:12}" ]

echo "smoke OK"
