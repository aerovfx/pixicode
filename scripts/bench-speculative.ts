#!/usr/bin/env bun
/**
 * Speed Benchmark: edu-assistant (single model) vs Speculative Decoding
 *
 * Runs 3 prompts × 2 methods, measures wall time, TTFT, tok/s, match rate.
 * Uses native Ollama /api/chat with think:false for Qwen3 models.
 *
 * Usage: bun scripts/bench-speculative.ts
 */

const OLLAMA = "http://localhost:11434"
const DRAFT = "qwen3.5:0.8b"
const TARGET = "edu-assistant:latest"
const K = 8 // draft tokens per round
const MAX_TOKENS = 100

const PROMPTS = [
  { id: "P1", text: "What is a binary search tree? Explain briefly." },
  { id: "P2", text: "Write a Python function to check if a string is a palindrome." },
  { id: "P3", text: "Explain the difference between TCP and UDP in networking." },
]

// ─── Ollama API ───────────────────────────────────────────────────────────────

interface OllamaResult {
  content: string
  eval_count: number
  total_ms: number
  prompt_ms: number
  eval_ms: number
  tok_s: number
}

async function chat(model: string, msgs: { role: string; content: string }[], maxTok: number): Promise<OllamaResult> {
  const r = await fetch(`${OLLAMA}/api/chat`, {
    method: "POST",
    body: JSON.stringify({ model, messages: msgs, stream: false, think: false, options: { num_predict: maxTok } }),
  })
  if (!r.ok) throw new Error(`${r.status}: ${await r.text()}`)
  const d = await r.json()
  const evalNs = d.eval_duration ?? 1
  return {
    content: d.message?.content ?? "",
    eval_count: d.eval_count ?? 0,
    total_ms: Math.round((d.total_duration ?? 0) / 1e6),
    prompt_ms: Math.round((d.prompt_eval_duration ?? 0) / 1e6),
    eval_ms: Math.round(evalNs / 1e6),
    tok_s: Math.round((d.eval_count ?? 0) / (evalNs / 1e9)),
  }
}

function tokenize(s: string): string[] {
  return s.split(/\s+/).filter(Boolean)
}

// ─── Method A: Target only (autoregressive) ───────────────────────────────────

interface BenchResult {
  method: string
  promptId: string
  wallMs: number
  ttftMs: number
  tokens: number
  tokS: number
  matchRate: number | null
  output: string
}

async function benchBaseline(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  const res = await chat(TARGET, [{ role: "user", content: prompt }], MAX_TOKENS)
  const wall = Math.round(performance.now() - t0)
  return {
    method: "Baseline",
    promptId,
    wallMs: wall,
    ttftMs: res.prompt_ms,
    tokens: res.eval_count,
    tokS: res.tok_s,
    matchRate: null,
    output: res.content,
  }
}

// ─── Method B: Speculative Decoding ───────────────────────────────────────────

async function benchSpeculative(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let iterations = 0
  let totalMatched = 0
  let totalDraftWords = 0
  let ttft = 0

  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    iterations++
    const ctx = accumulated
      ? [...userMsg, { role: "assistant", content: accumulated }]
      : [...userMsg]

    // Draft and target generate from same context — parallel requests
    const [draftRes, targetRes] = await Promise.all([
      chat(DRAFT, ctx, K),
      chat(TARGET, ctx, K + 1),
    ])

    if (ttft === 0) ttft = Math.round(performance.now() - t0)

    const draftTxt = draftRes.content.trim()
    const targetTxt = targetRes.content.trim()
    if (!targetTxt) break

    // Text-based verify: char-prefix match, snap to word boundary + 1 bonus word
    const dWords = tokenize(draftTxt)
    const tWords = tokenize(targetTxt)
    let n = 0
    while (n < dWords.length && n < tWords.length && dWords[n] === tWords[n]) n++
    totalDraftWords += dWords.length
    totalMatched += n

    // accepted = matched prefix + 1 bonus word from target
    let accepted: string
    if (n > 0) {
      const bonus = n < tWords.length ? " " + tWords[n] : ""
      accepted = tWords.slice(0, n).join(" ") + bonus
    } else {
      accepted = tWords[0] ?? ""
    }

    if (!accepted) break

    const aWords = tokenize(accepted).length
    accumulated += accumulated ? " " + accepted : accepted
    completionTokens += aWords

    if (targetTxt.length <= 3 || (accumulated.endsWith(".") && completionTokens > 30)) break
  }

  const wall = Math.round(performance.now() - t0)
  const matchRate = totalDraftWords > 0 ? Math.round((totalMatched / totalDraftWords) * 100) : 0
  const tokS = completionTokens > 0 && wall > 0 ? Math.round((completionTokens / wall) * 1000) : 0

  return {
    method: "Speculative",
    promptId,
    wallMs: wall,
    ttftMs: ttft,
    tokens: completionTokens,
    tokS,
    matchRate,
    output: accumulated,
  }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

