# Kế hoạch chuyển đổi Backend TypeScript → Rust

> **Mục tiêu**: Chuyển toàn bộ backend PixiCode từ TypeScript (Hono + Bun) sang Rust (Axum + Tokio)
> **Lý do**: Tối ưu hiệu năng, giảm memory footprint, tận dụng type safety mạnh hơn, zero-cost abstractions
> **Model hiện tại**: `edu-assistant:latest` (single model, không speculative decoding)

---

## 1. Bảng so sánh API: TypeScript vs Rust

### 1.1 Session API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /session` | ✅ Full (filter, search, limit) | ✅ Full CRUD | Rust đã có list, filter cơ bản |
| `GET /session/:id` | ✅ | ✅ | Parity |
| `POST /session` | ✅ | ✅ | Parity |
| `DELETE /session/:id` | ✅ | ✅ | Parity |
| `PATCH /session/:id` | ✅ (title, archive) | ✅ Full | ✅ GĐ1 — title + archive toggle |
| `GET /session/:id/message` | ✅ | ✅ | Parity |
| `POST /session/:id/message` | ✅ LLM streaming + tools | ⚠️ Prompt loop | ✅ GĐ1 — prompt.rs + permission gate |
| `POST /session/:id/prompt_async` | ✅ | ✅ Full | ✅ GĐ1 — run_prompt + permission gate |
| `POST /session/:id/abort` | ✅ | ✅ Basic | Rust publish event, thiếu cancel logic |
| `POST /session/:id/fork` | ✅ | ❌ | Fork session tại message N |
| `POST /session/:id/share` | ✅ | ❌ | Chia sẻ session |
| `POST /session/:id/summarize` | ✅ AI compaction | ❌ | Context compaction bằng AI |
| `POST /session/:id/revert` | ✅ File snapshot restore | ❌ | Undo message + khôi phục file |
| `POST /session/:id/unrevert` | ✅ | ❌ | Re-apply reverted messages |
| `POST /session/:id/init` | ✅ Project analysis | ✅ Basic | ✅ GĐ1 — project type detection |
| `POST /session/:id/command` | ✅ | ❌ | Execute command trong session |
| `POST /session/:id/shell` | ✅ | ❌ | Shell command trong session |
| `GET /session/:id/todo` | ✅ | ❌ | Todo list từ session |
| `GET /session/:id/children` | ✅ | ❌ | Forked sessions |
| `GET /session/:id/diff` | ✅ | ❌ | File diffs từ message |
| `GET /session/status` | ✅ Realtime | ✅ Full | ✅ GĐ1 — StatusTracker |

### 1.2 Project API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /project` | ✅ | ✅ | Parity |
| `GET /project/current` | ✅ | ✅ | Parity |
| `PATCH /project/:id` | ✅ | ✅ | Parity |

### 1.3 File API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /file` | ✅ Directory listing | ✅ Basic | Rust thiếu advanced filters |
| `GET /file/content` | ✅ | ✅ | Parity |
| `POST /file/content` | ✅ | ✅ | Parity |
| `GET /find` | ✅ Ripgrep integration | ❌ | Text search |
| `GET /find/file` | ✅ Glob patterns | ❌ | File search |
| `GET /find/symbol` | ✅ LSP symbols | ❌ | Workspace symbols |

### 1.4 PTY/Terminal API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /pty` | ✅ | ❌ | List PTY sessions |
| `GET /pty/:id` | ✅ | ❌ | PTY info |
| `POST /pty` | ✅ | ❌ | Create PTY |
| `PUT /pty/:id` | ✅ (resize, title) | ❌ | Update PTY |
| `DELETE /pty/:id` | ✅ | ❌ | Terminate PTY |
| `GET /pty/:id/connect` | ✅ WebSocket | ❌ | **WebSocket streaming I/O** |

