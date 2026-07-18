#!/usr/bin/env node
import { spawnSync } from "node:child_process"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")

function runNode(script) {
  const r = spawnSync(process.execPath, [join(root, "scripts", script)], {
    cwd: root,
    stdio: "inherit",
  })
  if (r.status !== 0) process.exit(r.status ?? 1)
}

runNode("use-apple-deps.mjs")
try {
  const env = {
    ...process.env,
    TISH_PLATFORM: "macos",
    TISH_SURFACE: "native",
  }
  const r = spawnSync(
    "tish",
    [
      "build",
      "--target",
      "native",
      "--native-backend",
      "rust",
      "--platform",
      "macos",
      "--surface",
      "native",
      "--check",
      "warn",
      "src/main.tish",
      "-o",
      "dist/hybrid-shell",
    ],
    { cwd: root, stdio: "inherit", env }
  )
  if (r.status !== 0) process.exit(r.status ?? 1)
} finally {
  runNode("restore-deps.mjs")
}
