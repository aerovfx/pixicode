# Kế hoạch chuyển backend toàn bộ sang Rust

## Hiện trạng

- **Backend chính (hiện tại):** `packages/pixicode` — TypeScript/Bun, framework Hono, chạy qua `bun run dev` / `pixicode serve` (bin từ package pixicode).
- **Backend Rust (đã có):** `packages/pixicode-core` — Rust, Axum, Tower, chạy qua `cargo run -p pixicode-core -- serve` (port 4096).

`pixicode-core` đã có: server HTTP (Axum), session (store, routes), config, provider, permission, question, global, MCP, TUI (theme, keybinds), workspace, file, auth, instance, path, vcs, command, SSE `/events`, OpenAPI stub `/doc`, DB (rusqlite), tools (read, edit, grep, glob, bash, task, webfetch, websearch, …), providers (Ollama, OpenAI, Anthropic, …), agent, ACP.

## Mục tiêu

Một backend duy nhất bằng **Rust** (`pixicode-core`): mọi API và luồng xử lý do Rust phục vụ; app/frontend và CLI (nếu cần) vẫn có thể gọi API giống hiện tại. Backend TS có thể giữ chỉ để dev/fallback hoặc gỡ bỏ sau khi đạt parity.

## Phạm vi cần đồng bộ / chuyển

### Bảng so sánh từng nhóm API

| Nhóm | TS (packages/pixicode) | Rust (pixicode-core) | Trạng thái |
|------|------------------------|----------------------|------------|
| Session | SessionRoutes (list, create, get, delete, messages, prompt, abort, fork, share, …) | session::* | **Partial** — xem bảng endpoint bên dưới |
| Project | ProjectRoutes | workspace::* | **Khác contract** — project list/current/patch vs workspace list/create/get/delete |
| PTY | PtyRoutes | — | **Missing** |
| Config | ConfigRoutes | config::* | **Partial** — TS có GET /config/providers |
| Experimental | ExperimentalRoutes (workspace, …) | — | **Missing** |
| Permission | PermissionRoutes | permission::* | **Khác contract** — TS: reply by requestID; Rust: grant/revoke by tool |
| Question | QuestionRoutes | question::* | **Có** |
| Provider | ProviderRoutes | provider::* | **Partial** — TS có thêm post endpoints |
| File | FileRoutes | file::* | **Partial** — so sánh query params và variant |
| MCP | McpRoutes | mcp::* | **Có** (list, add, remove, enable, disable) |
| TUI | TuiRoutes (theme, keybinds, control) | tui::* | **Partial** — thiếu control sub-routes |
| Global | GlobalRoutes | global::* | **Partial** — TS có patch, post; Rust get/put |
| Auth | PUT/DELETE /auth/:providerID | auth::* | **Có** (post/delete /auth/:provider) |
| Instance | POST /instance/dispose | instance::* | **Có** |
| Workspace middleware | WorkspaceContext + Instance.provide | — | **Missing** — Rust chưa inject workspace/directory |
| Workspace router | WorkspaceRouterMiddleware | — | **Missing** |
| OpenAPI | hono-openapi, /doc | openapi_doc stub | **Partial** — stub only |
| WebSocket | Hono/Bun websocket | — | **Missing** |
| SSE | streamSSE (nếu có) | /events | **Có** |

### Bảng so sánh endpoint Session (TS vs Rust)

Prefix TS: `/session` (mount). Prefix Rust: `/session`.

