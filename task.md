# Task: Rename opencode → pixicode & Rust Backend Migration

## Phase 1: Rename opencode → pixicode ✅
- [x] 1.1 Directory & file renames (`packages/opencode/` → `packages/pixicode/`, bin, config)
- [x] 1.2 Package identity (`package.json`, workspace refs `@opencode-ai/*` → `@pixicode-ai/*`)
- [x] 1.3 Source code strings (env vars, DB path, CLI name, flags, logs — ~3017 refs)
- [x] 1.4 Config & metadata (`.opencode/` → `.pixicode/`, Tauri `Cargo.toml`, `Cargo.lock`)
- [x] 1.5 Documentation (22 `README*.md`, `CONTRIBUTING.md`, `SECURITY.md`, `AGENTS.md`)
- [x] 1.6 Database backward compat (auto-copy `opencode.db` → `pixicode.db` + WAL/SHM)
- [x] 1.7 Verification — 0 remaining refs (ngoại trừ `@gitlab/opencode-gitlab-auth`)

---

## Phase 2: Rust Core Foundation
- [x] 2.1 **Cargo workspace setup**
  - [x] Root `Cargo.toml` workspace: `pixicode-core`, `pixicode-types`, `pixicode-tui`, `desktop/src-tauri`
  - [x] Shared dependencies, resolver = "2"
  - [x] `.cargo/config.toml`: dev/release profiles, target rustflags
- [x] 2.2 **CLI entry point** (`packages/pixicode-core/src/main.rs`)
  - [x] `clap` subcommands: `run`, `serve`, `auth`, `models`, `upgrade`, `export`, `import`, `ollama`
  - [x] Flags: `--print-logs`, `--log-level` (env `PIXICODE_LOG_LEVEL`), `--version`
  - [x] Signals: `setup_signals()` — SIGINT (ctrlc), SIGTERM/SIGHUP (signal-hook on Unix)
- [x] 2.3 **Config system** (`packages/pixicode-core/src/config/`)
  - [x] JSONC parser: `parse_jsonc`, `strip_jsonc_comments` trong `types.rs`
  - [x] Config struct: providers, models, theme, keybinds, agents, mcp, permission, etc.
  - [x] XDG: `ConfigPaths` — `$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`, `$XDG_CACHE_HOME` (+ `PIXICODE_*_HOME`)
  - [x] Env overrides: `PIXICODE_CONFIG`, `PIXICODE_CONFIG_CONTENT`, `apply_env_overrides()` (PIXICODE_MODEL, THEME, USERNAME, …)
- [x] 2.4 **Database layer** (`packages/pixicode-core/src/db/`)
  - [x] SQLite via `rusqlite` (Mutex guard, không pool)
  - [x] PRAGMA: WAL, busy_timeout, cache_size, foreign_keys, synchronous, temp_store
  - [x] Migration: embedded SQL trong `migrate.rs`, bảng `_migrations`
  - [x] Schema: `project`, `session`, `message`, `part`, `permission`, `todo`, `session_share`, `workspace`, `control_account` (session có cột summary_*)
  - [x] Wrapper: `with()`, `transaction()`
  - [x] Backward compat: copy `opencode.db` → `pixicode.db` nếu tồn tại
- [x] 2.5 **Logging** (`packages/pixicode-core/src/log/`)
  - [x] `tracing` + `tracing-subscriber`, `EnvFilter`
  - [x] File appender: `tracing_appender::rolling::hourly`, JSON format
  - [x] Log levels qua `PIXICODE_LOG_LEVEL` / `RUST_LOG`
  - [x] Structured JSON (file); optional pretty stdout khi `--print-logs`
- [x] 2.6 **HTTP Server** (`packages/pixicode-core/src/server/`)
  - [x] Axum + `tower::ServiceBuilder`, `TraceLayer`, `CorsLayer`
  - [x] CORS (PIXICODE_CORS_ORIGIN)
  - [x] Basic auth: `auth_middleware` (PIXICODE_SERVER_PASSWORD) — có code, chưa gắn vào router
  - [x] Request logging: `TraceLayer`; `request_logger` có code, chưa gắn
  - [x] 13 route groups: `/session`, `/config`, `/provider`, `/permission`, `/question`, `/global`, `/mcp`, `/tui`, `/workspace`, `/file`, `/auth`, `/instance/dispose`, `/path`|`/vcs`|`/command`
  - [x] OpenAPI stub: `/doc`
  - [x] SSE: `/events` (sse_handler) — real-time event bus
