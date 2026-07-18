#!/usr/bin/env node
// npx entrypoint for tish-desktop.
// Prefers the prebuilt native binary (`npm run build` → dist/tish-desktop);
// otherwise runs src/main.tish through `tish` with process+fs features.
//
// Rewrite `--platform` / `--surface` → `--desk-platform` / `--desk-surface` and
// strip TISH_PLATFORM/TISH_SURFACE from the host env. Those tokens make
// `tish run` of this CLI exit before the command body runs; doctor passes the
// desk-* values through to `tish resolve-id` only.

import { spawnSync } from "node:child_process"
import { existsSync } from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.join(__dirname, "..")
const nativeBin = path.join(
  root,
  "dist",
  process.platform === "win32" ? "tish-desktop.exe" : "tish-desktop"
)
const FEATURES = "process,fs"

function rewritePlatformSurface(argv) {
  const env = { ...process.env }
  delete env.TISH_PLATFORM
  delete env.TISH_SURFACE

  const out = []
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (a === "--platform" && i + 1 < argv.length) {
      out.push("--desk-platform", argv[++i])
      continue
    }
    if (a === "--surface" && i + 1 < argv.length) {
      out.push("--desk-surface", argv[++i])
      continue
    }
    if (typeof a === "string" && a.startsWith("--platform=")) {
      out.push("--desk-platform", a.slice("--platform=".length))
      continue
    }
    if (typeof a === "string" && a.startsWith("--surface=")) {
      out.push("--desk-surface", a.slice("--surface=".length))
      continue
    }
    out.push(a)
  }
  return { args: out, env }
}

const rawArgs = process.argv.slice(2)
const { args, env } = rewritePlatformSurface(rawArgs)

let result
if (existsSync(nativeBin)) {
  result = spawnSync(nativeBin, rawArgs, { stdio: "inherit", env })
} else {
  result = spawnSync(
    "tish",
    ["run", "--feature", FEATURES, path.join(root, "src/main.tish"), ...args],
    { stdio: "inherit", cwd: root, env }
  )
  if (result.error && result.error.code === "ENOENT") {
    process.stderr.write(
      "tish-desktop: no prebuilt binary and `tish` is not installed. Run `npm run build` in cli/ first.\n"
    )
    process.exit(127)
  }
}
process.exit(result.status === null ? 1 : result.status)
