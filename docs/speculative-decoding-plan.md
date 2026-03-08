# Kế hoạch áp dụng Speculative Decoding

## Luồng tổng quan

```
User
  │
  ▼
Agent (PixiCode)
  │
  ▼
Draft model — qwen3.5:0.8b
  │
  ▼
Target model (9B) — edu-assistant:latest
  │
  ▼
Response (stream)
```

## 1. Speculative decoding là gì?

- **Draft model** (nhỏ, nhanh): sinh một chuỗi **K** token dự đoán (e.g. K = 4–8).
- **Target model** (lớn, chất lượng): nhận **context + K token dự đoán**, chạy **một** forward pass để có logits cho K+1 vị trí.
- **Verify**: so sánh token draft với phân phối target; chấp nhận đoạn prefix khớp, token đầu tiên không khớp thì lấy mẫu từ target và lặp lại.
- **Lợi ích**: giảm số lần gọi target model → tăng throughput (tokens/s), đặc biệt khi draft và target cùng họ/vocab.

## 2. Ràng buộc trong PixiCode

- **Draft**: `qwen3.5:0.8b` (Ollama: `qwen3.5:0.8b`).
- **Target**: `edu-assistant:latest` (9B, có thể cũng chạy qua Ollama hoặc API riêng).
- Luồng hiện tại: `SessionProcessor.process()` → `LLM.stream(streamInput)` → `streamText({ model: language, ... })` với `language = Provider.getLanguage(input.model)`. Model là **một** `LanguageModelV2` (AI SDK).
- Cần giữ **một** luồng stream ra ngoài (fullStream) để UI/tool/token handling không đổi.

## 3. Hai hướng triển khai

### 3.1 Option A: Dùng tính năng có sẵn của Ollama (nếu có)

- Ollama có thể đã hoặc sẽ hỗ trợ speculative decoding (draft + target trong một request).
- **Việc cần làm**: (1) Kiểm tra tài liệu/API Ollama xem có tham số kiểu `speculative_model` / `draft_model` không. (2) Nếu có, chỉ cần cấu hình provider/model trong PixiCode (draft + target) và gọi API như bình thường.
- **Ưu điểm**: Ít code, tận dụng tối ưu phía server. **Nhược điểm**: Phụ thuộc Ollama, có thể không có sẵn.

### 3.2 Option B: Layer speculative trong PixiCode (wrapper LanguageModelV2)

- Tạo một **wrapper** implement `LanguageModelV2` (AI SDK) bọc hai model: draft + target.
- Bên trong wrapper:
  1. Nhận prompt (system + messages).
  2. **Loop** cho đến khi gặp stop hoặc đủ max tokens:
     - Gọi **draft** (non-stream hoặc stream) để lấy K token.
     - Gọi **target** một lần với input = prompt + K token draft → nhận logits (K+1 vị trí).
     - Verify: so sánh token draft với argmax (hoặc sample) từ logits target; xác định độ dài prefix chấp nhận được.
     - Emit stream parts (text-delta, v.v.) cho các token đã chấp nhận.
     - Nếu có token bị reject: lấy mẫu từ logits target tại vị trí đó, cập nhật context, tiếp tục (có thể giảm K hoặc reset).
  3. Chuẩn hóa output stream theo `LanguageModelV2StreamPart` để session processor không cần sửa.

- **Ưu điểm**: Không phụ thuộc backend, áp dụng được cho bất kỳ cặp draft/target nào (Ollama, OpenAI-compatible, v.v.). **Nhược điểm**: Phải implement đúng cơ chế verify và tokenizer (vocab alignment draft/target).

## 4. Kiến trúc đề xuất (Option B)

### 4.1 Điểm gắn vào code

- **Vị trí**: `packages/pixicode/src/session/llm.ts` — chỗ gọi `streamText({ model: language, ... })`.
- **Cách**: Nếu config bật speculative và model hiện tại có `draftModel` (hoặc tương đương), thay `language` bằng **speculative wrapper**:
  - `language = createSpeculativeLanguageModel(draftLanguage, targetLanguage, opts)`.
  - `draftLanguage` = `Provider.getLanguage(draftModel)`, `targetLanguage` = `Provider.getLanguage(targetModel)` (target có thể chính là `language` hiện tại).

### 4.2 Config (pixicode.json / model options)

Ví dụ:

