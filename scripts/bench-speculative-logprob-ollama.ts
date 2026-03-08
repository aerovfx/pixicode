#!/usr/bin/env bun
/**
 * Benchmark: Speculative với verify theo logit (Ollama native)
 *
 * Dùng chỉ Ollama /api/chat:
 *   - Draft: qwen3.5:0.8b, think:false, K tokens
 *   - Target: edu-assistant:latest, think:false, logprobs:true, top_logprobs:10, K+1 tokens
 *
 * So sánh: Baseline | Spec-Text (word match) | Spec-Logprob (draft token in target top-N, logp > threshold)
 *
 * Usage: bun scripts/bench-speculative-logprob-ollama.ts
 */

const OLLAMA = "http://localhost:11434"
const DRAFT = "qwen3.5:0.8b"
const TARGET = "edu-assistant:latest"
const K = 8
const MAX_TOKENS = 100
const TOP_LOGPROBS = 10
const LOGPROB_THRESHOLD = -4.0 // -3.0 stricter; -5.0 accepts more draft tokens

const PROMPTS = [
  { id: "P1", text: "What is a binary search tree? Explain briefly." },
  { id: "P2", text: "Write a Python function to check if a string is a palindrome." },
  { id: "P3", text: "Explain the difference between TCP and UDP in networking." },
]

// ─── Ollama /api/chat (no logprobs) ──────────────────────────────────────────

interface ChatResult {
  content: string
  eval_count: number
  total_ms: number
  prompt_ms: number
  eval_ms: number
  tok_s: number
}

