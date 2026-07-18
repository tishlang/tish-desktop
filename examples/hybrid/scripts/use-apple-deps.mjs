#!/usr/bin/env node
/** Swap rustDependencies to enable platform-apple + tish-macos for build:shell:apple. */
import { readFileSync, writeFileSync, copyFileSync, existsSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const pkgPath = join(root, "package.json")
const bakPath = join(root, "package.json.bak-apple")
const applePath = join(root, "package.apple.json")

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"))
const apple = JSON.parse(readFileSync(applePath, "utf8"))
if (!existsSync(bakPath)) {
  copyFileSync(pkgPath, bakPath)
}
pkg.tish = { ...pkg.tish, ...apple.tish }
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n")
console.log("hybrid: rustDependencies → platform-apple + tish-macos")
