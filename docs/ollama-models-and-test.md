# Load model Ollama và chạy test

## Models cho speculative decoding

| Model                 | Vai trò  | Trên máy bạn / Nguồn |
| --------------------- | -------- | ---------------------- |
| **Draft (nhỏ)**       | Draft    | **qwen3.5:0.8b** — mặc định. Hoặc qwen2:0.5b / qwen3:0.6b. |
| **edu-assistant:latest** | Target | Bạn đã có (local). |
| **qwen3-coder:480b-cloud** | (khác) | Bạn đã có; có thể dùng làm target nếu cần. |

- **Draft:** Cần một model nhỏ (0.5B–0.8B) để speculative nhanh. Trên Ollama Library có `qwen3.5:0.8b`; nếu không thấy hoặc không pull được, dùng `qwen:0.5b-chat` (rất nhỏ).
- **Target:** PixiCode mặc định dùng `edu-assistant:latest` (bạn đã có). Không có trên Library — chỉ tồn tại local hoặc registry riêng.

## 1. Load model (Ollama phải đang chạy)

Trên Ollama bạn đang có: **edu-assistant:latest**, **qwen3-coder:480b-cloud**. Draft mặc định: **qwen3.5:0.8b** — pull nếu chưa có.

Từ thư mục gốc repo:

```bash
./scripts/setup-ollama-models.sh
```

- **Draft qwen3.5:0.8b:** Có trên [Ollama Library](https://ollama.com/library/qwen3.5:0.8b). Chạy thủ công nếu script không pull được:
  ```bash
  ollama pull qwen3.5:0.8b
  ```
  Nếu vẫn không thấy (region/network): dùng draft thay thế nhỏ hơn:
  ```bash
  ollama pull qwen:0.5b-chat
  ```
  Rồi trong config speculative đặt `draft: "ollama/qwen:0.5b-chat"`.
- **Target edu-assistant:latest:** Bạn đã có local; script không cần pull. Nếu muốn target khác (vd. qwen3-coder) thì đổi model trong config.

### 1.1. Target: tạo edu-assistant:latest local (Modelfile)

Nếu bạn có base model (vd. Qwen 9B), tạo `edu-assistant` local:

```bash
# Ví dụ: dùng qwen2.5:7b làm base, đặt tên edu-assistant:latest
ollama pull qwen2.5:7b
mkdir -p /tmp/ollama-edu
cat > /tmp/ollama-edu/Modelfile << 'EOF'
FROM qwen2.5:7b
PARAMETER temperature 0.7
SYSTEM "You are an educational assistant."
EOF
ollama create edu-assistant:latest -f /tmp/ollama-edu/Modelfile
ollama list
```

Hoặc dùng model 9B khác (nếu có trên Library) và tạo alias `edu-assistant:latest`.

### 1.2. Target: dùng model thay thế để test

Để test speculative mà không tạo `edu-assistant`, dùng một target có sẵn (vd. `qwen2.5:7b`) và cấu hình model tương ứng (target `ollama/qwen2.5:7b`, speculative.draft `ollama/qwen3.5:0.8b`).

## 2. Cấu hình speculative (tuỳ chọn)

**Chế độ mặc định:** Chạy không speculative (chỉ target). Để **bật** speculative decoding với **edu-assistant:latest** (target) và **qwen3.5:0.8b** (draft), thêm trong `pixicode.json`:

```json
{
  "models": {
    "ollama/edu-assistant:latest": {
      "options": {
        "speculative": {
          "draft": "ollama/qwen3.5:0.8b",
          "numDraftTokens": 6
        }
      }
    }
  }
}
```

Nếu bạn dùng draft khác (vd. **qwen3.5:0.8b** hoặc **qwen:0.5b-chat**):

```json
"speculative": {
  "draft": "ollama/qwen3.5:0.8b",
  "numDraftTokens": 6
}
```

## 3. Chạy test

**Không** chạy test từ thư mục gốc repo. Chạy từ package:

```bash
cd packages/pixicode
bun test --timeout 30000
```

Chạy một file test cụ thể:

```bash
cd packages/pixicode
bun test path/to/test-file.ts
```

Backend (serve) cần chạy nếu test gọi API (session, LLM). Chạy full stack để test end-to-end:

```bash
# Terminal 1: backend
bun run dev:serve

# Terminal 2: web (hoặc chỉ chạy test)
bun run dev:web
# hoặc
cd packages/pixicode && bun test
```

## 4. Kiểm tra nhanh model

```bash
# Draft (mặc định)
ollama run qwen3.5:0.8b "Hello, one short sentence."
# hoặc
ollama run qwen3.5:0.8b "Hello, one short sentence."

# Target (bạn đã có)
ollama run edu-assistant:latest "Hello, one short sentence."
```

Thoát chat: `/bye` hoặc Ctrl+D.

## 5. Kiểm tra model đã cài

**Liệt kê tất cả model đã có:**

```bash
ollama list
```

Cột đầu là **tên** (vd. `qwen3.5:0.8b`, `qwen2:0.5b`, `edu-assistant:latest`), cột thứ hai là digest. Nếu tên xuất hiện trong list thì model đã cài.

**Kiểm tra một tên cụ thể (có trong list hay không):**

```bash
ollama list | grep -E "qwen3.5:0.8b|qwen2:0.5b"
```

**Chạy thử (nếu có thì chạy được):**

```bash
ollama run qwen3.5:0.8b "Hi"
# hoặc
ollama run qwen3.5:0.8b "Hi"
```

Thoát: `/bye` hoặc Ctrl+D. Nếu model chưa cài, Ollama sẽ báo không tìm thấy và có thể gợi ý pull.