```json
{
  "model": {
    "providerID": "ollama",
    "modelID": "edu-assistant:latest",
    "speculative": {
      "draft": {
        "providerID": "ollama",
        "modelID": "qwen3.5:0.8b"
      },
      "numDraftTokens": 6
    }
  }
}
```

Hoặc model riêng cho speculative:

```json
{
  "models": {
    "edu-assistant-spec": {
      "providerID": "ollama",
      "modelID": "edu-assistant:latest",
      "speculative": {
        "draftModelID": "qwen3.5:0.8b",
        "numDraftTokens": 6
      }
    }
  }
}
```

### 4.3 Module mới

- **`packages/pixicode/src/session/speculative.ts`** (hoặc `packages/pixicode/src/provider/speculative/`):
  - `createSpeculativeLanguageModel(draft, target, options): LanguageModelV2`
  - Logic: `doStream()` (và nếu cần `doGenerate()`) thực hiện loop draft → target → verify → emit stream.
  - Cần: tokenizer hoặc API trả về token ids để so sánh (Ollama có thể trả về token; OpenAI-compatible thường chỉ text → cần tokenizer chung hoặc so sánh theo text từng token).

### 4.4 Đồng bộ vocab / tokenizer

- Draft (Qwen 0.8B) và target (edu-assistant 9B) nên cùng họ tokenizer (Qwen) để tỷ lệ accept cao.
- Nếu API chỉ trả về text: có thể chunk text theo từ/space và so sánh từng “segment” (kém chính xác hơn token id).
- Tốt nhất: dùng API hoặc model trả về token ids cho draft và logits cho target để verify chuẩn.

## 5. Các phase triển khai (Option B)

| Phase | Nội dung | Đầu ra |
|-------|----------|--------|
| **P1** | Nghiên cứu API Ollama / target: có trả token ids / logits không; format request/response cho multi-step. | Doc ngắn: API nào dùng cho draft/target, có cần proxy không. |
| **P2** | Mở rộng config: thêm `speculative.draft` (hoặc `draftModelID`) và `numDraftTokens` cho model. Đọc config trong provider/llm và resolve draft + target model. | Config đọc được, có thể bật/tắt speculative theo model. |
| **P3** | Implement core: `SpeculativeLanguageModel` (wrapper) với `doStream()` — gọi draft lấy K token, gọi target 1 lần với context+K token, verify và emit stream. Giả định có token id hoặc cách so sánh token (text/token) thống nhất. | Wrapper dùng được nội bộ với 2 model Ollama. |
| **P4** | Tích hợp vào `LLM.stream()`: nếu model có speculative config thì tạo wrapper và truyền vào `streamText({ model: speculativeWrapper, ... })`. Giữ nguyên toàn bộ tool/temperature/messages. | End-to-end: User → Agent → Draft (0.8B) + Target (9B) → Response. |
| **P5** | Đo latency/throughput (tokens/s, time-to-first-token), so sánh với chạy target thuần. Tinh chỉnh K, batch size nếu có. | Số liệu và (tuỳ chọn) điều chỉnh K theo model. |

## 6. Rủi ro và phụ thuộc

- **Tokenizer khác nhau**: Draft và target khác vocab → accept rate thấp → ít lợi. Ưu tiên cặp cùng họ (Qwen).
- **Tool calls**: Speculative decoding thường chỉ cho text. Khi model trả về tool call, cần quy ước: chỉ verify đến trước tool call, hoặc tắt speculative cho turn có tool.
- **Streaming**: Draft có thể stream K token rồi gom lại, hoặc gọi non-stream; target cần logits một lần cho K+1 vị trí → có thể cần API “non-stream one step” hoặc dùng response stream và parse logits nếu backend hỗ trợ.

## 7. Tóm tắt

- **Mục tiêu**: Tăng tốc inference cho target 9B (edu-assistant) bằng draft 0.8B (qwen3.5:0.8b), vẫn giữ luồng User → Agent → Response như hiện tại.
- **Hướng ưu tiên**: Thử Option A (Ollama native) trước; nếu không có thì làm Option B (wrapper trong PixiCode).
- **Điểm gắn**: `LLM.stream()` trong `packages/pixicode/src/session/llm.ts`, config model có `speculative.draft` + `numDraftTokens`.
- **Sản phẩm**: Cấu hình một model “edu-assistant + speculative” dùng draft qwen3.5:0.8b và target edu-assistant:latest, trả về response qua cùng một stream cho session processor.
