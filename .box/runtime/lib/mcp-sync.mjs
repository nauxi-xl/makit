// FRAMEWORK FILE. Generates each installed agent's MCP config from the
// single source of truth, <workspace>/mcp.json (standard `mcpServers` shape).
//
//   claude  -> <workspace>/.mcp.json (+ auto-approve in .claude/settings.local.json)
//   codex   -> $CODEX_HOME/config.toml   ([mcp_servers.<name>] tables)
//   agy     -> ~/.gemini/config/mcp_config.json  (http servers use `serverUrl`)
//
// Only servers named in mcp.json are touched; everything else in those
// files is preserved. Re-running is safe.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const ws = process.argv[2] || "/workspace";
const source = path.join(ws, "mcp.json");
if (!fs.existsSync(source)) {
  console.log(`[box] no ${source}, skipping MCP sync`);
  process.exit(0);
}
const servers = JSON.parse(fs.readFileSync(source, "utf8")).mcpServers ?? {};
const names = Object.keys(servers);
const home = os.homedir();

const installed = (bin) => {
  try {
    execFileSync("sh", ["-c", `command -v ${bin}`], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
};

const readJson = (file) => {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    return {};
  }
};

const writeFile = (file, text) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text);
};

const writeJson = (file, value) => writeFile(file, JSON.stringify(value, null, 2) + "\n");

// ${VAR} and ${VAR:-default}, resolved from the container env.
const expand = (value) =>
  String(value).replace(/\$\{([A-Za-z0-9_]+)(?::-([^}]*))?\}/g, (_, name, fallback) =>
    process.env[name] || fallback || "",
  );

// For agents that cannot expand ${VAR} themselves. Empty env/header values
// are dropped so optional keys (e.g. CONTEXT7_API_KEY) can stay unset.
function resolved(server) {
  const out = structuredClone(server);
  for (const key of ["env", "headers"]) {
    if (!out[key]) continue;
    for (const [name, value] of Object.entries(out[key])) {
      const v = expand(value);
      if (v) out[key][name] = v;
      else delete out[key][name];
    }
    if (Object.keys(out[key]).length === 0) delete out[key];
  }
  if (out.args) out.args = out.args.map(expand);
  if (out.url) out.url = expand(out.url);
  return out;
}

const done = [];

if (installed("claude")) {
  // Claude Code expands ${VAR} itself, so secrets stay out of the file.
  writeJson(path.join(ws, ".mcp.json"), { mcpServers: servers });
  const settingsFile = path.join(ws, ".claude", "settings.local.json");
  const settings = readJson(settingsFile);
  settings.enableAllProjectMcpServers = true;
  writeJson(settingsFile, settings);
  done.push("claude");
}

if (installed("codex")) {
  const file = path.join(process.env.CODEX_HOME || path.join(home, ".codex"), "config.toml");
  const current = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
  const tables = Object.entries(servers).map(([name, s]) => codexTable(name, resolved(s)));
  writeFile(file, [stripCodexTables(current, names).trimEnd(), ...tables].join("\n\n").trimStart() + "\n");
  done.push("codex");
}

if (installed("agy")) {
  const file = path.join(home, ".gemini", "config", "mcp_config.json");
  const config = readJson(file);
  config.mcpServers ??= {};
  for (const [name, s] of Object.entries(servers)) {
    const r = resolved(s);
    if (r.url) {
      r.serverUrl = r.url;
      delete r.url;
      delete r.type;
    }
    config.mcpServers[name] = r;
  }
  writeJson(file, config);
  done.push("agy");
}

console.log(`[box] MCP servers (${names.join(", ") || "none"}) -> ${done.join(", ") || "no agents installed"}`);

// Drops [mcp_servers.<name>] and its sub-tables for every name we manage.
function stripCodexTables(toml, managed) {
  const owned = (header) =>
    managed.some((n) => {
      const base = `mcp_servers.${n}`;
      const quoted = `mcp_servers."${n}"`;
      return [base, quoted].some((b) => header === b || header.startsWith(`${b}.`));
    });
  let skipping = false;
  return toml
    .split(/\r?\n/)
    .filter((line) => {
      const header = line.match(/^\s*\[([^\[\]]+)\]\s*(#.*)?$/);
      if (header) skipping = owned(header[1].trim());
      else if (/^\s*\[\[/.test(line)) skipping = false;
      return !skipping;
    })
    .join("\n");
}

function codexTable(name, s) {
  const str = (v) => JSON.stringify(String(v));
  const inline = (obj) => `{ ${Object.entries(obj).map(([k, v]) => `${str(k)} = ${str(v)}`).join(", ")} }`;
  const lines = [`[mcp_servers.${name}]`];
  if (s.url) {
    lines.push(`url = ${str(s.url)}`);
    if (s.headers) lines.push(`http_headers = ${inline(s.headers)}`);
  } else {
    lines.push(`command = ${str(s.command)}`);
    if (s.args?.length) lines.push(`args = [${s.args.map(str).join(", ")}]`);
    if (s.env) lines.push(`env = ${inline(s.env)}`);
  }
  return lines.join("\n");
}
