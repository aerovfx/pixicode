# Speculative decoding — benchmark và tinh chỉnh K

## Chế độ chạy model

- **Mặc định:** Chạy **không** speculative (chỉ target). Nhanh hơn trên single-GPU Ollama.
- **Bật speculative:** Thêm `options.speculative` vào model trong `pixicode.json` (xem [Config mẫu](#config-mẫu)). Dùng khi muốn thử speculative hoặc có multi-GPU / logit verification.
- Speculative tự tắt khi turn có **tools** (tool calls).

## Benchmark Results (2026-03-07)

### Setup
- **Draft:** `qwen3.5:0.8b`; hoặc qwen2:0.5b / qwen3:0.6b
- **Target:** `edu-assistant:latest` (9.7B, Q4_K_M, ~8.6GB VRAM)
- **Family:** cả hai đều `qwen35`
- **GPU:** Single GPU, cả hai model loaded đồng thời trong VRAM (10.7GB)
- **Ollama:** v0.17.7

### Kết quả

| Method | Wall time | Tokens | Tok/s | TTFT | Match |
|--------|-----------|--------|-------|------|-------|
| Baseline (target autoregressive) | 9,406ms | 79 | **9** | — | — |
| Draft only | 7,600ms | 80 | **28** | — | — |
| Speculative K=6 (parallel) | 39,196ms | 35 | **1** | 3,221ms | **7%** |
| Speculative K=12 (parallel) | 72,392ms | 56 | **1** | 2,210ms | **8%** |
| Target incremental (20/round) | 11,015ms | 69 | **6** | 2,465ms | — |

### Vấn đề phát hiện

1. **Text-based verification thất bại** — match rate chỉ 7-8%
   - Word/char matching quá chặt khi không có logit access
   - Draft (0.8B) và target (9.7B) dùng từ khác nhau dù cùng ý
   - VD: Draft nói "a method used in" vs Target nói "an optimization technique where" → 0% match

2. **GPU bandwidth contention**
   - `Promise.all()` không giúp — Ollama serialize compute nội bộ
   - Mỗi iteration: draft forward → target forward → context switch overhead

3. **Speculative chậm hơn baseline 4-8x**
   - 39-72 giây vs 9 giây cho baseline autoregressive
   - Overhead từ repeated small generations >> continuous autoregressive

4. **Qwen3 thinking mode bug** qua OpenAI API
   - `/v1/chat/completions` không hỗ trợ `think: false`
   - Output vào `reasoning` field thay vì `content`
   - Chỉ native `/api/chat` với `think: false` mới hoạt động
   - `@ai-sdk/openai-compatible` bị ảnh hưởng trực tiếp

### Kết luận

**Speculative decoding text-based KHÔNG khả thi** trên single-GPU Ollama:
- Không có logit access → verification quá thô
- Cùng GPU → không parallel thật sự
- Overhead > benefit ở mọi K tested

### Khuyến nghị

1. **Tắt `options.speculative` mặc định** cho single-GPU Ollama
2. **Fix `think: false`** trong OpenAI-compatible path cho Qwen3
3. **Real speculative decoding cần:** logit-level verification HOẶC separate GPU cho draft/target
4. **Thay thế bằng:** prompt caching / KV cache reuse (giảm TTFT, đơn giản hơn)

### Thử verify theo logit (Ollama native logprobs)

Đã thử benchmark **verify theo logit** với Ollama `/api/chat` (logprobs: true, top_logprobs: 10):

- **Script:** `bun scripts/bench-speculative-logprob-ollama.ts`
- **Cách verify:** So sánh từng token của draft (text) với target logprobs: chấp nhận nếu draft token trùng target token hoặc nằm trong top-N với logprob > threshold (-4).

**Kết quả (qwen3.5:0.8b + edu-assistant:latest):**

| Method        | Wall (avg) | Match | vs Baseline |
|---------------|------------|-------|-------------|
| Baseline      | ~13.7s     | —     | —           |
| Spec-Text     | ~16s       | 21%   | 0.85x       |
| Spec-Logprob  | ~41.5s     | 11%   | 0.33x       |

**Kết luận thử nghiệm:**

- Verify theo logit **không nhanh hơn** trên Ollama: mỗi request target có `logprobs: true` chậm hơn nhiều, tổng wall time tăng ~2.5x so với Spec-Text.
- Match rate logprob **thấp hơn** text (11% vs 21%) vì alignment draft (text theo từ) với target (subword tokens) dễ lệch; không có tokenizer chung nên so sánh character-slice với token logprobs không ổn định.
- **Để speculative thật sự nhanh với verify logit:** cần (1) target trả logprobs không làm chậm đáng kể (hoặc verify trong cùng forward pass), (2) draft và target dùng chung tokenizer hoặc so sánh theo token ID thay vì text.

---

## Metrics

- **Time to first token (TTFT):** thời gian từ lúc gửi request đến token đầu tiên.
- **Tokens per second (tokens/s):** tổng completion tokens / thời gian từ token đầu đến token cuối.
- **End-to-end latency:** tổng thời gian đến khi `finishReason: "stop"`.

## Cách đo

1. Cùng một prompt, chạy 2 run:
   - Model **không** speculative (chỉ target, ví dụ `edu-assistant:latest` không config `speculative`).
   - Model **có** speculative (ví dụ `edu-assistant:latest` với `speculative: { draft: "ollama/qwen3.5:0.8b", numDraftTokens: 6 }`).
2. Ghi TTFT, tokens/s, e2e cho mỗi run (có thể log trong processor hoặc client, hoặc gọi `LLM.stream` trong script/test).
3. (Tuỳ chọn) Bật log debug cho `session.speculative` để xem số vòng lặp, số token accept mỗi vòng, tổng token đã emit.

## Tinh chỉnh K (`numDraftTokens`)

- Thử `numDraftTokens` = **4, 6, 8, 12**.
- K lớn hơn → tiềm năng tốc độ cao hơn nhưng accept rate có thể giảm.
- **Kết quả thực tế:** K = 6 và K = 12 đều cho match rate ~7-8% — không cải thiện.

## Config mẫu

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

## Lưu ý

- Speculative bị tắt khi turn có tool (`tools` không rỗng) — đúng.
- Verify bằng text (không có logits từ Ollama) nên accept rate thấp hơn so với verify bằng logits.
- Draft và target nên cùng họ (Qwen) để giảm lệch — nhưng ngay cả cùng họ, 0.8B vs 9.7B vẫn quá lệch.
- **Trên single-GPU: KHÔNG nên bật speculative.** Chỉ có lợi khi có multi-GPU hoặc logit access.

### Tại sao vẫn thấy "thinking" khi chạy draft?

- **Qwen2:0.5b không có thinking mode** (thẻ `<think>`) — dùng làm draft để tránh lệch token.
- Nếu vẫn thấy thinking trong UI: (1) **Turn có tool** → speculative tắt, stream từ **target** (edu-assistant có thể có thinking). (2) **Chọn draft làm model chính** → stream từ draft (vd. qwen3.5:0.8b); backend có thể trả `reasoning_text` → UI hiển thị. (3) **Thinking trong text:** Wrapper speculative **đã strip** nội dung thẻ think khỏi draft/target text trước khi verify và emit, nên không rò rỉ ra stream.

## Script test

```bash
bun scripts/test-speculative.ts
```
