#!/usr/bin/env bun
/**
 * Benchmark: Logprob-Based Speculative Decoding
 *
 * Hybrid approach:
 *   - Draft: native Ollama /api/chat (think:false) → fast, clean text
 *   - Target: OpenAI /v1/chat/completions (logprobs:true) → content + logprobs
 *   - Target model: edu-nothink (no RENDERER/PARSER → content in content field)
 *
 * Verification methods compared:
 *   A) Text-based word matching (old method)
 *   B) Logprob token-in-top-N matching (new method)
 *
 * Usage: bun scripts/bench-logprob-speculative.ts
 */

const OLLAMA = "http://localhost:11434"
const DRAFT_MODEL = "qwen3.5:0.8b" // native API with think:false
const TARGET_MODEL = "edu-nothink" // OpenAI API with logprobs
const TARGET_BASELINE = "edu-assistant:latest" // native API for baseline
const K = 8 // draft tokens per round
const MAX_TOKENS = 80
const TOP_LOGPROBS = 10
const LOGPROB_THRESHOLD = -3.0 // accept draft token if logprob > this in target's top-N

const PROMPTS = [
  { id: "P1", text: "What is a binary search tree? Explain briefly." },
  { id: "P2", text: "Explain the difference between TCP and UDP in networking." },
  { id: "P3", text: "What is dynamic programming? Give a brief explanation." },
]

// ─── Native Ollama /api/chat ─────────────────────────────────────────────────

interface NativeResult {
  content: string
  eval_count: number
  total_ms: number
  tok_s: number
}

async function nativeChat(
  model: string,
  msgs: { role: string; content: string }[],
  maxTok: number,
): Promise<NativeResult> {
  const r = await fetch(`${OLLAMA}/api/chat`, {
    method: "POST",
    body: JSON.stringify({ model, messages: msgs, stream: false, think: false, options: { num_predict: maxTok } }),
  })
  if (!r.ok) throw new Error(`Native ${r.status}: ${await r.text()}`)
  const d = await r.json()
  const evalNs = d.eval_duration ?? 1
  return {
    content: d.message?.content ?? "",
    eval_count: d.eval_count ?? 0,
    total_ms: Math.round((d.total_duration ?? 0) / 1e6),
    tok_s: Math.round((d.eval_count ?? 0) / (evalNs / 1e9)),
  }
}

// ─── OpenAI /v1/chat/completions with logprobs ──────────────────────────────

interface TokenLogprob {
  token: string
  logprob: number
  top_logprobs: { token: string; logprob: number }[]
}

interface OpenAIResult {
  content: string
  logprobs: TokenLogprob[]
  total_tokens: number
}

