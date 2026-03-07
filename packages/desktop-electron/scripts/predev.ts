import { $ } from "bun"

import { copyBinaryToSidecarFolder, getCurrentSidecar, windowsify } from "./utils"

await $`bun ./scripts/copy-icons.ts ${process.env.PIXICODE_CHANNEL ?? "dev"}`

const RUST_TARGET = Bun.env.RUST_TARGET

const sidecarConfig = getCurrentSidecar(RUST_TARGET)

const binaryPath = windowsify(`../pixicode/dist/${sidecarConfig.ocBinary}/bin/pixicode`)

await (sidecarConfig.ocBinary.includes("-baseline")
  ? $`cd ../pixicode && bun run build --single --baseline`
  : $`cd ../pixicode && bun run build --single`)

await copyBinaryToSidecarFolder(binaryPath, RUST_TARGET)