- [x] 2.7 **Event bus** (`packages/pixicode-core/src/bus/`)
  - [x] `tokio::broadcast`, `EventBus`, capacity 1024
  - [x] `BusEvent`: session/message/part/tool_call/config/instance
  - [x] SSE streaming endpoint `/events`

---

## Phase 3: Tool System (Rust)
- [x] 3.1 **Tool trait / registry**
  - [x] `Tool` trait: `name()`, `description()`, `schema()`, `execute()` (`trait_def.rs`)
  - [x] Registry: `ToolRegistry`, `with_builtins()`, `get()` / `execute()` (`registry.rs`)
  - [x] JSON Schema: `ToolSchema::to_json_value()`, `ToolParameter::to_json_value()`, `get_schemas_json()`
- [x] 3.2 **File tools**
  - [x] `read` — line range, max size (`file_read.rs`)
  - [x] `write` — tạo file mới (`file_write.rs`)
  - [x] `edit` — search & replace (`file_edit.rs`)
  - [x] `multiedit` — nhiều vị trí (`file_multiedit.rs`)
  - [x] `ls` — list directory (`file_ls.rs`)
  - [x] `glob` — glob pattern (`file_glob.rs`)
  - [x] `grep` — ripgrep (`file_grep.rs`)
  - [x] `codesearch` — code search (`file_codesearch.rs`)
- [x] 3.3 **Shell tools**
  - [x] `bash` — timeout, cwd, env (`shell_bash.rs`)
  - [ ] PTY cho interactive commands — chưa (dùng `Command::output`)
  - [x] Output truncation (`truncate_output`)
- [x] 3.4 **Web tools**
  - [x] `webfetch` — HTTP fetch, HTML → markdown (`web_fetch.rs`)
  - [x] `websearch` — web search (`web_search.rs`)
- [x] 3.5 **Advanced tools**
  - [x] `batch` — parallel execution (`tool_batch.rs`)
  - [x] `apply_patch` — unified diff (`tool_apply_patch.rs`)
  - [x] `task` — sub-agent delegation (`tool_task.rs`)
  - [x] `plan` — plan mode (`tool_plan.rs`)
  - [x] `question` — ask user (`tool_question.rs`)
  - [x] `todo` — todo list (`tool_todo.rs`)
  - [x] `skill` — skill loader (`tool_skill.rs`)
  - [x] `lsp` — LSP diagnostics/symbols/references (`tool_lsp.rs`)

---

## Phase 4: AI Provider Layer (Rust)
- [x] 4.1 **Provider trait** (đã có trong `pixicode-core/src/providers/`)
  - [x] `Provider` trait: `name()`, `models()`, `chat()`, `chat_stream()`
  - [x] Request/Response types: `ChatRequest`, `ChatResponse`, `ChatChunk`, messages, tools
  - [x] Token counting, cost calculation (`cost.rs`, `CostTracker`, `calculate_cost`)
  - [x] Rate limiting, retry logic (exponential backoff) — `retry.rs`, gắn vào OpenAI requests
- [x] 4.2 **HTTP-based providers** (dùng `reqwest`) — đa số đã implement
  - [x] Ollama API client
  - [x] OpenAI-compatible API client (`compatible.rs` base)
  - [x] Anthropic (Claude) — Messages API + streaming
  - [x] Google (Gemini) — Generative Language API
  - [x] AWS Bedrock — SigV4 auth, Converse API
  - [x] Azure OpenAI — AD auth, deployment-based routing
  - [x] Groq, Mistral, xAI, Cerebras, Cohere — OpenAI-compatible
  - [x] DeepInfra, TogetherAI, Perplexity — OpenAI-compatible
  - [x] OpenRouter — routing layer
  - [x] Google Vertex AI — skeleton `vertex.rs` (OAuth2 + regional; chat/models chưa implement)
