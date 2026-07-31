#!/usr/bin/env node
/** Dual Tauri webview mode: Vite UI + multi-window shell. */
import { spawn } from "node:child_process"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const repoCli = join(root, "..", "..", "cli", "bin", "mode.js")

const child = spawn(process.execPath, [repoCli, "dev", "--example", "hybrid"], {
  cwd: join(root, "..", ".."),
  stdio: "inherit",
})
child.on("exit", (code) => process.exit(code ?? 0))