| Method | Path (TS) | TS | Rust | Ghi chú |
|--------|-----------|----|------|--------|
| GET | / | list (query: directory, roots, start, search, limit) | list | **Có** — Rust có thể thiếu query params |
| GET | /status | session status map | — | **Missing** |
| GET | /:sessionID | get session | get | **Có** |
| GET | /:sessionID/children | list children | — | **Missing** |
| GET | /:sessionID/todo | get todo | — | **Missing** |
| POST | / | create | create | **Có** |
| DELETE | /:sessionID | delete | delete_session | **Có** |
| PATCH | /:sessionID | update session | — | **Missing** |
| POST | /:sessionID/init | init | — | **Missing** |
| POST | /:sessionID/fork | fork | — | **Missing** |
| POST | /:sessionID/abort | abort | abort | **Có** |
| POST | /:sessionID/share | share | — | **Missing** |
| GET | /:sessionID/diff | diff | — | **Missing** |
| DELETE | /:sessionID/share | unshare | — | **Missing** |
| POST | /:sessionID/summarize | summarize | — | **Missing** |
| GET | /:sessionID/message | list messages (cursor) | list_messages | **Có** |
| GET | /:sessionID/message/:messageID | get message | — | **Missing** (Rust có create_message) |
| DELETE | /:sessionID/message/:messageID | delete message | — | **Missing** |
| DELETE | /:sessionID/message/:messageID/part/:partID | delete part | — | **Missing** |
| PATCH | /:sessionID/message/:messageID/part/:partID | patch part | — | **Missing** |
| POST | /:sessionID/message | create message | create_message | **Có** |
| POST | /:sessionID/prompt_async | **prompt + stream LLM** | — | **Missing** — luồng chính |
| POST | /:sessionID/command | command | — | **Missing** |
| POST | /:sessionID/shell | shell | — | **Missing** |
| POST | /:sessionID/revert | revert | — | **Missing** |
| POST | /:sessionID/unrevert | unrevert | — | **Missing** |
| POST | /:sessionID/permissions/:permissionID | respond to permission | — | **Missing** (Rust permission ở /permission) |

### Bảng so sánh Config & Permission

| Method | Path (TS) | TS | Rust | Ghi chú |
|--------|-----------|----|------|--------|
| GET | /config | get full config | get | **Có** |
| PATCH | /config | update config (merge) | — | Rust dùng PUT config |
| GET | /config/providers | list config providers | — | **Missing** |
| GET | /permission | list pending requests | get | Contract có thể khác |
| POST | /permission/:requestID/reply | reply to request | — | Rust: POST /permission = grant, DELETE /permission/:tool = revoke (khác) |

### Bảng so sánh Project / Workspace

| Method | Path (TS) | TS | Rust | Ghi chú |
|--------|-----------|----|------|--------|
| GET | /project | list projects | — | Rust: GET /workspace = list workspaces |
| GET | /project/current | current project (Instance.project) | — | **Missing** |
| PATCH | /project/:projectID | update project | — | Rust: DELETE /workspace/:id |
| GET | /workspace | — | list | Rust có workspace riêng |
| POST | /workspace | — | create | |
| GET | /workspace/:id | — | get | |
| DELETE | /workspace/:id | — | delete | |

Ưu tiên cao: **session** (đặc biệt **POST prompt_async** + streaming LLM), **workspace/instance context**, **project/workspace**, **config/permission**. Tiếp theo: PTY, experimental, OpenAPI đầy đủ, WebSocket.

## Chiến lược chuyển đổi

### Giai đoạn 1: Parity API và luồng chính

Mục tiêu: app/frontend dùng **Rust server** thay TS mà không đổi contract cho luồng chính (session + prompt, workspace, config, permission).

#### 1. Session prompt & streaming LLM

- **TS:** `packages/pixicode/src/server/routes/session.ts` (POST `/:sessionID/prompt_async`), `session/prompt.ts`, `session/llm.ts`.
- **Rust:** `packages/pixicode-core/src/server/routes/session.rs` — có list, create, get, delete, list_messages, create_message, abort; **chưa có** endpoint gọi LLM + stream.
- **Việc cần làm:**
  - Thêm **POST /session/:id/prompt** (hoặc /prompt_async) trong Rust; body giống TS.
  - Handler: load session + messages → build prompt → gọi LLM stream (provider/tools trong core) → stream event (SSE/JSON) đúng format SDK.
  - Lưu assistant message + parts khi xong; hỗ trợ abort.
  - (Sau) Port thêm session endpoints thiếu: status, children, todo, patch, init, fork, share, diff, summarize, message get/delete/patch part, command, shell, revert, unrevert, permissions reply.

