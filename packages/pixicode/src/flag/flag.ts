function truthy(key: string) {
  const value = process.env[key]?.toLowerCase()
  return value === "true" || value === "1"
}

function falsy(key: string) {
  const value = process.env[key]?.toLowerCase()
  return value === "false" || value === "0"
}

export namespace Flag {
  export const PIXICODE_AUTO_SHARE = truthy("PIXICODE_AUTO_SHARE")
  export const PIXICODE_GIT_BASH_PATH = process.env["PIXICODE_GIT_BASH_PATH"]
  export const PIXICODE_CONFIG = process.env["PIXICODE_CONFIG"]
  export declare const PIXICODE_TUI_CONFIG: string | undefined
  export declare const PIXICODE_CONFIG_DIR: string | undefined
  export const PIXICODE_CONFIG_CONTENT = process.env["PIXICODE_CONFIG_CONTENT"]
  export const PIXICODE_DISABLE_AUTOUPDATE = truthy("PIXICODE_DISABLE_AUTOUPDATE")
  export const PIXICODE_DISABLE_PRUNE = truthy("PIXICODE_DISABLE_PRUNE")
  export const PIXICODE_DISABLE_TERMINAL_TITLE = truthy("PIXICODE_DISABLE_TERMINAL_TITLE")
  export const PIXICODE_PERMISSION = process.env["PIXICODE_PERMISSION"]
  export const PIXICODE_DISABLE_DEFAULT_PLUGINS = truthy("PIXICODE_DISABLE_DEFAULT_PLUGINS")
  export const PIXICODE_DISABLE_LSP_DOWNLOAD = truthy("PIXICODE_DISABLE_LSP_DOWNLOAD")
  export const PIXICODE_ENABLE_EXPERIMENTAL_MODELS = truthy("PIXICODE_ENABLE_EXPERIMENTAL_MODELS")
  export const PIXICODE_DISABLE_AUTOCOMPACT = truthy("PIXICODE_DISABLE_AUTOCOMPACT")
  export const PIXICODE_DISABLE_MODELS_FETCH = truthy("PIXICODE_DISABLE_MODELS_FETCH")
  export const PIXICODE_DISABLE_CLAUDE_CODE = truthy("PIXICODE_DISABLE_CLAUDE_CODE")
  export const PIXICODE_DISABLE_CLAUDE_CODE_PROMPT =
    PIXICODE_DISABLE_CLAUDE_CODE || truthy("PIXICODE_DISABLE_CLAUDE_CODE_PROMPT")
  export const PIXICODE_DISABLE_CLAUDE_CODE_SKILLS =
    PIXICODE_DISABLE_CLAUDE_CODE || truthy("PIXICODE_DISABLE_CLAUDE_CODE_SKILLS")
  export const PIXICODE_DISABLE_EXTERNAL_SKILLS =
    PIXICODE_DISABLE_CLAUDE_CODE_SKILLS || truthy("PIXICODE_DISABLE_EXTERNAL_SKILLS")
  export declare const PIXICODE_DISABLE_PROJECT_CONFIG: boolean
  export const PIXICODE_FAKE_VCS = process.env["PIXICODE_FAKE_VCS"]
  export declare const PIXICODE_CLIENT: string
  export const PIXICODE_SERVER_PASSWORD = process.env["PIXICODE_SERVER_PASSWORD"]
  export const PIXICODE_SERVER_USERNAME = process.env["PIXICODE_SERVER_USERNAME"]
  export const PIXICODE_ENABLE_QUESTION_TOOL = truthy("PIXICODE_ENABLE_QUESTION_TOOL")

