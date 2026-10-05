# agent-devbox

Template môi trường làm việc trong Docker cho coding agent: Claude Code, Codex CLI, Antigravity CLI, kèm MCP server (agentmemory, context7, GitHub). Package được quản lý bằng Nix.

```
./dev up        # Git Bash / Linux / macOS
.\dev up        # PowerShell  (hoặc dev.cmd up nếu bị chặn ExecutionPolicy)
```

Xong lệnh trên thì có thể:
- mở VS Code bằng `./dev code`, hoặc mở folder trong VS Code rồi chạy **Dev Containers: Reopen in Container** (dùng lại đúng container vừa tạo)
- vào terminal bằng `./dev shell`
- xem memory viewer ở http://localhost:3113

Yêu cầu: Docker (Desktop) với Compose ≥ 2.24. Nếu dùng VS Code thì cần extension *Dev Containers*.

## Lệnh

| Lệnh | Việc |
|---|---|
| `up [--code]` | Build image, khởi động memory server dùng chung và container dev |
| `shell` | Mở bash trong container |
| `code` | Mở VS Code bên trong container |
| `down` | Xoá container dev (memory server vẫn chạy) |
| `rebuild` | Build lại không dùng cache để lấy bản agent mới, rồi tạo lại container |
| `update` | Kéo file framework mới từ template repo |
| `init <path>` | Áp template vào một project có sẵn |

Ngoài ra có `connect` (đăng ký lại MCP và hooks) và `memory up|down|restart|logs`.

## Tuỳ biến (file của bạn, `update` không ghi đè)

- **`agents.Dockerfile`**: mỗi block `RUN` cài một agent. Thêm hoặc xoá block rồi chạy `./dev rebuild`.
- **`nix/project.nix`**: package cho project. direnv tự nạp khi vào shell, không cần rebuild. Lưu ý phải `git add` vì flake chỉ thấy các file đã được git track.
- **`mcp.json`**: danh sách MCP server, theo định dạng `mcpServers` chuẩn. Khi container start, `box-connect` sinh config tương ứng cho từng agent:
  - Claude Code: `.mcp.json` của project. Giá trị `${VAR}` được giữ nguyên để Claude tự expand.
  - Codex: `~/.codex/config.toml`
  - Antigravity CLI: `~/.gemini/config/mcp_config.json`. Server HTTP dùng key `serverUrl`.

  Codex và Antigravity không tự expand `${VAR}`, nên secret được ghi thẳng vào config của chúng (nằm trong `DEVENV_HOME`, không nằm trong repo).
- **`.env`** (tạo từ `.env.example`): API key, `GITHUB_TOKEN`, `ENABLE_SSH`, `DEVENV_HOME`, `BOX_PROJECT`.
- `.envrc`, `.gitignore`
- **`.claude/skills/`**: bộ skill cho Claude Code. `init`/`update` chỉ copy khi project chưa có thư mục này, nên skill bạn thêm riêng cho project không bị xoá.

Các file còn lại là **framework**, danh sách nằm trong `.box/framework-files`. Lệnh `./dev update` sẽ ghi đè các file này.

## Dữ liệu được giữ lại

| Ở đâu | Gồm gì | Phạm vi |
|---|---|---|
| `DEVENV_HOME` (mặc định `~/.agent-devbox`) | `credentials/` (đăng nhập và config của claude/codex/gemini/gh), `agentmemory/`, `ssh/`, `.env` (`AGENTMEMORY_SECRET`) | Dùng chung mọi project |
| `.box/state/` | bash history | Theo project |
| Docker volume `<project>_cache`, `<project>_vscode-server` | cache npm/uv/nix, VS Code server | Theo project |
| Docker volume `agent-devbox-nix-cache` | Binary cache Nix cục bộ, giúp rebuild không phải tải lại | Dùng chung |

Config MCP của Codex và Antigravity là global, nên project nào chạy `connect` sau cùng thì `mcp.json` của project đó được dùng.

## Kiến trúc

```
Dockerfile          debian:trixie-slim + Nix (single-user); nix/base.nix cài lúc build
agents.Dockerfile   FROM base: agentmemory + iii engine, claude, codex, agy, context7
.box/compose.yml    service "dev": dùng chung cho ./dev và VS Code (devcontainer.json)
.box/memory.compose.yml   một agentmemory duy nhất (network agent-devbox, 127.0.0.1:3111/3113)
.box/runtime/       script trong container (/opt/box): entrypoint, box-connect, ...
```

## SSH cho editor khác (Antigravity, Cursor, Zed...)

Đặt `ENABLE_SSH=true` trong `.env` rồi chạy `./dev up`. sshd sẽ nghe ở `127.0.0.1:${SSH_PORT:-2222}` với user `dev`, key là `DEVENV_HOME/ssh/id_box`. Trên Windows, OpenSSH yêu cầu file key chỉ có bạn được đọc:

```powershell
icacls "$HOME\.agent-devbox\ssh\id_box" /inheritance:r /grant:r "${env:USERNAME}:R"
```

## Ghi chú

- Trên Windows, đọc ghi file qua bind mount khá chậm. Với project lớn, nên clone vào ổ của WSL2.
- `./dev rebuild` không khởi động lại memory server. Muốn memory server dùng image mới thì chạy `./dev memory restart`.