- [x] 4.3 **Streaming support**
  - [x] SSE (Server-Sent Events) parser (`streaming.rs`, `SseStream`)
  - [x] Chunked response assembly
  - [x] Partial JSON parsing cho tool calls — `parse_openai_chunk` delta.tool_calls
  - [x] Stream cancellation — `stream_with_cancel`, `ChatRequest.cancel` (CancellationToken)
- [x] 4.4 **Auth management**
  - [x] API key storage: `MemoryStore`, `FileStore` (encrypted) trong `auth.rs`
  - [x] OAuth2: `OAuthStore`, `OAuthToken`; Azure OpenAI có OAuth2 token provider
  - [x] Keyring integration — `KeyringStore` (macOS Keychain, Windows Credential Manager, Linux secret-service); `pixicode auth set/list/remove` dùng keyring
  - [x] OAuth2 flows — `oauth_flows.rs`: Google (`google_auth_url`, `google_exchange_code`), Azure (`azure_auth_url`, `azure_exchange_code`); client ID/secret từ env (GOOGLE_*, AZURE_*)
  - [x] AWS credential chain — `load_aws_credentials()` (env + `~/.aws/credentials`)
  - [x] Custom auth headers — `ProviderConfig.headers`, OpenAI `extra_headers`

---

## Phase 5: Protocol & Advanced Features
- [x] 5.1 **Session management**
  - [x] Session CRUD — `SessionManager` + HTTP `/session` (create, list, get, delete)
  - [x] Message threading & history — `Session.messages`, `SessionStore` messages API
  - [x] Context window — `ContextManager`: `build_context()`, strategies (DropOldest, Summarize, RecentOnly, Smart), `ContextConfig`
  - [x] Token budget & compaction — `TokenBudget`, `budget_usage()`, `compact()`, `estimate_tokens`; chưa port ~67KB prompt.ts logic
  - [x] System prompt — `Session.system_prompt`, `set_system_prompt()`; assembly (agent + tools + context) chưa gộp một chỗ
- [x] 5.2 **Agent system**
  - [x] Agent definitions: `AgentType::Build` (full), `Plan` (read-only), `General`; `AgentConfig::build()`, `plan()`, `general()`
  - [x] Sub-agent: `General` + researcher/coder/reviewer trong `AgentRegistry::with_defaults()`
  - [x] Permission system — `PermissionChecker`, `AgentPermission` (Read/Write/Execute/Web/Tools/Full)
  - [x] Custom agent configs từ `.pixicode/agent/` — `AgentRegistry::load_from_dir(project_root)` đọc *.json / *.jsonc
- [x] 5.3 **MCP (Model Context Protocol)**
  - [x] MCP client — `McpClient`: `connect_stdio(cmd, args)`, `list_tools()`, `list_resources()`, `list_prompts()` (JSON-RPC qua stdio)
  - [x] MCP server — `McpServer`: `new(tools)`, `with_tool_handler()`, `run_stdio()` — xử lý initialize, tools/list, tools/call, resources/list, prompts/list
  - [x] Transport — `transport.rs`: `StdioClientTransport` (spawn process, request), `StdioServerTransport` (stdin/stdout, newline-delimited JSON)
  - [x] Tool/Resource/Prompt discovery — tools/list, resources/list, prompts/list qua JSON-RPC
- [x] 5.4 **LSP integration**
  - [x] JSON-RPC client cho language servers — `tools/lsp_client.rs`: `LspStdioClient` (spawn, Content-Length protocol, request/response)
  - [x] Diagnostics, symbols, references, completions, hover, format, rename — `tool_lsp` (LspAction); diagnostics dùng LSP khi server đã đăng ký
  - [x] Multi-language server management — `LSP_STATE`/`get_lsp_state()`, `register_lsp_server(lang, root_path, cmd, args)`, `LspServerEntry` + client per language