async function main() {
  console.log("╔════════════════════════════════════════════════════════════════════╗")
  console.log("║  SPEED BENCHMARK: edu-assistant vs Speculative Decoding           ║")
  console.log("╠════════════════════════════════════════════════════════════════════╣")
  console.log(`║  Target : ${TARGET} (9.7B Q4_K_M)`)
  console.log(`║  Draft  : ${DRAFT} (0.8B Q8_0)`)
  console.log(`║  K      : ${K} draft tokens/round`)
  console.log(`║  Max    : ${MAX_TOKENS} output tokens`)
  console.log(`║  Prompts: ${PROMPTS.length}`)
  console.log("╚════════════════════════════════════════════════════════════════════╝")

  // Warm up
  console.log("\n  Warming up both models...")
  await chat(DRAFT, [{ role: "user", content: "hi" }], 3)
  await chat(TARGET, [{ role: "user", content: "hi" }], 3)
  console.log("  Ready.\n")

  const results: BenchResult[] = []

  for (const p of PROMPTS) {
    console.log(`─── ${p.id}: "${p.text}" ───\n`)

    // Baseline
    process.stdout.write("  [Baseline]     running... ")
    const b = await benchBaseline(p.id, p.text)
    console.log(`${b.wallMs}ms, ${b.tokens} tok, ${b.tokS} tok/s`)
    console.log(`               → "${b.output.slice(0, 120)}..."\n`)
    results.push(b)

    // Speculative
    process.stdout.write("  [Speculative]  running... ")
    const s = await benchSpeculative(p.id, p.text)
    console.log(`${s.wallMs}ms, ${s.tokens} tok, ${s.tokS} tok/s, match=${s.matchRate}%`)
    console.log(`               → "${s.output.slice(0, 120)}..."\n`)
    results.push(s)
  }

  // ─── Summary table ──────────────────────────────────────────────────────────

  console.log("\n╔════════════════════════════════════════════════════════════════════════════════════╗")
  console.log("║  RESULTS                                                                          ║")
  console.log("╠════════════════════════════════════════════════════════════════════════════════════╣")
  console.log()
  console.log("  Prompt  Method        Wall(ms)  TTFT(ms)  Tokens  Tok/s  Match  Speedup")
  console.log("  ──────  ────────────  ────────  ────────  ──────  ─────  ─────  ───────")

  for (const p of PROMPTS) {
    const b = results.find((r) => r.promptId === p.id && r.method === "Baseline")!
    const s = results.find((r) => r.promptId === p.id && r.method === "Speculative")!
    const speedup = s.wallMs > 0 ? (b.wallMs / s.wallMs).toFixed(2) : "N/A"
    const faster = s.wallMs < b.wallMs ? "↑" : "↓"

    console.log(
      `  ${p.id.padEnd(6)}  Baseline      ${b.wallMs.toString().padStart(7)}   ${b.ttftMs.toString().padStart(7)}    ${b.tokens.toString().padStart(4)}   ${b.tokS.toString().padStart(4)}    —      —`,
    )
    console.log(
      `          Speculative  ${s.wallMs.toString().padStart(7)}   ${s.ttftMs.toString().padStart(7)}    ${s.tokens.toString().padStart(4)}   ${s.tokS.toString().padStart(4)}   ${(s.matchRate + "%").padStart(4)}   ${speedup}x ${faster}`,
    )
  }

  // ─── Aggregates ─────────────────────────────────────────────────────────────

  const baselines = results.filter((r) => r.method === "Baseline")
  const specs = results.filter((r) => r.method === "Speculative")

  const avgBaseWall = Math.round(baselines.reduce((a, r) => a + r.wallMs, 0) / baselines.length)
  const avgSpecWall = Math.round(specs.reduce((a, r) => a + r.wallMs, 0) / specs.length)
  const avgBaseTokS = Math.round(baselines.reduce((a, r) => a + r.tokS, 0) / baselines.length)
  const avgSpecTokS = Math.round(specs.reduce((a, r) => a + r.tokS, 0) / specs.length)
  const avgMatch = Math.round(specs.reduce((a, r) => a + (r.matchRate ?? 0), 0) / specs.length)
  const avgSpeedup = avgSpecWall > 0 ? (avgBaseWall / avgSpecWall).toFixed(2) : "N/A"

  console.log()
  console.log("  ── Averages ──────────────────────────────────────────────────")
  console.log(`  Baseline     : ${avgBaseWall}ms avg wall, ${avgBaseTokS} tok/s avg`)
  console.log(`  Speculative  : ${avgSpecWall}ms avg wall, ${avgSpecTokS} tok/s avg, ${avgMatch}% match`)
  console.log(`  Speedup      : ${avgSpeedup}x ${Number(avgSpeedup) >= 1 ? "(speculative FASTER)" : "(speculative SLOWER)"}`)
  console.log()

  if (Number(avgSpeedup) < 1) {
    console.log("  ⚠ KẾT LUẬN: Speculative decoding CHẬM HƠN baseline trên single-GPU Ollama.")
    console.log("    Nguyên nhân:")
    console.log("    - Match rate quá thấp (~" + avgMatch + "%) → hầu hết draft tokens bị reject")
    console.log("    - Mỗi iteration = 2 API calls (draft + target) thay vì 1")
    console.log("    - GPU chia bandwidth cho 2 models → không có parallel thật")
    console.log("    - Text-based verify không có logit → quá nghiêm ngặt")
  } else {
    console.log("  ✓ Speculative decoding nhanh hơn baseline!")
  }

  console.log("\n╚════════════════════════════════════════════════════════════════════════════════════╝")
}

main().catch(console.error)
