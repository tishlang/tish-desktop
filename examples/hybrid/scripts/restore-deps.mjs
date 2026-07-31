#!/usr/bin/env node
/** Restore package.json after build:shell:apple. */
import { copyFileSync, existsSync, unlinkSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const pkgPath = join(root, "package.json")
const bakPath = join(root, "package.json.bak-apple")
if (existsSync(bakPath)) {
  copyFileSync(bakPath, pkgPath)
  unlinkSync(bakPath)
  console.log("hybrid: package.json restored")
}