  // Experimental
  export const PIXICODE_EXPERIMENTAL = truthy("PIXICODE_EXPERIMENTAL")
  export const PIXICODE_EXPERIMENTAL_FILEWATCHER = truthy("PIXICODE_EXPERIMENTAL_FILEWATCHER")
  export const PIXICODE_EXPERIMENTAL_DISABLE_FILEWATCHER = truthy("PIXICODE_EXPERIMENTAL_DISABLE_FILEWATCHER")
  export const PIXICODE_EXPERIMENTAL_ICON_DISCOVERY =
    PIXICODE_EXPERIMENTAL || truthy("PIXICODE_EXPERIMENTAL_ICON_DISCOVERY")

  const copy = process.env["PIXICODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT"]
  export const PIXICODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT =
    copy === undefined ? process.platform === "win32" : truthy("PIXICODE_EXPERIMENTAL_DISABLE_COPY_ON_SELECT")
  export const PIXICODE_ENABLE_EXA =
    truthy("PIXICODE_ENABLE_EXA") || PIXICODE_EXPERIMENTAL || truthy("PIXICODE_EXPERIMENTAL_EXA")
  export const PIXICODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS = number("PIXICODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS")
  export const PIXICODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX = number("PIXICODE_EXPERIMENTAL_OUTPUT_TOKEN_MAX")
  export const PIXICODE_EXPERIMENTAL_OXFMT = PIXICODE_EXPERIMENTAL || truthy("PIXICODE_EXPERIMENTAL_OXFMT")
  export const PIXICODE_EXPERIMENTAL_LSP_TY = truthy("PIXICODE_EXPERIMENTAL_LSP_TY")
  export const PIXICODE_EXPERIMENTAL_LSP_TOOL = PIXICODE_EXPERIMENTAL || truthy("PIXICODE_EXPERIMENTAL_LSP_TOOL")
  export const PIXICODE_DISABLE_FILETIME_CHECK = truthy("PIXICODE_DISABLE_FILETIME_CHECK")
  export const PIXICODE_EXPERIMENTAL_PLAN_MODE = PIXICODE_EXPERIMENTAL || truthy("PIXICODE_EXPERIMENTAL_PLAN_MODE")
  export const PIXICODE_EXPERIMENTAL_MARKDOWN = !falsy("PIXICODE_EXPERIMENTAL_MARKDOWN")
  export const PIXICODE_MODELS_URL = process.env["PIXICODE_MODELS_URL"]
  export const PIXICODE_MODELS_PATH = process.env["PIXICODE_MODELS_PATH"]

  function number(key: string) {
    const value = process.env[key]
    if (!value) return undefined
    const parsed = Number(value)
    return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined
  }
}

// Dynamic getter for PIXICODE_DISABLE_PROJECT_CONFIG
// This must be evaluated at access time, not module load time,
// because external tooling may set this env var at runtime
Object.defineProperty(Flag, "PIXICODE_DISABLE_PROJECT_CONFIG", {
  get() {
    return truthy("PIXICODE_DISABLE_PROJECT_CONFIG")
  },
  enumerable: true,
  configurable: false,
})

// Dynamic getter for PIXICODE_TUI_CONFIG
// This must be evaluated at access time, not module load time,
// because tests and external tooling may set this env var at runtime
Object.defineProperty(Flag, "PIXICODE_TUI_CONFIG", {
  get() {
    return process.env["PIXICODE_TUI_CONFIG"]
  },
  enumerable: true,
  configurable: false,
})

// Dynamic getter for PIXICODE_CONFIG_DIR
// This must be evaluated at access time, not module load time,
// because external tooling may set this env var at runtime
Object.defineProperty(Flag, "PIXICODE_CONFIG_DIR", {
  get() {
    return process.env["PIXICODE_CONFIG_DIR"]
  },
  enumerable: true,
  configurable: false,
})

// Dynamic getter for PIXICODE_CLIENT
// This must be evaluated at access time, not module load time,
// because some commands override the client at runtime
Object.defineProperty(Flag, "PIXICODE_CLIENT", {
  get() {
    return process.env["PIXICODE_CLIENT"] ?? "cli"
  },
  enumerable: true,
  configurable: false,
})
