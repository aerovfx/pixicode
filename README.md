<p align="center">
  <a href="https://github.com/aerovfx/pixicode">
    <img src="packages/web/public/logo.png" alt="Aerovfx logo" width="180">
  </a>
</p>
<p align="center">AI coding agent — fork with IDE-like session UI.</p>
<p align="center">
  <a href="https://github.com/aerovfx/pixicode"><img alt="Repo" src="https://img.shields.io/badge/repo-aerovfx%2Fpixicode-blue?style=flat-square" /></a>
</p>

<p align="center">
  <a href="README.md">English</a> |
  <a href="README.zh.md">简体中文</a> |
  <a href="README.zht.md">繁體中文</a> |
  <a href="README.ko.md">한국어</a> |
  <a href="README.fr.md">Français</a>
</p>

## Interface (session + editor + file tree)

Giao diện làm việc theo ba cột giống IDE (VS Code / Cursor): session & AI ở trái, editor ở giữa, cây file ở phải. Nội dung tạo từ cột trái có thể cập nhật vào editor giữa và tự động phản ánh ở cây file; hỗ trợ chỉnh sửa trực tiếp và lưu file.

[![Aerovfx — session interface: session + editor + file tree](docs/interface-session.png)](docs/interface-session.png)

| Cột | Chức năng |
|-----|-----------|
| **Trái** | Session (outline, ghi chú), ô **Ask anything...** và chọn agent/model. Nội dung hoặc chỉnh sửa do AI tạo ra có thể được áp dụng vào file và hiển thị ở cột giữa. |
| **Giữa** | Editor file dạng tab: xem và **chỉnh sửa** nội dung (nút Edit, Ctrl+S lưu). Tự động reload khi session thay đổi file; có thể mở file đầu tiên bị thay đổi. |
| **Phải** | Cây thư mục (Changes / All files), hiển thị file đã sửa (M). Bật **Auto-accept permissions** để chỉnh sửa từ AI được ghi thẳng xuống đĩa. |

---

## Kiến trúc hệ thống

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Clients                                                                 │
│  ├── Web app (Solid.js, Vite)     bun run dev:web / dev:web:full        │
│  ├── Desktop (Tauri)              bun run dev:desktop                    │
│  └── TUI (Rust / TS)              cargo run -p pixicode-tui / bun run dev│
└─────────────────────────────────┬───────────────────────────────────────┘
                                  │ HTTP / SSE
┌─────────────────────────────────▼───────────────────────────────────────┐
│  Backend (Serve)                                                         │
│  ├── TypeScript (pixicode)        bun run dev:serve                      │
│  └── Rust (pixicode-core)        cargo run -p pixicode-core -- serve     │
│  • Session, messages, permissions • file.read / file.write • LSP, tools  │
└─────────────────────────────────┬───────────────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────────────┐
│  Core / Data                                                            │
│  ├── pixicode-core (Rust)  — tools, providers, cost, apply_patch, …     │
│  ├── pixicode-types       — shared types                                │
│  └── SDK (JS)             — OpenAPI client (file.write, session.*, …)   │
└─────────────────────────────────────────────────────────────────────────┘
```

- **Frontend**: Web app gọi SDK (file.read, file.write, session.*). Editor giữa dùng file.write để lưu khi user bấm Save / Ctrl+S; khi AI sửa file (qua session), app reload tab và có thể mở file thay đổi đầu tiên.
- **Backend**: Serve xử lý session, permissions, và file (GET/POST `/file/content`). Có thể chạy bằng TS hoặc Rust core.
- **SDK**: Client sinh từ OpenAPI, dùng cho web/desktop (directory, workspace, file.read, file.write, session.diff, …).

---

### Cài đặt

```bash
# Nhanh
curl -fsSL https://pixibox.ai/install | bash

# Hoặc
npm i -g pixicode-ai@latest   # hoặc bun/pnpm/yarn
brew install anomalyco/tap/pixicode
```

### Chạy từ repo (development)

Từ thư mục gốc repo, sau khi clone và cài dependency:

| Lệnh | Mô tả |
|------|--------|
| `bun run dev` | TUI/console (terminal) |
| `bun run dev:desktop` | Ứng dụng desktop (Tauri) |
| `bun run dev:web` | Chỉ frontend web |
| `bun run dev:serve` | Backend (TypeScript) |
| `bun run dev:serve:rust` | Backend (Rust core) |
| `bun run dev:web:full` | Serve + web cùng lúc |
| `cargo run -p pixicode-tui` | TUI Rust |
| `cargo run -p pixicode-core -- serve` | Backend Rust |

**Tự động cập nhật & lưu:** Khi AI sửa file, app reload tab đang mở và có thể mở file thay đổi đầu tiên ở cột giữa. Để mọi chỉnh sửa từ AI được ghi thẳng xuống đĩa, bật **Auto-accept permissions** (toggle cạnh ô composer).

### Agents

- **build** — Agent mặc định, đầy đủ quyền (file, shell, …).
- **plan** — Chỉ đọc, phù hợp phân tích code / lên kế hoạch.
- **@general** — Subagent cho tìm kiếm phức tạp, task nhiều bước.

### Tài liệu

Tài liệu gốc: [pixibox.ai/docs](https://pixibox.ai/docs). Repo: [github.com/aerovfx/pixicode](https://github.com/aerovfx/pixicode).

### License

MIT © 2025–2026.