### 1.5 Config API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /config` | ✅ Multi-level merge | ⚠️ Read-only | Rust đọc config, chưa merge levels |
| `GET /config/providers` | ✅ With defaults | ⚠️ Basic | Thiếu default enrichment |
| `PATCH /config` | ✅ JSONC edit | ✅ Basic | ✅ GĐ1 — patch_project_config |

### 1.6 Permission API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /permission` | ✅ | ✅ | Parity |
| `POST /permission/:id/reply` | ✅ | ✅ | Parity |
| `POST /permission/grant` | ✅ | ✅ | Parity |
| `DELETE /permission/revoke` | ✅ | ✅ | Parity |

### 1.7 Provider API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /provider` | ✅ All + connected | ✅ Basic | Rust thiếu connected status |
| `GET /provider/auth` | ✅ Auth methods | ❌ | OAuth info |
| `POST /provider/:id/oauth/*` | ✅ OAuth flow | ❌ | OAuth authorization |
| `GET /provider/:id/models` | ✅ | ✅ | Parity |

### 1.8 Question API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /question` | ✅ | ✅ | Parity |
| `POST /question/:id/reply` | ✅ | ✅ | Parity |
| `POST /question/:id/reject` | ✅ | ✅ | ✅ GĐ1 — Reject question |

### 1.9 Workspace API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /workspace` | ✅ | ✅ | Parity |
| `POST /workspace` | ✅ | ✅ | Parity |
| `DELETE /workspace/:id` | ✅ | ✅ | Parity |

### 1.10 VCS/Git API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /vcs` | ✅ Branch, dirty, root | ✅ | Parity |

### 1.11 MCP API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /mcp` | ✅ Server status | ⚠️ Stub | Chỉ đọc config |
| `POST /mcp` | ✅ Add server | ❌ | Add MCP server |
| `POST /mcp/:name/auth` | ✅ OAuth | ❌ | MCP OAuth |
| MCP tool translation | ✅ Full | ❌ | Translate MCP tools → AI SDK |

### 1.12 Auth API

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `PUT /auth/:provider` | ✅ | ⚠️ Stub | Set credentials |
| `DELETE /auth/:provider` | ✅ | ⚠️ Stub | Remove credentials |

### 1.13 Other APIs

| Endpoint | TypeScript | Rust | Ghi chú |
|----------|-----------|------|---------|
| `GET /event` | ✅ SSE streaming | ✅ SSE | Parity |
| `GET /path` | ✅ | ✅ | Parity |
| `GET /command` | ✅ Full list | ❌ Stub | Command discovery |
| `GET /agent` | ✅ | ❌ | Agent listing |
| `GET /skill` | ✅ | ❌ | Skill listing |
| `GET /lsp` | ✅ LSP status | ❌ | LSP server management |
| `GET /formatter` | ✅ | ❌ | Formatter status |
| `POST /log` | ✅ | ❌ | Client logging |
| `POST /instance/dispose` | ✅ | ✅ | Parity |
| `GET /global/health` | ✅ | ✅ | ✅ GĐ1 — Health check |

### 1.14 Tổng hợp parity

| Nhóm | Tổng endpoints | Rust ✅ | Rust ⚠️ | Rust ❌ | Parity % |
|------|---------------|---------|---------|---------|----------|
| Session | 21 | 10 | 1 | 10 | 48% |
| Project | 3 | 3 | 0 | 0 | 100% |
| File | 6 | 3 | 0 | 3 | 50% |
| PTY | 6 | 0 | 0 | 6 | 0% |
| Config | 3 | 1 | 2 | 0 | 33% |
| Permission | 4 | 4 | 0 | 0 | 100% |
| Provider | 4 | 1 | 0 | 3 | 25% |
| Question | 3 | 3 | 0 | 0 | 100% |
| Workspace | 3 | 3 | 0 | 0 | 100% |
| VCS | 1 | 1 | 0 | 0 | 100% |
| MCP | 4 | 0 | 1 | 3 | 0% |
| Auth | 2 | 0 | 2 | 0 | 0% |
| Other | 10 | 4 | 0 | 6 | 40% |
| **Tổng** | **70** | **33** | **6** | **31** | **47%** |

