#!/usr/bin/env bun
/**
 * Speculative Decoding Test v2 — improved algorithm
 *
 * Changes from v1:
 *  - Parallel draft+target requests (both models loaded in VRAM)
 *  - Relaxed character-prefix matching instead of strict word matching
 *  - Test different K values
 *  - Compare: baseline vs speculative vs parallel-both
 *
 * Usage: bun scripts/test-speculative.ts
 */

const OLLAMA = "http://localhost:11434"
const DRAFT_MODEL = "qwen3.5:0.8b"
const TARGET_MODEL = "edu-assistant:latest"
const MAX_TOKENS = 80
const TEST_PROMPT = "Explain what speculative decoding is in AI inference, in 3 sentences."

// ─── Helpers ──────────────────────────────────────────────────────────────────

function tokenize(s: string): string[] {
  return s.split(/\s+/).filter(Boolean)
}

/**
 * Relaxed verification: find longest matching CHARACTER prefix between draft and target,
 * then snap to word boundary. This handles cases where models use slightly different
 * punctuation or spacing but the core text is the same.
 */
function verifyRelaxed(draftText: string, targetText: string): {
  accepted: string
  matchedChars: number
  draftChars: number
} {
  const d = draftText.trimStart()
  const t = targetText.trimStart()
  let i = 0
  while (i < d.length && i < t.length && d[i] === t[i]) i++

  if (i === 0) {
    // No match — still accept target's first word as "bonus"
    const firstWord = tokenize(t)[0] || ""
    return { accepted: firstWord, matchedChars: 0, draftChars: d.length }
  }

  // Snap to last word boundary
  let accepted = t.slice(0, i)
  const lastSpace = accepted.lastIndexOf(" ")
  if (i < t.length && lastSpace > 0) {
    accepted = accepted.slice(0, lastSpace)
  }
  // Add one bonus word from target beyond the match
  const remaining = t.slice(accepted.length).trim()
  const bonus = tokenize(remaining)[0] || ""
  if (bonus) accepted = accepted + " " + bonus

  return { accepted: accepted.trim(), matchedChars: i, draftChars: d.length }
}

interface ChatResult {
  content: string
  eval_count: number
  total_duration_ms: number
  prompt_eval_ms: number
  eval_ms: number
  tok_per_sec: number
}

async function ollamaChat(
  model: string,
  messages: { role: string; content: string }[],
  maxTokens: number,
): Promise<ChatResult> {
  const res = await fetch(`${OLLAMA}/api/chat`, {
    method: "POST",
    body: JSON.stringify({
      model,
      messages,
      stream: false,
      think: false,
      options: { num_predict: maxTokens },
    }),
  })
  if (!res.ok) throw new Error(`Ollama error ${res.status}: ${await res.text()}`)
  const data = await res.json()
  const evalDuration = data.eval_duration ?? 1
  return {
    content: data.message?.content ?? "",
    eval_count: data.eval_count ?? 0,
    total_duration_ms: Math.round((data.total_duration ?? 0) / 1e6),
    prompt_eval_ms: Math.round((data.prompt_eval_duration ?? 0) / 1e6),
    eval_ms: Math.round(evalDuration / 1e6),
    tok_per_sec: Math.round((data.eval_count ?? 0) / (evalDuration / 1e9)),
  }
}

// ─── Test 1: Baseline ─────────────────────────────────────────────────────────

async function testBaseline(prompt: string) {
  console.log("\n═══ TEST 1: Baseline (target only) ═══\n")
  const t0 = performance.now()
  const result = await ollamaChat(TARGET_MODEL, [{ role: "user", content: prompt }], MAX_TOKENS)
  const wallMs = Math.round(performance.now() - t0)
  console.log(`  Output: "${result.content}"\n`)
  console.log(`  Tokens: ${result.eval_count}  Wall: ${wallMs}ms  Speed: ${result.tok_per_sec} tok/s`)
  return { wallMs, tokens: result.eval_count, content: result.content, tokPerSec: result.tok_per_sec }
}

// ─── Test 2: Draft only ───────────────────────────────────────────────────────

