# P1: Ollama / API cho Speculative Decoding

## 1. Ollama native speculative decoding

- Ollama **chưa hỗ trợ** speculative decoding built-in (xem [issue #9216](https://github.com/ollama/ollama/issues/9216), [issue #5800](https://github.com/ollama/ollama/issues/5800)).
- Không có tham số `draft_model` hay `speculative_model` trong API chuẩn.
- Response chat/generate không trả `logits` hay `token_ids` trong stream.

## 2. API dùng trong PixiCode

- PixiCode gọi model qua **OpenAI-compatible** chat completions:
  - [OpenAICompatibleChatLanguageModel](packages/pixicode/src/provider/sdk/copilot/chat/openai-compatible-chat-language-model.ts): `doStream` / `doGenerate`.
  - Body: `{ model, messages, stream, max_tokens, temperature, ... }`.
- Response stream: `choices[].delta.content` (text), không có `logits` hay `token_ids`.

## 3. Chiến lược verify (không có logits)

- **Text-based verify:**  
  - Draft sinh K token (gom từ stream text).  
  - Target nhận **một** request với prompt = context + draft_text, `max_tokens = K+1`.  
  - So sánh draft text và target output theo từng “token” text (split space hoặc segment). Chấp nhận prefix khớp; token đầu không khớp thì lấy đúng 1 token từ target và lặp lại.
- **Sau này:** Nếu có endpoint/backend trả logits (custom Ollama fork hoặc proxy), có thể chuyển sang verify bằng logits để tăng accept rate.

## 4. Quyết định

- Dùng **chat completions** (stream hoặc non-stream) cho cả draft và target.
- Verify bằng **text** (split space hoặc tokenizer nếu có).
- Draft và target dùng cùng base URL Ollama (cùng provider config).
