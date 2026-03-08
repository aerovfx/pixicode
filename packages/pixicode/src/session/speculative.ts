import type {
  LanguageModelV2,
  LanguageModelV2Prompt,
  LanguageModelV2StreamPart,
  LanguageModelV2FinishReason,
  LanguageModelV2Usage,
  LanguageModelV2Content,
} from "@ai-sdk/provider"
import { Log } from "@/util/log"

const TXT_ID = "txt-0"

/** Strip <think>...</think> so draft/target thinking does not affect verify or leak into output. */
function stripThinkTags(s: string): string {
  return s.replace(/<think>[\s\S]*?<\/think>/gi, "").trim()
}

function textFromContent(content: LanguageModelV2Content[]): string {
  const raw = content.filter((c) => c.type === "text").map((c) => (c as { text: string }).text).join("")
  return stripThinkTags(raw)
}

function appendAssistant(prompt: LanguageModelV2Prompt, text: string): LanguageModelV2Prompt {
  if (!text) return prompt
  return [
    ...prompt,
    {
      role: "assistant" as const,
      content: [{ type: "text" as const, text }],
    },
  ]
}

function tokenize(s: string): string[] {
  return s.split(/\s+/).filter(Boolean)
}

function verifyText(draft: string, target: string): string {
  const d = tokenize(draft)
  const t = tokenize(target)
  let n = 0
  while (n < d.length && n < t.length && d[n] === t[n]) n++
  if (n > 0) return t.slice(0, n).join(" ")
  if (t.length > 0) return t[0]
  return ""
}

export function createSpeculativeLanguageModel(
  draft: LanguageModelV2,
  target: LanguageModelV2,
  opts: { numDraftTokens: number },
): LanguageModelV2 {
  const numDraftTokens = Math.min(32, Math.max(1, opts.numDraftTokens))

  return {
    specificationVersion: target.specificationVersion,
    provider: target.provider,
    modelId: target.modelId,
    defaultObjectGenerationMode: target.defaultObjectGenerationMode,
    get supportedUrls() {
      return target.supportedUrls
    },
    get supportsStructuredOutputs() {
      return target.supportsStructuredOutputs
    },

    async doGenerate(options) {
      return target.doGenerate(options)
    },

    async doStream(options) {
      const prompt = options.prompt
      const maxOutputTokens = options.maxOutputTokens ?? 4096
      const abort = options.abortSignal
      const { tools: _t, toolChoice: _tc, ...noTools } = options as { tools?: unknown; toolChoice?: unknown; [k: string]: unknown }
      let accumulated = ""
      let completionTokens = 0
      let finishReason: LanguageModelV2FinishReason = "stop"
      const log = Log.create({ service: "session.speculative" })

      const stream = new ReadableStream<LanguageModelV2StreamPart>({
        async start(controller) {
          controller.enqueue({ type: "stream-start", warnings: [] })
          let textStarted = false
          let iterations = 0
          const emit = (chunk: string) => {
            if (!chunk) return
            if (!textStarted) {
              controller.enqueue({ type: "text-start", id: TXT_ID })
              textStarted = true
            }
            controller.enqueue({ type: "text-delta", id: TXT_ID, delta: chunk })
            accumulated += accumulated ? ` ${chunk}` : chunk
            completionTokens += tokenize(chunk).length
          }

          try {
            while (completionTokens < maxOutputTokens) {
              abort?.throwIfAborted()
              const promptWithAssistant = appendAssistant(prompt, accumulated)
              const draftResult = await draft.doGenerate({
                ...noTools,
                prompt: promptWithAssistant,
                maxOutputTokens: numDraftTokens,
                abortSignal: abort,
              })
              const draftText = textFromContent(draftResult.content)
              if (draftResult.finishReason === "stop" && !draftText.trim()) {
                finishReason = "stop"
                break
              }
              abort?.throwIfAborted()
              const withDraft = accumulated ? `${accumulated} ${draftText}` : draftText
              const targetResult = await target.doGenerate({
                ...noTools,
                prompt: appendAssistant(prompt, withDraft),
                maxOutputTokens: numDraftTokens + 1,
                abortSignal: abort,
              })
              const targetText = textFromContent(targetResult.content)
              const accepted = verifyText(draftText, targetText)
              const acceptedCount = tokenize(accepted).length
              iterations += 1
              log.debug("round", { iterations, acceptedCount, completionTokens: completionTokens + acceptedCount })
              emit(accepted)
              if (targetResult.finishReason === "stop") {
                finishReason = "stop"
                break
              }
            }

          } catch (err) {
            finishReason = "error"
            if (err instanceof Error) controller.enqueue({ type: "error", error: err })
          }

          if (textStarted) controller.enqueue({ type: "text-end", id: TXT_ID })
          const usage: LanguageModelV2Usage = {
            inputTokens: undefined,
            outputTokens: completionTokens,
            totalTokens: undefined,
          }
          controller.enqueue({ type: "finish", finishReason, usage })
        },
      })

      return {
        stream,
        request: { body: undefined },
        response: { headers: {} },
      }
    },
  }
}