async function testDraft(prompt: string) {
  console.log("\n═══ TEST 2: Draft only ═══\n")
  const t0 = performance.now()
  const result = await ollamaChat(DRAFT_MODEL, [{ role: "user", content: prompt }], MAX_TOKENS)
  const wallMs = Math.round(performance.now() - t0)
  console.log(`  Output: "${result.content}"\n`)
  console.log(`  Tokens: ${result.eval_count}  Wall: ${wallMs}ms  Speed: ${result.tok_per_sec} tok/s`)
  return { wallMs, tokens: result.eval_count, tokPerSec: result.tok_per_sec }
}

// ─── Test 3: Speculative with PARALLEL requests ──────────────────────────────

async function testSpeculativeParallel(prompt: string, K: number) {
  console.log(`\n═══ TEST 3: Speculative (K=${K}, parallel, relaxed match) ═══\n`)

  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let iterations = 0
  let totalMatched = 0
  let totalDraft = 0
  let firstTokenMs = 0
  const roundInfo: string[] = []

  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    iterations++
    const context = accumulated
      ? [...userMsg, { role: "assistant", content: accumulated }]
      : [...userMsg]

    // Fire BOTH draft and target in parallel from same context
    const [draftResult, targetResult] = await Promise.all([
      ollamaChat(DRAFT_MODEL, context, K),
      ollamaChat(TARGET_MODEL, context, K + 1),
    ])

    if (firstTokenMs === 0) firstTokenMs = Math.round(performance.now() - t0)

    const draftText = draftResult.content.trim()
    const targetText = targetResult.content.trim()

    if (!targetText) {
      roundInfo.push(`  R${iterations}: target empty → stop`)
      break
    }

    const { accepted, matchedChars, draftChars } = verifyRelaxed(draftText, targetText)
    const matchRate = draftChars > 0 ? Math.round((matchedChars / draftChars) * 100) : 0
    totalMatched += matchedChars
    totalDraft += draftChars

    const acceptedWords = tokenize(accepted).length
    roundInfo.push(
      `  R${iterations}: ` +
      `match=${matchedChars}/${draftChars}chars(${matchRate}%) ` +
      `accepted="${accepted.slice(0, 60)}" (+${acceptedWords}w)`,
    )

    if (accepted) {
      accumulated += accumulated ? " " + accepted : accepted
      completionTokens += acceptedWords
    } else {
      break
    }

    // Stop if generation is complete
    if (targetText.length <= 2 || (accumulated.endsWith(".") && completionTokens > 30)) break
  }

  const wallMs = Math.round(performance.now() - t0)
  const overallMatch = totalDraft > 0 ? Math.round((totalMatched / totalDraft) * 100) : 0
  const effectiveTps = completionTokens > 0 ? Math.round((completionTokens / wallMs) * 1000) : 0

  console.log(`  Output: "${accumulated}"\n`)
  for (const r of roundInfo) console.log(r)
  console.log()
  console.log(`  Iterations: ${iterations}  Tokens: ${completionTokens}  Wall: ${wallMs}ms`)
  console.log(`  TTFT: ${firstTokenMs}ms  Match: ${overallMatch}%  Effective: ${effectiveTps} tok/s`)

  return { wallMs, tokens: completionTokens, matchRate: overallMatch, iterations, firstTokenMs, effectiveTps }
}

// ─── Test 4: Target-only with continuation (incremental) ─────────────────────

async function testTargetIncremental(prompt: string, chunkSize: number) {
  console.log(`\n═══ TEST 4: Target incremental (${chunkSize} tokens/round) ═══\n`)

  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let iterations = 0
  let firstTokenMs = 0

  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    iterations++
    const context = accumulated
      ? [...userMsg, { role: "assistant", content: accumulated }]
      : [...userMsg]

    const result = await ollamaChat(TARGET_MODEL, context, chunkSize)
    if (firstTokenMs === 0) firstTokenMs = Math.round(performance.now() - t0)

    const text = result.content.trim()
    if (!text) break

    accumulated += accumulated ? " " + text : text
    completionTokens += tokenize(text).length

    if (accumulated.endsWith(".") && completionTokens > 30) break
  }

  const wallMs = Math.round(performance.now() - t0)
  const effectiveTps = completionTokens > 0 ? Math.round((completionTokens / wallMs) * 1000) : 0

  console.log(`  Output: "${accumulated}"\n`)
  console.log(`  Iterations: ${iterations}  Tokens: ${completionTokens}  Wall: ${wallMs}ms`)
  console.log(`  TTFT: ${firstTokenMs}ms  Effective: ${effectiveTps} tok/s`)

  return { wallMs, tokens: completionTokens, iterations, firstTokenMs, effectiveTps }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