---

## 2. Hiện trạng Rust codebase

### 2.1 Đã hoàn thiện (Production-ready)
- **HTTP Server**: Axum + Tower middleware + graceful shutdown
- **Database**: SQLite (rusqlite) + migrations + typed models
- **Event Bus**: tokio::broadcast channel (11 event types)
- **Tool System**: 26+ tools (file, shell, web, LSP, plan, batch...)
- **Provider System**: 11+ providers (OpenAI, Anthropic, Google, Ollama, Azure, Bedrock, Vertex, Compatible...)
- **Session Store**: CRUD + context window management
- **Git Integration**: Worktree + repository + snapshot
- **Agent System**: Agent definitions + permissions
- **MCP Client**: Stdio transport + tool translation
- **CLI**: 8 commands (serve, ollama, auth, export, import...)
- **Config**: Multi-level loading, serde deserialization

### 2.2 Còn thiếu (Critical gaps)

| Component | Mô tả | Độ khó | Ưu tiên |
|-----------|--------|--------|---------|
| **Prompt Loop** | LLM streaming + tool execution cycle | 🔴 Cao | P0 |
| **PTY Management** | Terminal session + WebSocket streaming | 🔴 Cao | P0 |
| **Config Write** | JSONC edit + multi-level merge | 🟡 TB | P1 |
| **Session Fork/Revert** | Fork tại message, undo với file snapshot | 🟡 TB | P1 |
| **Session Compaction** | AI summarization để giảm context | 🟡 TB | P1 |
| **File Search** | Ripgrep integration (find text, find file) | 🟢 Thấp | P1 |
| **MCP Server Lifecycle** | Start/stop/reconnect MCP servers | 🟡 TB | P2 |
| **OAuth Flows** | Provider OAuth authorization | 🟡 TB | P2 |
| **LSP Server Mgmt** | Start/manage language servers | 🟡 TB | P2 |
| **TUI** | Terminal UI (React/Ink equivalent) | 🔴 Cao | P2 |
| **Plugin System** | Plugin loading + execution | 🟡 TB | P3 |
| **Session Share** | Share link generation | 🟢 Thấp | P3 |

---

## 3. Ba giai đoạn chuyển đổi

### Giai đoạn 1: Đạt parity luồng chính (8-10 tuần)

**Mục tiêu**: Rust backend có thể xử lý luồng chính: user gửi message → LLM response + tool execution → kết quả trả về client.

#### 1.1 Prompt Loop Engine (3-4 tuần) — P0
Đây là phần **quan trọng nhất**, là trái tim của hệ thống.

**Cần implement:**
```
POST /session/:id/message
  → Build system prompt (instructions, context, tools)
  → Stream to LLM provider (edu-assistant:latest via Ollama)
  → Parse tool calls from response
  → Execute tools (permission gate)
  → Loop until LLM finishes or max iterations
  → Store messages + parts to DB
  → Publish bus events
  → Stream SSE to client
```

**Files cần tạo/sửa:**
```
src/session/
  ├── prompt.rs        (NEW) — Main prompt/completion loop
  ├── llm.rs           (NEW) — LLM streaming adapter
  ├── system.rs        (NEW) — System prompt builder
  ├── instruction.rs   (NEW) — Instruction management
  └── processor.rs     (NEW) — Message processing pipeline

src/server/routes/
  └── session.rs       (MODIFY) — Wire up streaming endpoint
```

**Luồng chi tiết:**
1. Nhận user message → tạo Message record trong DB
2. Build prompt: system instructions + conversation history + available tools
3. Gọi `provider.chat_stream()` với prompt
4. Parse streaming response:
   - Text delta → emit SSE `text-delta`
   - Tool call → execute tool → emit SSE `tool-result`
   - Finish → emit SSE `finish`
5. Nếu có tool calls → append tool results → loop lại bước 3
6. Store final message + parts vào DB
7. Publish `MessageCreated`, `PartCreated` events

