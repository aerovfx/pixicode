# Benchmark: edu-assistant (single model) vs Speculative Decoding

> Ngày chạy: 2026-03-07
> Ollama: v0.17.7 | Single GPU | macOS ARM64

## Setup

| Thành phần | Chi tiết |
|------------|----------|
| **Target model** | `edu-assistant:latest` — 9.7B params, Q4_K_M, ~8.6 GB VRAM |
| **Draft model** | `qwen3.5:0.8b` — ~0.8B params |
| **Model family** | Qwen (draft qwen2, target edu-assistant) |
| **VRAM tổng** | ~10.7 GB (cả hai loaded đồng thời) |
| **K (draft tokens/round)** | 8 |
| **Max output tokens** | 100 |
| **Verification** | Text-based (word matching), không có logit access |
| **Script** | `bun scripts/bench-speculative.ts` |

## Prompts

| ID | Prompt |
|----|--------|
| P1 | "What is a binary search tree? Explain briefly." |
| P2 | "Write a Python function to check if a string is a palindrome." |
| P3 | "Explain the difference between TCP and UDP in networking." |

## Kết quả chi tiết

### P1: Binary Search Tree

| Method | Wall time | TTFT | Tokens | Tok/s | Match | Speedup |
|--------|-----------|------|--------|-------|-------|---------|
| **Baseline** | **12,113 ms** | 430 ms | 100 | **9** | — | — |
| Speculative | 50,161 ms | 7,193 ms | 41 | 1 | 16% | 0.24x ↓ |

**Baseline output:**
> A **Binary Search Tree (BST)** is a specific type of data structure that consists of nodes arranged in a hierarchical fo...

**Speculative output:**
> A **Binary Search Tree (BST)** is a rooted, ordered binary tree in which every node has the following properties: 1. Al...

---

### P2: Python Palindrome

| Method | Wall time | TTFT | Tokens | Tok/s | Match | Speedup |
|--------|-----------|------|--------|-------|-------|---------|
| **Baseline** | **13,349 ms** | 467 ms | 100 | **8** | — | — |
| Speculative | 285,622 ms | 103,630 ms | 41 | 0 | 29% | 0.05x ↓ |

**Baseline output:**
> Here's a simple and efficient Python function to check whether a string is a palindrome: `def is_palindrome(s:...`

**Speculative output:**
> `def is_palindrome(s: str) -> bool: """Check if a string is a palindrome...`

> **Lưu ý:** P2 speculative chạy cực chậm (285s) do code block formatting khiến draft và target diverge mạnh, mỗi round chỉ accept 1 word → rất nhiều iterations.

---

### P3: TCP vs UDP

| Method | Wall time | TTFT | Tokens | Tok/s | Match | Speedup |
|--------|-----------|------|--------|-------|-------|---------|
| **Baseline** | 11,618 ms | 343 ms | 100 | **9** | — | — |
| Speculative | **10,066 ms** | 1,852 ms | 12 | 1 | 20% | 1.15x ↑ |

> **Lưu ý:** P3 speculative "nhanh hơn" chỉ vì dừng sớm (12 tokens) — không thực sự nhanh hơn per-token.

---

## Tổng hợp

| Metric | Baseline (avg) | Speculative (avg) |
|--------|----------------|-------------------|
| **Wall time** | **12,360 ms** | 115,283 ms |
| **Tok/s** | **9** | 1 |
| **Match rate** | — | 22% |
| **Speedup** | — | **0.11x (9x chậm hơn)** |

## Biểu đồ so sánh

```
Wall time (ms) — thấp hơn = tốt hơn
═══════════════════════════════════════════════════

P1 Baseline    ████████████░░░░░░░░░░░░░░  12,113 ms
P1 Speculative ██████████████████████████████████████████████████  50,161 ms

P2 Baseline    █████████████░░░░░░░░░░░░░  13,349 ms
P2 Speculative ██████████████████████████████████████████████████████████████  285,622 ms !!

P3 Baseline    ███████████░░░░░░░░░░░░░░░  11,618 ms
P3 Speculative ██████████░░░░░░░░░░░░░░░░  10,066 ms (sớm dừng)


Tok/s — cao hơn = tốt hơn
═══════════════════════════════════════════════════

Baseline    █████████  9 tok/s
Speculative █          1 tok/s
Draft only  ████████████████████████████  28 tok/s (tham khảo)
```

## Phân tích nguyên nhân

### 1. Match rate quá thấp (22%)
- Text-based verification so sánh từng word giữa draft và target output
- Model 0.8B và 9.7B chọn từ khác nhau dù cùng ý nghĩa
- VD: Draft nói "a method used in" → Target nói "an optimization technique where" = **0% match**
- Không có logit/probability access để verify mềm hơn

### 2. GPU bandwidth contention
- Cả hai model chạy trên cùng 1 GPU
- `Promise.all()` gửi request song song nhưng Ollama serialize compute nội bộ
- Mỗi iteration = draft forward pass + target forward pass + context switch
- Không có parallel thật sự

### 3. Overhead per iteration
- Baseline: 1 API call → 100 tokens liên tục (9 tok/s)
- Speculative: ~30-40 API calls (mỗi iteration 2 calls) → chỉ accept 1-2 words/round
- Tổng overhead >> continuous autoregressive generation

### 4. Qwen3 thinking mode
- Qua OpenAI-compatible API (`/v1/chat/completions`), Qwen3 models bật thinking mặc định
- Output vào `reasoning` field thay vì `content` → empty response
- Chỉ native `/api/chat` với `think: false` mới hoạt động
- Ảnh hưởng trực tiếp tới `@ai-sdk/openai-compatible` integration

## Kết luận

| | Baseline | Speculative |
|---|---|---|
| **Tốc độ** | 9 tok/s | 1 tok/s |
| **Latency** | 12s / 100 tokens | 115s / 41 tokens |
| **Chất lượng** | Target model quality | Target-verified nhưng ít tokens |
| **Khuyến nghị** | **Sử dụng** | **Không sử dụng** trên single-GPU |

### Speculative decoding text-based KHÔNG khả thi trên single-GPU Ollama

## Khuyến nghị

1. **Tắt `options.speculative` mặc định** cho Ollama single-GPU (đã thực hiện)
2. **Chỉ bật khi có:**
   - Multi-GPU (draft trên GPU1, target trên GPU2)
   - Logit-level verification (API hỗ trợ `logprobs`)
   - Ollama native speculative decoding (tương lai)
3. **Thay thế bằng:**
   - Prompt caching / KV cache reuse → giảm TTFT
   - Streaming autoregressive → latency cảm nhận tốt hơn

## Cách chạy lại

```bash
# Đảm bảo cả 2 model đã loaded
ollama run qwen3.5:0.8b "hi" --nowordwrap
ollama run edu-assistant:latest "hi" --nowordwrap

# Chạy benchmark
bun scripts/bench-speculative.ts
```
