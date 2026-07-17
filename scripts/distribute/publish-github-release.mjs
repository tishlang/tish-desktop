#!/usr/bin/env node
/** Create/update a GitHub Release from dist/release (requires gh + GITHUB_TOKEN). */
import { spawnSync } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..")
const tag = process.env.RELEASE_TAG || process.env.GITHUB_REF_NAME || "v0.1.0-draft"
const dist = path.join(root, "dist/release")

if (!fs.existsSync(dist)) {
  console.warn("[publish-github-release] no dist/release — run build-release first")
  process.exit(0)
}

const r = spawnSync(
  "gh",
  ["release", "create", tag, "--title", tag, "--notes", "Draft release", "--draft", ...listFiles(dist)],
  { cwd: root, stdio: "inherit", env: process.env }
)
if (r.error) {
  console.warn("[publish-github-release] gh not available — skip")
  process.exit(0)
}
process.exit(r.status ?? 0)

function listFiles(dir) {
  const out = []
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name)
    if (ent.isDirectory()) out.push(...listFiles(p))
    else out.push(p)
  }
  return out
}