**Dependencies đã có:**
- ✅ Provider streaming (`providers/streaming.rs`)
- ✅ Tool execution (`tools/registry.rs` + 26 tools)
- ✅ DB storage (`db/mod.rs`)
- ✅ Bus events (`bus/mod.rs`)
- ✅ SSE streaming (`server/sse.rs`)

#### 1.2 Workspace/Instance Middleware (1 tuần)
```
src/server/
  └── middleware.rs    (MODIFY) — Enhance workspace context

src/project/
  ├── instance.rs     (NEW) — Per-directory instance state
  └── bootstrap.rs    (NEW) — Project initialization
```

**Cần làm:**
- Instance state per working directory (cwd detection)
- Project auto-detection (tìm .git, package.json, Cargo.toml)
- Bootstrap logic khi client connect lần đầu
- Workspace routing (multi-workspace support)

#### 1.3 Config Write + Multi-level Merge (1 tuần)
```
src/config/
  ├── merge.rs        (NEW) — System → User → Project config merge
  ├── writer.rs       (NEW) — JSONC writer (preserve comments)
  └── types.rs        (MODIFY) — Add defaults, validation
```

**Cần làm:**
- Merge 3 cấp config: system managed → user global → project local
- JSONC parser/writer (preserve comments khi edit)
- `PATCH /config` endpoint ghi file
- Config file watcher (auto-reload khi thay đổi)

#### 1.4 Permission Gate Integration (1 tuần)
```
src/permission/
  ├── gate.rs         (NEW) — Execution gates
  ├── arity.rs        (NEW) — Permission arity definitions
  └── checker.rs      (NEW) — Permission checking logic
```

**Cần làm:**
- Trước khi execute tool → check permission
- Nếu cần approval → push vào PendingPermissions → chờ user reply
- Auto-approve nếu tool đã được grant
- Permission arity: once, session, project, always

#### 1.5 Session Status Tracking (0.5 tuần)
```
src/session/
  └── status.rs       (NEW) — Realtime status tracking
```

**Cần làm:**
- Track session state: idle, streaming, tool_executing, waiting_permission
- `GET /session/status` endpoint
- Broadcast status changes qua bus

#### 1.6 Testing & Integration (1-2 tuần)
- Unit tests cho prompt loop
- Integration tests: full flow từ HTTP request → LLM → tool → response
- Load test: concurrent sessions
- Verify frontend (TUI) hoạt động với Rust backend

**Deliverable Giai đoạn 1:**
> Rust backend có thể thay thế TS backend cho luồng chính: user chat → LLM + tools → response. Frontend TUI kết nối vào Rust server và hoạt động bình thường.

---

### Giai đoạn 2: Feature parity mở rộng (6-8 tuần)

**Mục tiêu**: Migrate các tính năng quan trọng còn lại để Rust backend đủ feature cho daily use.

#### 2.1 PTY/Terminal Management (2-3 tuần) — P0

**Cần implement:**
```
src/pty/
  ├── mod.rs          (NEW) — PTY manager
  ├── session.rs      (NEW) — PTY session lifecycle
  ├── buffer.rs       (NEW) — Output ring buffer
  └── websocket.rs    (NEW) — WebSocket handler

src/server/routes/
  └── pty.rs          (MODIFY) — Full PTY routes
```

**Dependencies Rust:**
- `portable-pty` hoặc `pty-process` crate cho PTY allocation
- `axum::extract::ws::WebSocket` cho WebSocket
- `tokio::io` cho async I/O

**Tính năng:**
- Create/destroy PTY sessions
- WebSocket bidirectional streaming (stdin/stdout)
- Output buffer với cursor position tracking
- Shell detection (bash, zsh, fish)
- Terminal resize (SIGWINCH)
- Process lifecycle management

#### 2.2 Session Advanced Features (2 tuần)

