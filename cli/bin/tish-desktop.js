#!/usr/bin/env node
// npx entrypoint for tish-desktop.
// Prefers the prebuilt native binary (`npm run build` → dist/tish-desktop);
// otherwise runs src/main.tish through `tish` with process+fs features.
// Args are forwarded either way.

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
const args = process.argv.slice(2)
const FEATURES = "process,fs"

let result
if (existsSync(nativeBin)) {
  result = spawnSync(nativeBin, args, { stdio: "inherit" })
} else {
  // Options for `tish` must come BEFORE the file (see `tish run --help`).
  result = spawnSync(
    "tish",
    ["run", "--feature", FEATURES, path.join(root, "src/main.tish"), ...args],
    { stdio: "inherit", cwd: root }
  )
  if (result.error && result.error.code === "ENOENT") {
    process.stderr.write(
      "tish-desktop: no prebuilt binary and `tish` is not installed. Run `npm run build` in cli/ first.\n"
    )
    process.exit(127)
  }
}
process.exit(result.status === null ? 1 : result.status)