async function openaiChat(
  model: string,
  msgs: { role: string; content: string }[],
  maxTok: number,
): Promise<OpenAIResult> {
  const r = await fetch(`${OLLAMA}/v1/chat/completions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      model,
      messages: msgs,
      max_tokens: maxTok,
      logprobs: true,
      top_logprobs: TOP_LOGPROBS,
      temperature: 0.7,
    }),
  })
  if (!r.ok) throw new Error(`OpenAI ${r.status}: ${await r.text()}`)
  const d = await r.json()
  const choice = d.choices?.[0]
  const content = choice?.message?.content ?? ""
  const logprobs: TokenLogprob[] = choice?.logprobs?.content ?? []

  return {
    content,
    logprobs,
    total_tokens: d.usage?.total_tokens ?? 0,
  }
}

// ─── Strip <think>...</think> from content and logprobs ──────────────────────

function stripThinkTokens(result: OpenAIResult): OpenAIResult {
  // Find the index of </think> token in logprobs
  let contentStartIdx = 0
  for (let i = 0; i < result.logprobs.length; i++) {
    if (result.logprobs[i].token === "</think>") {
      contentStartIdx = i + 1
      // Skip the newline after </think>
      while (
        contentStartIdx < result.logprobs.length &&
        result.logprobs[contentStartIdx].token.trim() === ""
      ) {
        contentStartIdx++
      }
      break
    }
  }

  const contentLogprobs = result.logprobs.slice(contentStartIdx)
  // Reconstruct content from logprob tokens
  const content = contentLogprobs.map((lp) => lp.token).join("")

  return {
    content: content.trim(),
    logprobs: contentLogprobs,
    total_tokens: result.total_tokens,
  }
}

// ─── Tokenize text into words ────────────────────────────────────────────────

function tokenize(s: string): string[] {
  return s.split(/\s+/).filter(Boolean)
}

// ─── Verification: Text-based (old method) ───────────────────────────────────

function verifyTextBased(draftText: string, targetText: string): { accepted: string; matchRate: number } {
  const d = tokenize(draftText)
  const t = tokenize(targetText)
  let n = 0
  while (n < d.length && n < t.length && d[n] === t[n]) n++

  const matchRate = d.length > 0 ? Math.round((n / d.length) * 100) : 0

  if (n > 0) {
    const bonus = n < t.length ? " " + t[n] : ""
    return { accepted: t.slice(0, n).join(" ") + bonus, matchRate }
  }
  return { accepted: t[0] ?? "", matchRate }
}

// ─── Verification: Logprob-based (new method) ────────────────────────────────

function verifyLogprobBased(
  draftText: string,
  targetLogprobs: TokenLogprob[],
  threshold: number,
): { accepted: string; matchRate: number; details: string[] } {
  // Tokenize draft into subword tokens by reconstructing from target logprob boundaries
  // Since both models use the same tokenizer family, we can compare at token level
  const draftClean = draftText.trim()
  const details: string[] = []

  let accepted = ""
  let matched = 0
  let total = 0

  // Compare token by token using target's logprobs
  let draftPos = 0

  for (let i = 0; i < targetLogprobs.length && draftPos < draftClean.length; i++) {
    const targetToken = targetLogprobs[i].token
    const topTokens = targetLogprobs[i].top_logprobs

    // Extract the draft token at the same position (same length as target token)
    const draftToken = draftClean.slice(draftPos, draftPos + targetToken.length)

    if (!draftToken) break
    total++

    // Check 1: Exact match with target
    if (draftToken === targetToken) {
      matched++
      accepted += targetToken
      draftPos += targetToken.length
      details.push(`  [${i}] EXACT: "${draftToken}" == "${targetToken}"`)
      continue
    }

    // Check 2: Draft token is in target's top-N predictions
    const found = topTokens.find((t) => t.token === draftToken)
    if (found && found.logprob > threshold) {
      matched++
      accepted += draftToken // Use draft's version since it's acceptable
      draftPos += targetToken.length
      details.push(
        `  [${i}] SOFT:  "${draftToken}" in top-${TOP_LOGPROBS} (logp=${found.logprob.toFixed(2)})`,
      )
      continue
    }

    // Check 3: Draft token starts with target token or vice versa (partial match)
    if (targetToken.startsWith(draftToken) || draftToken.startsWith(targetToken)) {
      // Alignment issue — different tokenization. Use target's token and advance
      matched++
      accepted += targetToken
      draftPos += targetToken.length
      details.push(`  [${i}] ALIGN: "${draftToken}" ~ "${targetToken}"`)
      continue
    }

    // REJECT — draft diverges from target
    details.push(
      `  [${i}] REJECT: "${draftToken}" != "${targetToken}" (not in top-${TOP_LOGPROBS} or logp < ${threshold})`,
    )
    // Add target's correction token
    accepted += targetToken
    break
  }

  const matchRate = total > 0 ? Math.round((matched / total) * 100) : 0
  return { accepted: accepted.trim(), matchRate, details }
}

// ─── Benchmark: Baseline ─────────────────────────────────────────────────────

interface BenchResult {
  method: string
  promptId: string
  wallMs: number
  tokens: number
  tokS: number
  matchRate: number | null
  output: string
}

async function benchBaseline(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  const res = await nativeChat(TARGET_BASELINE, [{ role: "user", content: prompt }], MAX_TOKENS)
  const wall = Math.round(performance.now() - t0)
  return {
    method: "Baseline",
    promptId,
    wallMs: wall,
    tokens: res.eval_count,
    tokS: res.tok_s,
    matchRate: null,
    output: res.content,
  }
}

// ─── Benchmark: Speculative with text verification ───────────────────────────

async function benchSpecText(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let iterations = 0
  let totalMatched = 0
  let totalDraft = 0

  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    iterations++
    const ctx = accumulated ? [...userMsg, { role: "assistant", content: accumulated }] : [...userMsg]

    // Draft (native, fast) and Target (native, for text comparison)
    const [draftRes, targetRes] = await Promise.all([
      nativeChat(DRAFT_MODEL, ctx, K),
      nativeChat(TARGET_BASELINE, ctx, K + 1),
    ])

    const draftTxt = draftRes.content.trim()
    const targetTxt = targetRes.content.trim()
    if (!targetTxt) break

    const { accepted, matchRate } = verifyTextBased(draftTxt, targetTxt)
    totalDraft += tokenize(draftTxt).length
    totalMatched += Math.round((matchRate / 100) * tokenize(draftTxt).length)

    if (!accepted) break
    accumulated += accumulated ? " " + accepted : accepted
    completionTokens += tokenize(accepted).length

    if (targetTxt.length <= 3 || (accumulated.endsWith(".") && completionTokens > 30)) break
  }

  const wall = Math.round(performance.now() - t0)
  const overallMatch = totalDraft > 0 ? Math.round((totalMatched / totalDraft) * 100) : 0

  return {
    method: "Spec-Text",
    promptId,
    wallMs: wall,
    tokens: completionTokens,
    tokS: completionTokens > 0 && wall > 0 ? Math.round((completionTokens / wall) * 1000) : 0,
    matchRate: overallMatch,
    output: accumulated,
  }
}

// ─── Benchmark: Speculative with logprob verification ────────────────────────

async function benchSpecLogprob(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let iterations = 0
  let totalMatched = 0
  let totalTokens = 0
  const allDetails: string[] = []

  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    iterations++
    const ctx = accumulated ? [...userMsg, { role: "assistant", content: accumulated }] : [...userMsg]

    // Draft: native API (fast, think:false)
    // Target: OpenAI API (logprobs, edu-nothink model)
    const [draftRes, targetRes] = await Promise.all([
      nativeChat(DRAFT_MODEL, ctx, K),
      openaiChat(TARGET_MODEL, ctx, K + 4 + 1), // +4 for <think> tokens
    ])

    const draftTxt = draftRes.content.trim()
    if (!draftTxt) break

    // Strip <think>...</think> from target
    const stripped = stripThinkTokens(targetRes)
    if (!stripped.content || stripped.logprobs.length === 0) break

    const { accepted, matchRate, details } = verifyLogprobBased(draftTxt, stripped.logprobs, LOGPROB_THRESHOLD)
    allDetails.push(`--- Round ${iterations} (draft: "${draftTxt.slice(0, 50)}") ---`)
    allDetails.push(...details)

    totalTokens += Math.max(1, Math.ceil(draftTxt.length / 4)) // approximate token count
    totalMatched += Math.round((matchRate / 100) * Math.max(1, Math.ceil(draftTxt.length / 4)))

    if (!accepted) break
    accumulated += accumulated ? " " + accepted : accepted
    completionTokens += tokenize(accepted).length

    if (stripped.content.length <= 3 || (accumulated.endsWith(".") && completionTokens > 30)) break
  }

  const wall = Math.round(performance.now() - t0)
  const overallMatch = totalTokens > 0 ? Math.round((totalMatched / totalTokens) * 100) : 0

  // Print verification details
  console.log("\n    Logprob verification details:")
  for (const d of allDetails.slice(0, 30)) console.log(`    ${d}`)
  if (allDetails.length > 30) console.log(`    ... (${allDetails.length - 30} more lines)`)

  return {
    method: "Spec-Logprob",
    promptId,
    wallMs: wall,
    tokens: completionTokens,
    tokS: completionTokens > 0 && wall > 0 ? Math.round((completionTokens / wall) * 1000) : 0,
    matchRate: overallMatch,
    output: accumulated,
  }
}

// ─── Standalone: Compare verification methods on one prompt ──────────────────

async function compareVerification(prompt: string) {
  console.log("\n══════════════════════════════════════════════════════════════")
  console.log("  VERIFICATION METHOD COMPARISON (single generation)")
  console.log("══════════════════════════════════════════════════════════════\n")

  const userMsg = [{ role: "user", content: prompt }]

  // Generate from both models
  console.log("  Generating from draft and target...")
  const [draftRes, targetNative, targetLogprob] = await Promise.all([
    nativeChat(DRAFT_MODEL, userMsg, 30),
    nativeChat(TARGET_BASELINE, userMsg, 30),
    openaiChat(TARGET_MODEL, userMsg, 34), // +4 for think tokens
  ])

  const draftTxt = draftRes.content.trim()
  const targetTxt = targetNative.content.trim()
  const stripped = stripThinkTokens(targetLogprob)

  console.log(`\n  Draft output:    "${draftTxt.slice(0, 100)}"`)
  console.log(`  Target (native): "${targetTxt.slice(0, 100)}"`)
  console.log(`  Target (openai): "${stripped.content.slice(0, 100)}"`)

  // Method A: Text-based
  const textResult = verifyTextBased(draftTxt, targetTxt)
  console.log(`\n  [Text-Based] Match rate: ${textResult.matchRate}%`)
  console.log(`    Accepted: "${textResult.accepted.slice(0, 80)}"`)

  // Method B: Logprob-based
  const logResult = verifyLogprobBased(draftTxt, stripped.logprobs, LOGPROB_THRESHOLD)
  console.log(`\n  [Logprob-Based] Match rate: ${logResult.matchRate}%`)
  console.log(`    Accepted: "${logResult.accepted.slice(0, 80)}"`)
  for (const d of logResult.details) console.log(`    ${d}`)

  // Method C: Logprob-based with relaxed threshold
  const relaxedResult = verifyLogprobBased(draftTxt, stripped.logprobs, -5.0)
  console.log(`\n  [Logprob-Relaxed (-5.0)] Match rate: ${relaxedResult.matchRate}%`)
  console.log(`    Accepted: "${relaxedResult.accepted.slice(0, 80)}"`)

  console.log("\n  Target logprob tokens:")
  for (let i = 0; i < Math.min(15, stripped.logprobs.length); i++) {
    const lp = stripped.logprobs[i]
    const top3 = lp.top_logprobs
      .slice(0, 3)
      .map((t) => `${JSON.stringify(t.token)}(${t.logprob.toFixed(1)})`)
      .join(", ")
    console.log(`    [${i}] ${JSON.stringify(lp.token).padEnd(18)} logp=${lp.logprob.toFixed(2).padStart(7)}  top: ${top3}`)
  }
}

// ─── Main ────────────────────────────────────────────────────────────────────

async function main() {
  console.log("╔═══════════════════════════════════════════════════════════════════╗")
  console.log("║  LOGPROB-BASED SPECULATIVE DECODING BENCHMARK                    ║")
  console.log("╠═══════════════════════════════════════════════════════════════════╣")
  console.log(`║  Draft   : ${DRAFT_MODEL} (native /api/chat, think:false)`)
  console.log(`║  Target  : ${TARGET_MODEL} (OpenAI /v1/chat, logprobs:true)`)
  console.log(`║  Baseline: ${TARGET_BASELINE} (native /api/chat, think:false)`)
  console.log(`║  K=${K}  Max=${MAX_TOKENS}  TopN=${TOP_LOGPROBS}  Threshold=${LOGPROB_THRESHOLD}`)
  console.log("╚═══════════════════════════════════════════════════════════════════╝")

  // Warm up all models
  console.log("\n  Warming up models...")
  await nativeChat(DRAFT_MODEL, [{ role: "user", content: "hi" }], 3)
  await nativeChat(TARGET_BASELINE, [{ role: "user", content: "hi" }], 3)
  await openaiChat(TARGET_MODEL, [{ role: "user", content: "hi" }], 7)
  console.log("  Ready.\n")

  // Part 1: Compare verification methods on a single prompt
  await compareVerification(PROMPTS[0].text)

  // Part 2: Full benchmark
  console.log("\n\n══════════════════════════════════════════════════════════════")
  console.log("  FULL BENCHMARK: 3 Methods × 3 Prompts")
  console.log("══════════════════════════════════════════════════════════════\n")

  const results: BenchResult[] = []

  for (const p of PROMPTS) {
    console.log(`─── ${p.id}: "${p.text}" ───\n`)

    // Baseline
    process.stdout.write("  [Baseline]      running... ")
    const b = await benchBaseline(p.id, p.text)
    console.log(`${b.wallMs}ms, ${b.tokens} tok, ${b.tokS} tok/s`)
    console.log(`                → "${b.output.slice(0, 100)}..."\n`)
    results.push(b)

    // Speculative text-based
    process.stdout.write("  [Spec-Text]     running... ")
    const st = await benchSpecText(p.id, p.text)
    console.log(`${st.wallMs}ms, ${st.tokens} tok, ${st.tokS} tok/s, match=${st.matchRate}%`)
    console.log(`                → "${st.output.slice(0, 100)}..."\n`)
    results.push(st)

    // Speculative logprob-based
    process.stdout.write("  [Spec-Logprob]  running... ")
    const sl = await benchSpecLogprob(p.id, p.text)
    console.log(`\n  ${sl.wallMs}ms, ${sl.tokens} tok, ${sl.tokS} tok/s, match=${sl.matchRate}%`)
    console.log(`                → "${sl.output.slice(0, 100)}..."\n`)
    results.push(sl)
  }

  // ─── Summary ────────────────────────────────────────────────────────────

  console.log("\n╔══════════════════════════════════════════════════════════════════════════╗")
  console.log("║  RESULTS SUMMARY                                                        ║")
  console.log("╠══════════════════════════════════════════════════════════════════════════╣\n")

  console.log("  Prompt  Method          Wall(ms)  Tokens  Tok/s  Match  Speedup")
  console.log("  ──────  ──────────────  ────────  ──────  ─────  ─────  ───────")

  for (const p of PROMPTS) {
    const b = results.find((r) => r.promptId === p.id && r.method === "Baseline")!
    const st = results.find((r) => r.promptId === p.id && r.method === "Spec-Text")!
    const sl = results.find((r) => r.promptId === p.id && r.method === "Spec-Logprob")!

    const stSpeed = st.wallMs > 0 ? (b.wallMs / st.wallMs).toFixed(2) : "N/A"
    const slSpeed = sl.wallMs > 0 ? (b.wallMs / sl.wallMs).toFixed(2) : "N/A"

    console.log(
      `  ${p.id.padEnd(6)}  Baseline        ${b.wallMs.toString().padStart(7)}    ${b.tokens.toString().padStart(4)}   ${b.tokS.toString().padStart(4)}    —      —`,
    )
    console.log(
      `          Spec-Text      ${st.wallMs.toString().padStart(7)}    ${st.tokens.toString().padStart(4)}   ${st.tokS.toString().padStart(4)}   ${(st.matchRate + "%").padStart(4)}   ${stSpeed}x`,
    )
    console.log(
      `          Spec-Logprob   ${sl.wallMs.toString().padStart(7)}    ${sl.tokens.toString().padStart(4)}   ${sl.tokS.toString().padStart(4)}   ${(sl.matchRate + "%").padStart(4)}   ${slSpeed}x`,
    )
  }

  // Averages
  const baselines = results.filter((r) => r.method === "Baseline")
  const specTexts = results.filter((r) => r.method === "Spec-Text")
  const specLogs = results.filter((r) => r.method === "Spec-Logprob")

  const avg = (arr: BenchResult[], key: keyof BenchResult) =>
    Math.round(arr.reduce((a, r) => a + (r[key] as number), 0) / arr.length)

  const avgBWall = avg(baselines, "wallMs")
  const avgSTWall = avg(specTexts, "wallMs")
  const avgSLWall = avg(specLogs, "wallMs")
  const avgSTMatch = avg(specTexts, "matchRate")
  const avgSLMatch = avg(specLogs, "matchRate")

  console.log()
  console.log("  ── Averages ──────────────────────────────────────────")
  console.log(`  Baseline      : ${avgBWall}ms avg, ${avg(baselines, "tokS")} tok/s`)
  console.log(`  Spec-Text     : ${avgSTWall}ms avg, ${avg(specTexts, "tokS")} tok/s, ${avgSTMatch}% match`)
  console.log(`  Spec-Logprob  : ${avgSLWall}ms avg, ${avg(specLogs, "tokS")} tok/s, ${avgSLMatch}% match`)
  console.log()
  console.log(`  Text speedup  : ${(avgBWall / avgSTWall).toFixed(2)}x`)
  console.log(`  Logprob speedup: ${(avgBWall / avgSLWall).toFixed(2)}x`)
  console.log()

  if (avgSLMatch > avgSTMatch) {
    console.log(`  ✓ Logprob match rate (${avgSLMatch}%) > Text match rate (${avgSTMatch}%)`)
    console.log(`    Improvement: +${avgSLMatch - avgSTMatch} percentage points`)
  } else {
    console.log(`  ✗ Logprob match rate (${avgSLMatch}%) did not improve over text (${avgSTMatch}%)`)
  }

  console.log("\n╚══════════════════════════════════════════════════════════════════════════╝")
}

main().catch(console.error)
