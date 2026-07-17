#!/usr/bin/env node
/* eslint-env node */
/**
 * Legacy entry — forwards to the Tish CLI.
 * Prefer: node cli/bin/tish-desktop.js dev --example <name>
 *         npm run example:<name>
 */
import { spawnSync } from "node:child_process"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "..")
const args = process.argv.slice(2)
const name = args.find((a) => !a.startsWith("--")) || "basic"
const rebuild = args.includes("--rebuild")
const cli = path.join(root, "cli/bin/tish-desktop.js")

const forwarded = ["dev", "--example", name]
if (rebuild) forwarded.push("--rebuild")

console.log(`[dev] legacy wrapper → node ${cli} ${forwarded.join(" ")}`)
const r = spawnSync(process.execPath, [cli, ...forwarded], {
  cwd: root,
  stdio: "inherit",
  env: process.env,
})
process.exit(r.status ?? 1)
