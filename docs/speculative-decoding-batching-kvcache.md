# Speculative decoding + Batching + KV cache reuse — Áp dụng và đánh giá

## Tóm tắt áp dụng

| Kỹ thuật              | Trạng thái    | Vị trí / Ghi chú                                                                 |
| --------------------- | ------------- | -------------------------------------------------------------------------------- |
| **Speculative decoding** | ✅ Đã áp dụng | Wrapper app (draft 0.8B → target 9B, verify text), bật khi model có config, tắt khi có tool. |
| **Batching**          | ⚠️ Chưa áp dụng (server) | Thuộc tầng inference (Ollama/vLLM). App gửi 1 request/turn; batching cần hỗ trợ từ backend. |
| **KV cache reuse**    | ✅ Một phần (trong server) | Ollama tái sử dụng KV cache nội bộ mỗi model. Giữa draft và target không dùng chung cache (khác model). |

---

## 1. Speculative decoding (đã áp dụng)

### Luồng hiện tại

```
User → Agent → LLM.stream
  → resolveSpeculativeDraft(model) → draft (qwen3.5:0.8b) + target (edu-assistant:latest)
  → createSpeculativeLanguageModel(draft, target, { numDraftTokens: K })
  → doStream: loop [ draft.doGenerate(K) → target.doGenerate(prompt+draft, K+1) → verify text → emit ]
  → streamText → SessionProcessor → response
```

### Đặc điểm

- **Config:** `model.options.speculative = { draft: "ollama/qwen3.5:0.8b", numDraftTokens: 6 }`.
- **Verify:** Text-based (split space), không dùng logits (Ollama không trả).
- **Tool:** Turn có tool thì dùng target thuần, không qua wrapper.
- **File:** `packages/pixicode/src/session/speculative.ts`, gắn trong `llm.ts`.

### Đánh giá

- **Ưu:** Giảm cảm nhận latency nhờ draft nhanh; không đổi SessionProcessor; tắt speculative khi có tool đúng như thiết kế.
- **Hạn chế:** Accept rate thấp hơn verify-by-logits; phụ thuộc draft/target cùng họ (Qwen) để ít lệch.

---

## 2. Batching

### Ý nghĩa

- Gộp nhiều request inference (nhiều prompt hoặc nhiều user) thành một batch để tăng utilization GPU và throughput (tokens/s/GPU).

### Vị trí áp dụng

- **Server inference:** Backend (Ollama, vLLM, TGI, …) nhận nhiều request và xử lý theo batch (continuous batching / dynamic batching).
- **Client (PixiCode):** Hiện gửi **một request per turn** (một session, một stream). Để “batching” có hiệu lực, cần nhiều request đồng thời tới cùng một server; khi đó server mới có thể batch (nếu hỗ trợ).

### Hiện trạng

- **Ollama:** API chat/generate theo request đơn, không expose batch API chuẩn; batching (nếu có) là nội bộ.
- **vLLM / TGI:** Có continuous batching; cần dùng backend tương thích và client gửi nhiều request song song (nhiều session/user) thì mới tận dụng.

### Đánh giá

- **App hiện tại:** Không có thay đổi code để “bật batching” — batching là hành vi của server khi có nhiều request.
- **Khuyến nghị:** (1) Nếu chạy Ollama: theo dõi bản cập nhật hỗ trợ batching/queue. (2) Nếu chuyển sang vLLM (hoặc API tương thích): giữ client gửi request như hiện tại; cấu hình server để bật continuous batching và (nếu cần) nhiều worker để phục vụ nhiều session đồng thời.

---

## 3. KV cache reuse

### Ý nghĩa

- Lưu KV cache của prefix (prompt + phần đã sinh) để lần generate tiếp chỉ tính cho token mới, giảm compute và latency.

### Vị trí áp dụng

- **Trong một model:** Mỗi lần gọi `doGenerate(prompt)` với prompt dài, engine inference (Ollama, vLLM, …) thường cache K/V của prompt và chỉ decode phần mới. PixiCode không điều khiển trực tiếp; đây là hành vi mặc định của server.
- **Speculative trong PixiCode:** Mỗi vòng gọi `draft.doGenerate(...)` và `target.doGenerate(prompt + draftText, K+1)`. Prompt cho target là “context + accumulated + draft”; server target có thể reuse KV cache cho phần “context + accumulated” nếu API hỗ trợ (vd. gửi prefix cache id hoặc same connection/session). Hiện API OpenAI-compatible/Ollama không expose cache handle từ client, nên reuse (nếu có) là nội bộ server.
- **Giữa draft và target:** Draft (0.8B) và target (9B) là hai model khác nhau → **không** chia sẻ KV cache với nhau.

### Đánh giá

- **Đã có (ẩn):** Ollama/vLLM reuse KV cache trong một model cho từng request.
- **Chưa có (app):** Client không gửi cache key/prefix; không có cơ chế “tiếp tục từ cache” qua API. Để tận dụng tối đa cần API server hỗ trợ (vd. cache key, multi-step với same context).
- **Kết luận:** KV cache reuse hiện nằm ở phía server; kiến trúc speculative của app (draft → target, từng vòng) tương thích với việc server tối ưu nội bộ, không cần sửa flow app để “bật” thêm reuse giữa draft và target.

---

## 4. Đánh giá tổng thể quá trình áp dụng

### Đạt được

1. **Speculative decoding:** Hoạt động end-to-end với draft (0.8B) và target (9B), config theo model, tắt khi có tool; doc P1, P2–P5 và benchmark đã có.
2. **Tách bạch trách nhiệm:** App chỉ điều khiển luồng (draft → target → verify → stream); batching và KV cache là tối ưu của tầng inference.
3. **Tương thích:** Không đổi SessionProcessor hay format stream; có thể bật/tắt speculative theo model.

### Hạn chế

1. **Verify bằng text:** Accept rate thấp hơn so với verify bằng logits; phụ thuộc draft/target gần nhau (cùng họ).
2. **Batching:** Chưa áp dụng ở app; phụ thuộc backend (Ollama/vLLM) và workload (nhiều request đồng thời).
3. **KV cache reuse qua API:** Chưa có; reuse hiện chỉ trong nội bộ server từng model.

### Khuyến nghị tiếp theo

| Hạng mục              | Hành động gợi ý |
| ---------------------- | ----------------- |
| Speculative           | Thử K = 4, 6, 8; ghi lại TTFT/tokens/s (xem `docs/speculative-decoding-bench.md`). Cân nhắc backend trả logits để verify chính xác hơn. |
| Batching              | Khi có nhiều user/session: dùng backend hỗ trợ continuous batching (vd. vLLM), giữ client gửi request như hiện tại. |
| KV cache              | Theo dõi API Ollama/vLLM (cache handle, prefix caching); khi có chuẩn chung có thể bổ sung option trong client (vd. gửi context_id) để giảm latency cho prompt dài. |

---

## Tài liệu liên quan

- [P1: Ollama/API](speculative-decoding-p1-api.md)
- [Benchmark và K](speculative-decoding-bench.md)
- Wrapper: `packages/pixicode/src/session/speculative.ts`
- Gắn vào stream: `packages/pixicode/src/session/llm.ts`