**Fork:**
```
src/session/
  └── fork.rs         (NEW) — Fork session tại message N
```
- Copy session + messages up to point N
- Create new session with parent_id reference
- Deep copy file snapshots

**Revert:**
```
src/session/
  └── revert.rs       (NEW) — Undo message effects
```
- Snapshot files before tool execution
- Revert: restore file snapshots, mark messages as reverted
- Unrevert: re-apply changes

**Compaction:**
```
src/session/
  └── compaction.rs   (NEW) — AI-powered context summarization
```
- Khi context quá dài → gọi LLM summarize
- Replace old messages với summary
- Preserve key information (file changes, decisions)

#### 2.3 File Search Integration (1 tuần)
```
src/file/
  ├── ripgrep.rs      (NEW) — Ripgrep subprocess wrapper
  ├── glob.rs         (NEW) — Glob pattern search
  └── watcher.rs      (NEW) — File system watcher
```

**Endpoints:**
- `GET /find` — Text search via ripgrep subprocess
- `GET /find/file` — File name search via glob
- `GET /find/symbol` — LSP workspace symbols

#### 2.4 MCP Server Lifecycle (1-2 tuần)
```
src/mcp/
  ├── manager.rs      (NEW) — MCP server lifecycle manager
  ├── tool_bridge.rs  (NEW) — MCP tool → internal tool translation
  └── oauth.rs        (NEW) — MCP OAuth flows
```

**Cần làm:**
- Start/stop MCP servers (stdio transport)
- Translate MCP tools thành internal tool format
- Health check + auto-restart
- OAuth flow cho MCP servers cần auth

#### 2.5 Auth + OAuth Flows (1 tuần)
```
src/providers/
  └── auth.rs         (MODIFY) — Full credential management

src/server/routes/
  ├── auth.rs         (MODIFY) — Real credential CRUD
  └── provider.rs     (MODIFY) — OAuth endpoints
```

**Cần làm:**
- Keyring integration (đã có `keyring` crate)
- OAuth2 authorization code flow
- Token refresh
- Credential validation

**Deliverable Giai đoạn 2:**
> Rust backend hỗ trợ PTY terminals, session fork/revert/compaction, file search, MCP servers, và full authentication. Daily use workflow hoàn chỉnh.

---

### Giai đoạn 3: Full parity + Tối ưu (4-6 tuần)

**Mục tiêu**: Đạt 100% feature parity và tối ưu performance.

#### 3.1 TUI (Terminal UI) (2-3 tuần)

**Lựa chọn:**
- **Option A**: Giữ React/Ink TUI trong TS, chạy như process riêng gọi Rust HTTP API
- **Option B**: Viết TUI mới bằng `ratatui` + `crossterm`

**Đề xuất: Option A** (ngắn hạn) → **Option B** (dài hạn)
- Giai đoạn 3 ban đầu: giữ TS TUI, chỉ thay backend
- Sau đó: migrate TUI sang ratatui nếu muốn single binary

#### 3.2 Advanced Features (1-2 tuần)

| Feature | File | Mô tả |
|---------|------|--------|
| Session Share | `session/share.rs` | Generate share links |
| Command System | `command/mod.rs` | Command discovery + execution |
| Skill System | `skill/mod.rs` | Skill discovery + execution |
| Agent System | `agent/execution.rs` | Agent execution (beyond definitions) |
| Plugin System | `plugin/loader.rs` | Dynamic plugin loading |
| Formatter | `format/mod.rs` | LSP-based code formatting |
| Session Todo | `session/todo.rs` | Todo extraction from AI responses |

#### 3.3 Performance Optimization (1-2 tuần)

| Optimization | Mô tả | Impact |
|-------------|--------|--------|
| **Connection pooling** | Pool Ollama connections thay vì per-request | Giảm latency ~20% |
| **Zero-copy streaming** | Bytes streaming thay vì String allocation | Giảm memory ~30% |
| **Parallel tool execution** | Tokio spawn cho independent tools | Giảm wall time |
| **SQLite WAL tuning** | Optimize PRAGMA cho workload pattern | Giảm DB latency |
| **Binary size** | LTO + strip + codegen-units=1 | Giảm binary size ~40% |
| **Memory profiling** | Jemalloc + DHAT profiling | Identify hotspots |