async function chat(
  model: string,
  msgs: { role: string; content: string }[],
  maxTok: number,
): Promise<ChatResult> {
  const r = await fetch(`${OLLAMA}/api/chat`, {
    method: "POST",
    body: JSON.stringify({
      model,
      messages: msgs,
      stream: false,
      think: false,
      options: { num_predict: maxTok },
    }),
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

// ─── Ollama /api/chat with logprobs (target only) ───────────────────────────

interface TokenLogprob {
  token: string
  logprob: number
  top_logprobs?: { token: string; logprob: number }[]
}

interface ChatLogprobResult extends ChatResult {
  logprobs: TokenLogprob[]
}

async function chatWithLogprobs(
  model: string,
  msgs: { role: string; content: string }[],
  maxTok: number,
): Promise<ChatLogprobResult> {
  const r = await fetch(`${OLLAMA}/api/chat`, {
    method: "POST",
    body: JSON.stringify({
      model,
      messages: msgs,
      stream: false,
      think: false,
      logprobs: true,
      top_logprobs: TOP_LOGPROBS,
      options: { num_predict: maxTok },
    }),
  })
  if (!r.ok) throw new Error(`${r.status}: ${await r.text()}`)
  const d = await r.json()
  const evalNs = d.eval_duration ?? 1
  const logprobs: TokenLogprob[] = Array.isArray(d.logprobs) ? d.logprobs : []
  return {
    content: d.message?.content ?? "",
    eval_count: d.eval_count ?? 0,
    total_ms: Math.round((d.total_duration ?? 0) / 1e6),
    prompt_ms: Math.round((d.prompt_eval_duration ?? 0) / 1e6),
    eval_ms: Math.round(evalNs / 1e6),
    tok_s: Math.round((d.eval_count ?? 0) / (evalNs / 1e9)),
    logprobs,
  }
}

function tokenize(s: string): string[] {
  return s.split(/\s+/).filter(Boolean)
}

// ─── Verify: text-based (word match) ───────────────────────────────────────────

function verifyText(draft: string, target: string): { accepted: string; matchRate: number } {
  const d = tokenize(draft)
  const t = tokenize(target)
  let n = 0
  while (n < d.length && n < t.length && d[n] === t[n]) n++
  const matchRate = d.length > 0 ? Math.round((n / d.length) * 100) : 0
  if (n > 0) {
    const bonus = n < t.length ? " " + t[n] : ""
    return { accepted: t.slice(0, n).join(" ") + bonus, matchRate }
  }
  return { accepted: t[0] ?? "", matchRate }
}

// ─── Verify: logprob-based (draft token in target top-N, logp > threshold) ────

function verifyLogprob(
  draftText: string,
  targetLogprobs: TokenLogprob[],
  threshold: number,
): { accepted: string; matchRate: number } {
  const draftClean = draftText.trim()
  let accepted = ""
  let matched = 0
  let total = 0
  let draftPos = 0

  for (let i = 0; i < targetLogprobs.length && draftPos < draftClean.length; i++) {
    const targetToken = targetLogprobs[i].token
    const topTokens = targetLogprobs[i].top_logprobs ?? []

    const draftToken = draftClean.slice(draftPos, draftPos + targetToken.length)
    if (!draftToken) break
    total++

    if (draftToken === targetToken) {
      matched++
      accepted += targetToken
      draftPos += targetToken.length
      continue
    }

    const found = topTokens.find((t) => t.token === draftToken)
    if (found && found.logprob > threshold) {
      matched++
      accepted += draftToken
      draftPos += targetToken.length
      continue
    }

    if (targetToken.startsWith(draftToken) || draftToken.startsWith(targetToken)) {
      matched++
      accepted += targetToken
      draftPos += targetToken.length
      continue
    }

    accepted += targetToken
    break
  }

  const matchRate = total > 0 ? Math.round((matched / total) * 100) : 0
  return { accepted: accepted.trim(), matchRate }
}

// ─── Bench result ────────────────────────────────────────────────────────────

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

async function benchSpecText(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let totalMatched = 0
  let totalDraft = 0
  let ttft = 0
  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    const ctx = accumulated ? [...userMsg, { role: "assistant", content: accumulated }] : [...userMsg]
    const [draftRes, targetRes] = await Promise.all([chat(DRAFT, ctx, K), chat(TARGET, ctx, K + 1)])
    if (ttft === 0) ttft = Math.round(performance.now() - t0)

    const draftTxt = draftRes.content.trim()
    const targetTxt = targetRes.content.trim()
    if (!targetTxt) break

    const { accepted, matchRate } = verifyText(draftTxt, targetTxt)
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
    ttftMs: ttft,
    tokens: completionTokens,
    tokS: completionTokens > 0 && wall > 0 ? Math.round((completionTokens / wall) * 1000) : 0,
    matchRate: overallMatch,
    output: accumulated,
  }
}

async function benchSpecLogprob(promptId: string, prompt: string): Promise<BenchResult> {
  const t0 = performance.now()
  let accumulated = ""
  let completionTokens = 0
  let totalMatched = 0
  let totalTokens = 0
  let ttft = 0
  const userMsg = [{ role: "user", content: prompt }]

  while (completionTokens < MAX_TOKENS) {
    const ctx = accumulated ? [...userMsg, { role: "assistant", content: accumulated }] : [...userMsg]
    const [draftRes, targetRes] = await Promise.all([
      chat(DRAFT, ctx, K),
      chatWithLogprobs(TARGET, ctx, K + 1),
    ])
    if (ttft === 0) ttft = Math.round(performance.now() - t0)

    const draftTxt = draftRes.content.trim()
    if (!draftTxt) break
    if (!targetRes.logprobs?.length) break

    const { accepted, matchRate } = verifyLogprob(draftTxt, targetRes.logprobs, LOGPROB_THRESHOLD)
    const roundTokens = Math.max(1, targetRes.logprobs.length)
    totalTokens += roundTokens
    totalMatched += Math.round((matchRate / 100) * roundTokens)
    if (!accepted) break

    accumulated += accumulated ? " " + accepted : accepted
    completionTokens += tokenize(accepted).length
    const content = targetRes.content?.trim() ?? ""
    if (content.length <= 3 || (accumulated.endsWith(".") && completionTokens > 30)) break
  }

  const wall = Math.round(performance.now() - t0)
  const overallMatch = totalTokens > 0 ? Math.round((totalMatched / totalTokens) * 100) : 0
  return {
    method: "Spec-Logprob",
    promptId,
    wallMs: wall,
    ttftMs: ttft,
    tokens: completionTokens,
    tokS: completionTokens > 0 && wall > 0 ? Math.round((completionTokens / wall) * 1000) : 0,
    matchRate: overallMatch,
    output: accumulated,
  }
}

async function main() {
  console.log("╔════════════════════════════════════════════════════════════════════╗")
  console.log("║  SPECULATIVE + LOGPROB VERIFY (Ollama native /api/chat)           ║")
  console.log("╠════════════════════════════════════════════════════════════════════╣")
  console.log(`║  Draft  : ${DRAFT}`)
  console.log(`║  Target : ${TARGET} (logprobs: true, top_logprobs: ${TOP_LOGPROBS})`)
  console.log(`║  K=${K}  Max=${MAX_TOKENS}  threshold=${LOGPROB_THRESHOLD}`)
  console.log("╚════════════════════════════════════════════════════════════════════╝")

  console.log("\n  Warming up...")
  await chat(DRAFT, [{ role: "user", content: "hi" }], 3)
  await chat(TARGET, [{ role: "user", content: "hi" }], 3)
  await chatWithLogprobs(TARGET, [{ role: "user", content: "hi" }], 5)
  console.log("  Ready.\n")

  const results: BenchResult[] = []

  for (const p of PROMPTS) {
    console.log(`─── ${p.id}: "${p.text}" ───\n`)

    process.stdout.write("  [Baseline]     ")
    const b = await benchBaseline(p.id, p.text)
    console.log(`${b.wallMs}ms, ${b.tokens} tok, ${b.tokS} tok/s`)
    results.push(b)

    process.stdout.write("  [Spec-Text]    ")
    const st = await benchSpecText(p.id, p.text)
    console.log(`${st.wallMs}ms, ${st.tokens} tok, ${st.tokS} tok/s, match=${st.matchRate}%`)
    results.push(st)

    process.stdout.write("  [Spec-Logprob] ")
    const sl = await benchSpecLogprob(p.id, p.text)
    console.log(`${sl.wallMs}ms, ${sl.tokens} tok, ${sl.tokS} tok/s, match=${sl.matchRate}%`)
    results.push(sl)
    console.log("")
  }

  console.log("\n╔════════════════════════════════════════════════════════════════════════════════╗")
  console.log("║  SUMMARY                                                                       ║")
  console.log("╠════════════════════════════════════════════════════════════════════════════════╣")
  console.log("  Prompt  Method        Wall(ms)  TTFT(ms)  Tokens  Tok/s   Match  vs Baseline")
  console.log("  ──────  ────────────  ────────  ────────  ──────  ─────   ─────  ───────────")

  const baseByPrompt: Record<string, BenchResult> = {}
  for (const r of results) if (r.method === "Baseline") baseByPrompt[r.promptId] = r

  for (const p of PROMPTS) {
    const b = baseByPrompt[p.id]!
    const st = results.find((r) => r.promptId === p.id && r.method === "Spec-Text")!
    const sl = results.find((r) => r.promptId === p.id && r.method === "Spec-Logprob")!
    const speedupText = b.wallMs > 0 ? (b.wallMs / st.wallMs).toFixed(2) : "N/A"
    const speedupLog = b.wallMs > 0 ? (b.wallMs / sl.wallMs).toFixed(2) : "N/A"
    console.log(
      `  ${p.id.padEnd(6)}  Baseline      ${b.wallMs.toString().padStart(7)}   ${b.ttftMs.toString().padStart(7)}    ${b.tokens.toString().padStart(4)}   ${b.tokS.toString().padStart(4)}    —      —`,
    )
    console.log(
      `          Spec-Text     ${st.wallMs.toString().padStart(7)}   ${st.ttftMs.toString().padStart(7)}    ${st.tokens.toString().padStart(4)}   ${st.tokS.toString().padStart(4)}   ${(st.matchRate + "%").padStart(4)}   ${speedupText}x`,
    )
    console.log(
      `          Spec-Logprob  ${sl.wallMs.toString().padStart(7)}   ${sl.ttftMs.toString().padStart(7)}    ${sl.tokens.toString().padStart(4)}   ${sl.tokS.toString().padStart(4)}   ${(sl.matchRate + "%").padStart(4)}   ${speedupLog}x`,
    )
  }

  const base = results.filter((r) => r.method === "Baseline")
  const specText = results.filter((r) => r.method === "Spec-Text")
  const specLog = results.filter((r) => r.method === "Spec-Logprob")
  const avgBase = Math.round(base.reduce((a, r) => a + r.wallMs, 0) / base.length)
  const avgText = Math.round(specText.reduce((a, r) => a + r.wallMs, 0) / specText.length)
  const avgLog = Math.round(specLog.reduce((a, r) => a + r.wallMs, 0) / specLog.length)
  const matchText = Math.round(specText.reduce((a, r) => a + (r.matchRate ?? 0), 0) / specText.length)
  const matchLog = Math.round(specLog.reduce((a, r) => a + (r.matchRate ?? 0), 0) / specLog.length)

  console.log("\n  ── Averages ──")
  console.log(`  Baseline     : ${avgBase}ms`)
  console.log(`  Spec-Text     : ${avgText}ms  match ${matchText}%  speedup ${(avgBase / avgText).toFixed(2)}x`)
  console.log(`  Spec-Logprob  : ${avgLog}ms  match ${matchLog}%  speedup ${(avgBase / avgLog).toFixed(2)}x`)
  console.log("\n╚════════════════════════════════════════════════════════════════════════════════╝")
}

main().catch(console.error)