async function main() {
  console.log("═══════════════════════════════════════════════════════════════")
  console.log("  SPECULATIVE DECODING BENCHMARK v2")
  console.log(`  Draft:  ${DRAFT_MODEL} (0.8B, Q8_0)`)
  console.log(`  Target: ${TARGET_MODEL} (9.7B, Q4_K_M)`)
  console.log(`  Both in VRAM: ✓  Family: qwen35`)
  console.log("═══════════════════════════════════════════════════════════════")

  console.log("\n  Warming up...")
  await ollamaChat(DRAFT_MODEL, [{ role: "user", content: "hi" }], 5)
  await ollamaChat(TARGET_MODEL, [{ role: "user", content: "hi" }], 5)

  const baseline = await testBaseline(TEST_PROMPT)
  const draft = await testDraft(TEST_PROMPT)
  const specK6 = await testSpeculativeParallel(TEST_PROMPT, 6)
  const specK12 = await testSpeculativeParallel(TEST_PROMPT, 12)
  const incr = await testTargetIncremental(TEST_PROMPT, 20)

  console.log("\n═══════════════════════════════════════════════════════════════")
  console.log("  RESULTS")
  console.log("═══════════════════════════════════════════════════════════════")
  console.log()
  console.log("  Method                     Wall     Tok  Tok/s  TTFT    Match")
  console.log("  ─────────────────────────  ──────  ────  ─────  ──────  ─────")
  console.log(`  Baseline (target auto)     ${baseline.wallMs.toString().padStart(5)}ms  ${baseline.tokens.toString().padStart(3)}   ${baseline.tokPerSec.toString().padStart(4)}   N/A     N/A`)
  console.log(`  Draft only                 ${draft.wallMs.toString().padStart(5)}ms  ${draft.tokens.toString().padStart(3)}   ${draft.tokPerSec.toString().padStart(4)}   N/A     N/A`)
  console.log(`  Speculative K=6 parallel   ${specK6.wallMs.toString().padStart(5)}ms  ${specK6.tokens.toString().padStart(3)}   ${specK6.effectiveTps.toString().padStart(4)}   ${specK6.firstTokenMs}ms  ${specK6.matchRate}%`)
  console.log(`  Speculative K=12 parallel  ${specK12.wallMs.toString().padStart(5)}ms  ${specK12.tokens.toString().padStart(3)}   ${specK12.effectiveTps.toString().padStart(4)}   ${specK12.firstTokenMs}ms  ${specK12.matchRate}%`)
  console.log(`  Target incremental (20/r)  ${incr.wallMs.toString().padStart(5)}ms  ${incr.tokens.toString().padStart(3)}   ${incr.effectiveTps.toString().padStart(4)}   ${incr.firstTokenMs}ms  N/A`)

  console.log()
  const bestSpec = specK6.wallMs < specK12.wallMs ? specK6 : specK12
  const speedup = (baseline.wallMs / bestSpec.wallMs).toFixed(2)
  console.log(`  Best speculative speedup vs baseline: ${speedup}x`)
  console.log(`  Match rate: ${bestSpec.matchRate}% (text-based, same Qwen family)`)
  console.log()

  if (bestSpec.matchRate < 30) {
    console.log("  FINDINGS:")
    console.log("  - Text-based verify (word/char matching) has low accept rate (~7-20%)")
    console.log("  - Ollama sequential model switching adds significant overhead")
    console.log("  - Even with parallel requests, models share GPU compute bandwidth")
    console.log("  - Without logit access, text-based spec decoding is impractical")
    console.log()
    console.log("  RECOMMENDATIONS:")
    console.log("  1. Disable speculative for single-GPU Ollama (overhead > benefit)")
    console.log("  2. Use Ollama native speculative decoding if/when available")
    console.log("  3. For real speedup: use draft/target on separate GPUs")
    console.log("  4. Consider prompt caching instead (lower TTFT, simpler)")
  }

  console.log("\n═══════════════════════════════════════════════════════════════")
}

main().catch(console.error)
