/**
 * Silence "Cannot remove nonexistent droppable/draggable/transformer" warnings
 * from @thisbeyond/solid-dnd when a sortable unmounts after its id was already
 * removed from the DnD context (e.g. closing a project in the sidebar).
 */
import path from "path"
import { existsSync, readdirSync, readFileSync, writeFileSync } from "fs"

try {
  run()
} catch {
  // Ignore if package not found or write fails
}

function run() {
  const root = process.cwd()
  const nm = path.join(root, "node_modules")
  const candidates: string[] = [path.join(nm, "@thisbeyond", "solid-dnd")]
  const bunDir = path.join(nm, ".bun")
  if (existsSync(bunDir)) {
    for (const name of readdirSync(bunDir)) {
      if (name.startsWith("@thisbeyond+solid-dnd@")) {
        const dir = path.join(bunDir, name, "node_modules", "@thisbeyond", "solid-dnd")
        if (existsSync(path.join(dir, "dist", "dev.jsx"))) candidates.push(dir)
      }
    }
  }

  const stripWarn = (s: string) => {
    if (!s.includes("Cannot remove nonexistent draggable")) return s
    return s
      .replace(
        /if \(!untrack\(\(\) => state\[type\]\[id\]\)\) \{\s*console\.warn\([\s\S]*?\);\s*return;\s*\}/g,
        "if (!untrack(() => state[type][id])) return;",
      )
      .replace(
        /if \(!untrack\(\(\) => state\[type\]\[id\]\["transformers"\]\[transformerId\]\)\) \{\s*console\.warn\([\s\S]*?\);\s*return;\s*\}/g,
        'if (!untrack(() => state[type][id]["transformers"][transformerId])) return;',
      )
      .replace(
        /if \(!untrack\(\(\) => state\.draggables\[id\]\)\) \{\s*console\.warn\([^)]+\);\s*return;\s*\}/g,
        "if (!untrack(() => state.draggables[id])) return;",
      )
      .replace(
        /if \(!untrack\(\(\) => state\.droppables\[id\]\)\) \{\s*console\.warn\([^)]+\);\s*return;\s*\}/g,
        "if (!untrack(() => state.droppables[id])) return;",
      )
  }

  for (const dir of candidates) {
    const devJsx = path.join(dir, "dist", "dev.jsx")
    const devJs = path.join(dir, "dist", "dev.js")
    if (!existsSync(devJsx)) continue
    for (const file of [devJsx, devJs]) {
      if (!existsSync(file)) continue
      const content = readFileSync(file, "utf-8")
      const next = stripWarn(content)
      if (next !== content) {
        writeFileSync(file, next)
        console.log("[postinstall] Silenced solid-dnd remove warnings in", path.relative(root, file))
      }
    }
    break
  }
}
