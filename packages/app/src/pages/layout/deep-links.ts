export const deepLinkEvent = "pixicode:deep-link"

export const parseDeepLink = (input: string) => {
  if (!input.startsWith("pixicode://")) return
  if (typeof URL.canParse === "function" && !URL.canParse(input)) return
  const url = (() => {
    try {
      return new URL(input)
    } catch {
      return undefined
    }
  })()
  if (!url) return
  if (url.hostname !== "open-project") return
  const directory = url.searchParams.get("directory")
  if (!directory) return
  return directory
}

export const collectOpenProjectDeepLinks = (urls: string[]) =>
  urls.map(parseDeepLink).filter((directory): directory is string => !!directory)

type PixiCodeWindow = Window & {
  __PIXICODE__?: {
    deepLinks?: string[]
  }
}

export const drainPendingDeepLinks = (target: PixiCodeWindow) => {
  const pending = target.__PIXICODE__?.deepLinks ?? []
  if (pending.length === 0) return []
  if (target.__PIXICODE__) target.__PIXICODE__.deepLinks = []
  return pending
}
