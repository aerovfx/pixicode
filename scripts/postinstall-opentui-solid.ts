/**
 * Apply jsx-runtime fix for @opentui/solid (missing jsx-runtime.js causes Bun to fail).
 * Runs after install so `npm run dev` / `bun run dev` works without the patch.
 */
import path from "path"
import { existsSync, readdirSync, readFileSync, writeFileSync } from "fs"

try {
  run()
} catch {
  // Ignore if path not found or write fails (e.g. read-only node_modules)
}

function run() {
  const root = process.cwd()
  const nm = path.join(root, "node_modules")
  const candidates: string[] = [path.join(nm, "@opentui", "solid")]
  const pixicodeNm = path.join(root, "packages", "pixicode", "node_modules", "@opentui", "solid")
  if (existsSync(pixicodeNm)) candidates.push(pixicodeNm)
  const bunDir = path.join(nm, ".bun")
  if (existsSync(bunDir)) {
    for (const name of readdirSync(bunDir)) {
      if (name.startsWith("@opentui+solid@")) {
        const dir = path.join(bunDir, name, "node_modules", "@opentui", "solid")
        if (existsSync(path.join(dir, "package.json"))) candidates.push(dir)
      }
    }
  }

  const jsxRuntimeJs = `import { createElement, createComponent, spread } from "."

function Fragment(props) {
  return props.children;
}
function jsx(type, props) {
  if (typeof type === "string") {
    const el = createElement(type);
    if (props) spread(el, props);
    return el;
  }
  return createComponent(type, props || {});
}
export { Fragment, jsx, jsx as jsxDEV, jsx as jsxs };
`

  const exportBlock = `    "./jsx-runtime": {
      "types": "./jsx-runtime.d.ts",
      "import": "./jsx-runtime.js",
      "default": "./jsx-runtime.js"
    },
    "./jsx-dev-runtime": {
      "types": "./jsx-runtime.d.ts",
      "import": "./jsx-runtime.js",
      "default": "./jsx-runtime.js"
    }`

  for (const dir of candidates) {
    const pkgPath = path.join(dir, "package.json")
    if (!existsSync(pkgPath)) continue

    const pkg = readFileSync(pkgPath, "utf-8")
    if (pkg.includes('"./jsx-runtime.js"')) continue

    const oldExport =
      '"./jsx-runtime": "./jsx-runtime.d.ts",\n    "./jsx-dev-runtime": "./jsx-runtime.d.ts"'
    if (!pkg.includes('"./jsx-runtime": "./jsx-runtime.d.ts"')) continue
    const newPkg = pkg.replace(oldExport, exportBlock)
    if (newPkg === pkg) continue

    writeFileSync(path.join(dir, "jsx-runtime.js"), jsxRuntimeJs)
    writeFileSync(pkgPath, newPkg)
    console.log("[postinstall] Applied @opentui/solid jsx-runtime fix at", dir)
    break
  }
}