#### 2. Workspace / Instance context (middleware)

- **TS:** `packages/pixicode/src/server/server.ts` — middleware `WorkspaceContext.provide` + `Instance.provide` với `workspace` + `directory` (query/header).
- **Rust:** Chưa có middleware đọc workspace/directory.
- **Việc cần làm:**
  - Thêm **Axum middleware**: đọc `directory` (và `workspace`) từ query hoặc header `x-pixicode-directory` / `x-pixicode-workspace`.
  - Set extension (vd. `Extension(WorkspaceCtx { directory, workspace_id })`) cho handler.
  - (Tùy chọn) Bootstrap instance per directory nếu cần.

#### 3. Project / Workspace mapping

- **TS:** `packages/pixicode/src/server/routes/project.ts` — GET /project, GET /project/current, PATCH /project/:projectID.
- **Rust:** Có /workspace (list, create, get, delete); không có “project” hay “current”.
- **Việc cần làm:**
  - Thêm **GET /project**, **GET /project/current** (trả format giống TS `Project.Info`), có thể map từ workspace + context.
  - Thêm **PATCH /project/:projectID** (update name, icon, commands) — map sang workspace metadata nếu dùng chung store.

#### 4. Config & permission parity

- **Config:** TS có GET /config, PATCH /config (merge), GET /config/providers. Rust có GET/PUT /config. Thêm GET /config/providers nếu app dùng; hỗ trợ PATCH merge hoặc document PUT.
- **Permission:** TS có GET /permission (list pending), POST /permission/:requestID/reply. Rust có GET /permission, POST (grant), DELETE /permission/:tool (revoke). Thêm **POST /permission/:requestID/reply** trong Rust để app reply theo requestID giống TS.

### Giai đoạn 2: Bổ sung thiếu và đặc thù

5. **PTY** (nếu cần): thêm route và xử lý PTY trong Rust (tương đương PtyRoutes).
6. **Experimental**: thêm route tương ứng trong Rust.
7. **OpenAPI**: dùng utoipa (hoặc công cụ khác) sinh spec từ Rust và phục vụ `/doc` giống TS.
8. **WebSocket**: thêm endpoint WS trong Axum nếu frontend/CLI phụ thuộc.

### Giai đoạn 3: Đưa Rust thành backend mặc định

9. **Bin mặc định**
   - Cấu hình monorepo/package để lệnh `pixicode serve` (hoặc `bun run dev:serve`) chạy binary Rust `pixicode-core` thay vì TS server (ví dụ: script trong root hoặc trong package gọi `cargo run -p pixicode-core -- serve` hoặc binary đã build).
10. **E2E / tích hợp**
    - Chạy toàn bộ test E2E và tích hợp với server Rust; sửa test hoặc API nếu lệch contract.
11. **Deprecate / gỡ TS server**
    - Đánh dấu deprecated hoặc xóa `Server.listen` và các route TS, giữ lại TS chỉ cho CLI/TUI/script không phục vụ HTTP (nếu vẫn cần).

## Công việc kỹ thuật gợi ý

- **Rust**
  - Middleware Axum: đọc `workspace`, `directory` (query/header), set extension; optional: instance bootstrap tương đương `InstanceBootstrap`.
  - Session: đảm bảo prompt → LLM stream → lưu message, tương thích SDK/frontend (format event, SSE, JSON).
  - Soạn doc OpenAPI (utoipa) và route `/doc` trả spec.
- **Repo**
  - Một doc (hoặc checklist) trong `docs/` liệt kê từng endpoint TS → Rust, trạng thái parity (done / partial / missing).
  - CI: chạy server Rust và test E2E (nếu có).

## Lưu ý

- Chuyển "toàn bộ" backend sang Rust là dự án lớn, nên làm theo từng phase trên để tránh big-bang.
- Giữ **một nguồn sự thật** cho API: contract (path, body, response) nên được mô tả (OpenAPI) và test; khi port từ TS sang Rust, ưu tiên giữ nguyên contract để app/CLI không đổi.
