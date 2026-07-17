#!/usr/bin/env node
/**
 * Build production UI + shell for an example (default: basic).
 * Usage: node scripts/distribute/build-release.mjs [--example basic] [--platform Darwin|Windows|Linux]
 */
import { spawnSync } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..")
const args = process.argv.slice(2)
const exIdx = args.indexOf("--example")
const example = exIdx >= 0 ? args[exIdx + 1] : "basic"
const exampleDir = path.join(root, "examples", example)
const outDir = path.join(root, "dist/release", process.platform)

function run(cmd, cmdArgs, cwd) {
  console.log(`[distribute] ${cmd} ${cmdArgs.join(" ")}`)
  const r = spawnSync(cmd, cmdArgs, { cwd, stdio: "inherit", env: process.env, shell: true })
  if (r.status !== 0) process.exit(r.status ?? 1)
}

if (!fs.existsSync(exampleDir)) {
  console.error(`[distribute] missing example: ${exampleDir}`)
  process.exit(1)
}

fs.mkdirSync(outDir, { recursive: true })
run("npm", ["install"], exampleDir)
run("npm", ["run", "build:ui"], exampleDir)
run("npm", ["run", "build:shell"], exampleDir)

const shells = {
  basic: "dist/basic-shell",
  "file-browser": "dist/file-browser-shell",
  "native-chrome": "dist/native-chrome-shell",
}
const shellRel = shells[example] || `dist/${example}-shell`
const shellSrc = path.join(exampleDir, shellRel)
const dest = path.join(outDir, path.basename(shellRel))
if (fs.existsSync(shellSrc)) {
  fs.cpSync(shellSrc, dest, { recursive: true })
  console.log(`[distribute] copied ${shellSrc} → ${dest}`)
} else {
  console.warn(`[distribute] shell not found at ${shellSrc}`)
}
console.log(`[distribute] done → ${outDir}`)