- [x] 5.5 **ACP (Agent Client Protocol)**
  - [x] ACP server — `acp/server.rs`: `AcpServer`, `create_task()`, `update_progress()` (in-memory)
  - [x] Task execution, progress — `AcpTask`, `TaskStatus`; chưa transport/HTTP
  - [x] Export trong lib — `pub mod acp;` `pub mod git;` đã thêm
- [x] 5.6 **Advanced features**
  - [x] Git worktree — `git/worktree.rs`: `WorktreeManager::list()`, `create(path, branch)`, `remove(path)` (git CLI)
  - [x] Snapshot & revert — `git/snapshot.rs`: `SnapshotManager::create(message)`, `revert(snapshot_id)`, `list()` (git stash)
  - [x] Session export/import — `db/session_io.rs`: `export_session`/`import_session`, CLI Export/Import dùng DB (session + message)
  - [x] Share sessions — `session_share` table, `share_url` trong session/DB
  - [x] Stats — `CostTracker`, `calculate_cost` (providers); usage trong session
  - [x] Plugin system — `plugin/mod.rs`: `PluginManager::new(list)`, `list()`, `run(name_or_path, args)` (spawn process)

---

## Phase 6: Desktop Integration
- [x] 6.1 **Unified Cargo workspace**
  - [x] Share `pixicode-core` crate với Tauri desktop app — `pixicode-core`, `pixicode-types` path deps trong `desktop/src-tauri/Cargo.toml`
  - [x] Desktop-specific commands via Tauri IPC — `core_bridge.rs`: `get_core_data_dir()`, `get_core_version_info()` (dùng `ConfigPaths`, `VersionInfo`, `pixicode_core::VERSION`)
  - [x] Shared types via `pixicode-types` crate — desktop trả về `CoreVersionInfo` (shape giống `VersionInfo`) và dùng `VersionInfo` từ types
- [x] 6.2 **TUI (Terminal UI)**
  - [x] Port TUI rendering — `pixicode-tui` dùng `ratatui` + crossterm (không dùng @opentui/solid; TUI Rust riêng)
  - [x] Keybindings, themes, input handling — `Theme` (dark/light), `ThemeName`, Ctrl+T đổi theme; `input.rs`: is_quit, is_clear_input, is_toggle_theme, is_switch_view, is_submit, is_escape; Help view liệt kê phím
- [x] 6.3 **Build & Distribution**
  - [x] Cross-compile targets — `.github/workflows/release.yml`: macOS (aarch64, x64), Linux (x64, arm64), Windows (x64)
  - [x] Release workflow — GitHub Actions on tag `v*`: create release, build matrix, upload assets, optional crates.io + Homebrew stub
  - [ ] Package managers — brew/scoop/choco/pacman/npm: release.yml có homebrew job (stub); chưa formula/manifest
  - [x] Install script — `scripts/install.sh`: curl | bash, PIXICODE_INSTALL_DIR/XDG_BIN_DIR, --version, --no-modify-path, tải artifact từ GitHub Release
  - [x] Auto-updater — desktop dùng `tauri-plugin-updater`

---

## Kế hoạch bắt đầu (Execution plan)

Mục tiêu: Web app có thể dùng Rust server thay Node, từng bước port logic.

### Milestone 0: Chuẩn bị (1–2 ngày)
- [x] **0.1** Default port Rust serve = `4096` (trùng Node) — `main.rs` `default_value = "4096"`.
- [x] **0.2** Script `dev:serve:rust` = `cargo run -p pixicode-core -- serve` — có trong `package.json`.
- [x] **0.3** Doc trong AGENTS: chạy `dev:serve:rust` + `dev:web` (hoặc `dev:web:full:rust`); so sánh response với Node.
- [x] **0.4** Schema DB Rust (`db/migrate.rs`) — có bảng session, message, project, part, ...; tương thích Drizzle.