#### 3.4 Testing & Hardening (1 tuần)

- Property-based testing (proptest)
- Fuzzing cho parsers (cargo-fuzz)
- Benchmark suite (criterion)
- CI/CD pipeline
- Cross-platform testing (macOS, Linux)

**Deliverable Giai đoạn 3:**
> Full feature parity. TS backend có thể được retire. Single Rust binary phục vụ toàn bộ backend.

---

## 4. Chiến lược chuyển đổi

### 4.1 Parallel Running (Giai đoạn 1-2)

```
┌─────────────────┐
│   TUI (React)   │
│   Frontend      │
└────────┬────────┘
         │ HTTP
    ┌────▼────┐
    │  Proxy  │  ← Route dựa trên feature readiness
    ├─────────┤
    │         │
┌───▼───┐ ┌──▼────┐
│ Rust  │ │  TS   │  ← Chạy song song
│ :3001 │ │ :3000 │
└───────┘ └───────┘
```

- Ban đầu: proxy route sang TS cho features chưa có trong Rust
- Dần dần chuyển routes sang Rust khi implement xong
- Cuối cùng: tắt TS server

### 4.2 Feature Flags

```rust
// config
[migration]
use_rust_session = true     # Giai đoạn 1
use_rust_pty = false        # Chờ Giai đoạn 2
use_rust_mcp = false        # Chờ Giai đoạn 2
```

### 4.3 Database Compatibility

- Giữ nguyên SQLite schema — cả TS và Rust dùng chung DB
- Rust đã có migration system tương thích
- Rollback: nếu Rust lỗi → quay lại TS ngay lập tức

---

## 5. Ước lượng timeline

```
Tuần 1-2    ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  Prompt Loop (core)
Tuần 3-4    ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  Prompt Loop (tools + streaming)
Tuần 5      ┃▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░┃  Workspace/Instance + Config Write
Tuần 6      ┃▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░┃  Permission Gate + Status
Tuần 7-8    ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  Testing + Integration (GĐ1 done)
───────────────────────────────────────────────────────
Tuần 9-11   ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  PTY/Terminal
Tuần 12-13  ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  Session Fork/Revert/Compaction
Tuần 14     ┃▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░┃  File Search + MCP
Tuần 15-16  ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  Auth/OAuth + Testing (GĐ2 done)
───────────────────────────────────────────────────────
Tuần 17-19  ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  TUI + Advanced Features
Tuần 20-21  ┃▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┃  Performance + Hardening
Tuần 22     ┃▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░┃  Final testing + TS retirement
```

**Tổng: ~22 tuần (5.5 tháng)**

---

## 6. Rủi ro và giải pháp

| Rủi ro | Xác suất | Giải pháp |
|--------|----------|-----------|
| Prompt loop phức tạp hơn dự kiến | Cao | Prototype sớm, test với edu-assistant |
| PTY cross-platform issues | TB | Dùng `portable-pty` đã battle-tested |
| JSONC parsing trong Rust | Thấp | Dùng `jsonc-parser` crate |
| TUI parity với React/Ink | Cao | Giữ TS TUI ban đầu (Option A) |
| LLM streaming edge cases | TB | Port test cases từ TS, fuzzing |
| Performance regression | Thấp | Benchmark từ đầu, so sánh A/B |

---

## 7. Metrics đo lường thành công

| Metric | TS Baseline | Target Rust | Cách đo |
|--------|------------|-------------|---------|
| TTFT (Time to First Token) | ~800ms | <400ms | Benchmark tool |
| Memory idle | ~180MB | <50MB | `ps aux` RSS |
| Memory active (5 sessions) | ~400MB | <120MB | Load test |
| Binary size | ~90MB (node_modules) | <30MB | `ls -la` |
| Cold start | ~2s | <200ms | Timer |
| Concurrent sessions | ~20 | ~100 | Load test |
| Tool execution overhead | ~15ms | <3ms | Instrumented |
| SQLite query latency | ~5ms | <1ms | Tracing |

