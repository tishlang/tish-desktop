#!/usr/bin/env node
/** Generate updater latest.json placeholder for tauri-plugin-updater. */
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..")
const outDir = path.join(root, "dist/release")
fs.mkdirSync(outDir, { recursive: true })
const latest = {
  version: process.env.RELEASE_VERSION || "0.1.0",
  notes: "Draft updater manifest — configure TAURI_SIGNING_PRIVATE_KEY and endpoint.",
  pub_date: new Date().toISOString(),
  platforms: {},
}
const out = path.join(outDir, "latest.json")
fs.writeFileSync(out, JSON.stringify(latest, null, 2))
console.log(`[publish-updater] wrote ${out}`)