### Milestone 1: Vertical slice 1 — Path + Global + Session list (3–5 ngày)
- [ ] **1.1** `/path`: Giữ implementation hiện tại (đã trả `cwd`, `home`).
- [ ] **1.2** `/global`: Đọc từ DB hoặc file global state (port từ Node `GlobalRoutes`), trả JSON đúng format app mong đợi.
- [ ] **1.3** `/session` GET (list): Query DB bảng `session` (+ `project` nếu cần), trả danh sách session đúng schema (id, title, directory, time_created, time_updated). Không cần filter theo directory ngay.
- [ ] **1.4** So sánh response với Node (cùng DB file): list session giống nhau khi chạy Rust vs Node.
- [ ] **1.5** (Tùy chọn) Bật `dev:serve:rust` làm backend cho web: chạy Rust thay Node, mở web app, kiểm tra trang home / danh sách project/session.

### Milestone 2: Session CRUD đầy đủ (5–7 ngày)
- [ ] **2.1** `POST /session`: Tạo session mới (insert `session` + `project` nếu cần), trả session object.
- [ ] **2.2** `GET /session/:id`: Lấy chi tiết session (+ messages nếu app cần).
- [ ] **2.3** `GET /session/:id/messages`: List messages từ bảng `message` (và `part` nếu có).
- [ ] **2.4** `POST /session/:id/message`: Append message (insert `message` + `part`), trả message/part.
- [ ] **2.5** `DELETE /session/:id`: Xóa session (cascade).
- [ ] **2.6** Đồng bộ format request/response với Node (so sánh với `SessionRoutes` trong `packages/pixicode/src/server/routes/session.ts`).

### Milestone 3: Config + Provider (3–5 ngày)
- [ ] **3.1** `GET /config`, `PUT /config`: Đọc/ghi config từ file (JSONC), format tương thích Node.
- [ ] **3.2** `GET /provider`, `GET /provider/:id/models`: Trả danh sách provider và models (từ config + models.dev hoặc cache); có thể stub models trước.
- [ ] **3.3** App có thể chọn model từ dialog khi backend là Rust.

### Milestone 4: Chuyển mặc định sang Rust (2–3 ngày)
- [ ] **4.1** `dev:serve` chạy Rust thay Node: ví dụ `dev:serve` = `cargo run -p pixicode-core -- serve`, giữ `dev:serve:node` = `bun run --cwd packages/pixicode src/index.ts serve` cho fallback.
- [ ] **4.2** CI: build `pixicode-core` và chạy test (nếu có) cho Rust.
- [ ] **4.3** Doc: "Backend mặc định là Rust; dùng `dev:serve:node` nếu cần Node."

### Thứ tự ưu tiên sau Milestone 4
1. **Permission** (`/permission`): Cần cho agent chạy tool.
2. **Question** (`/question`): Hỏi user trong session.
3. **File** (`/file`): Đọc/ghi file, list dir — port từ Node FileRoutes.
4. **Auth** (`/auth`): Set/remove credentials — tích hợp keyring hoặc file.
5. **MCP** (`/mcp`): List/add/remove MCP servers (config); MCP client thực thi có thể để sau.
6. **SSE /events**: Event bus để app subscribe; đồng bộ với Node BusEvent.

### Ghi chú
- Mỗi milestone nên kết thúc bằng "web app (hoặc curl) gọi Rust server và nhận response đúng".
- Giữ cùng một file DB (`pixicode.db`) cho cả Node và Rust trong giai đoạn chuyển đổi; tránh đổi schema Rust khi Node vẫn ghi.
- Khi một route đã port xong và test ổn, đánh dấu trong task Phase 2.6 tương ứng.

---

## Công việc còn lại (Remaining)

- **Phase 3**: PTY cho shell interactive (tùy chọn).
- **Phase 5.3 MCP**: Implement MCP client (connect, list_tools/resources/prompts), server (expose tools), transport (stdio/SSE/WebSocket).
- **Phase 5.4 LSP**: JSON-RPC client tới language servers thật; multi-language server management.
- **Phase 5.6**: Git worktree/snapshot (implement thay stub); Session export/import (DB + serialise); Plugin system (runtime load).
- **Phase 6**: Desktop integration (Tauri IPC, TUI, build/distribution).
- **Milestone 0.3**: Thêm doc README/AGENTS cho `dev:serve:rust` + `dev:web`.
- **Milestone 1–4**: Vertical slice path/global/session → session CRUD → config/provider → chuyển mặc định sang Rust.