---

## 8. Thứ tự ưu tiên implement (Giai đoạn 1)

```
Priority 0 (MUST):
  ┌─────────────────────────────────────────────┐
  │  1. prompt.rs     — LLM streaming loop      │
  │  2. llm.rs        — Provider adapter         │
  │  3. system.rs     — System prompt builder    │
  │  4. processor.rs  — Message pipeline         │
  │  5. session.rs    — Wire up POST /message    │
  └─────────────────────────────────────────────┘

Priority 1 (SHOULD):
  ┌─────────────────────────────────────────────┐
  │  6. gate.rs       — Permission gates         │
  │  7. instance.rs   — Per-dir instance state   │
  │  8. merge.rs      — Config multi-level merge │
  │  9. status.rs     — Session status tracking  │
  │ 10. writer.rs     — Config JSONC writer      │
  └─────────────────────────────────────────────┘

Priority 2 (NICE):
  ┌─────────────────────────────────────────────┐
  │ 11. ripgrep.rs    — Text search              │
  │ 12. glob.rs       — File search              │
  │ 13. question reject — Reject question        │
  │ 14. health check  — GET /global/health       │
  └─────────────────────────────────────────────┘
```

---

## Kết luận

Rust codebase đã có **47% API parity** (tăng từ 36%) với nền tảng vững chắc: HTTP server, database, 26+ tools, 11+ providers, event bus. Prompt Loop Engine đã được implement.

Với chiến lược parallel running (chạy song song TS + Rust), việc chuyển đổi có thể thực hiện **incremental** mà không ảnh hưởng production. Mỗi giai đoạn có deliverable rõ ràng và có thể rollback nếu cần.

---

## 9. Tiến độ thực hiện

### GĐ1 — Đã hoàn thành

| Task | File(s) | Status |
|------|---------|--------|
| **1.1 Prompt Loop Engine** | `session/prompt.rs`, `session/system.rs` | ✅ Done |
| **1.2 Workspace/Instance** | `server/middleware.rs` (ProjectType detection) | ✅ Partial |
| **1.3 Config Write** | `config/types.rs` (patch_project_config), `routes/config.rs` (PATCH) | ✅ Done |
| **1.4 Permission Gate** | `session/permission_gate.rs` + tích hợp vào prompt loop | ✅ Done |
| **1.5 Session Status** | `session/status.rs`, `GET /session/status` route | ✅ Done |
| **PATCH /session/:id** | `routes/session.rs` (title + archive toggle) | ✅ Done |
| **POST /session/:id/init** | `routes/session.rs` (project type detection) | ✅ Done |
| **POST /question/:id/reject** | `routes/question.rs` | ✅ Done |
| **GET /global/health** | `routes/global.rs` | ✅ Done |
| **StatusTracker in AppState** | `server/state.rs` | ✅ Done |
| **ToolRegistry in AppState** | `server/state.rs` | ✅ Done |

### Files mới tạo trong GĐ1

```
src/session/
  ├── prompt.rs            — Prompt Loop Engine (streaming + tool execution + persistence)
  ├── permission_gate.rs   — Permission Gate (safe tools, session/project grants, user approval)
  ├── status.rs            — Real-time session status tracking
  └── system.rs            — System prompt builder
```

### Thống kê API parity sau GĐ1

- **Trước GĐ1**: 25/70 endpoints (36%)
- **Sau GĐ1**: 33/70 endpoints (47%)
- **Tăng**: +8 endpoints, +11% parity

### Bước tiếp theo: GĐ1.6 Testing & Integration

1. Unit tests cho prompt loop
2. Integration tests: full flow HTTP → LLM → tools → response
3. Load test: concurrent sessions
4. Verify frontend TUI hoạt động với Rust backend
