#!/usr/bin/env node
/** Assert hybrid file tree + layer contracts. */
import { existsSync, readFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const required = [
  "app/demo.tish",
  "app/Shell.tish",
  "app/Shell.macos.tish",
  "app/NativePane.macos.tish",
  "app/Button.macos.tish",
  "ui/DemoPane.tish",
  "ui/main.tish",
  "src/main.tish",
  "src/main.apple.tish",
  "scripts/build-css.tish",
  "ui/Extensions.tish",
  "extensions.html",
]
let failed = false
for (const rel of required) {
  const p = join(root, rel)
  if (!existsSync(p)) {
    console.error("missing:", rel)
    failed = true
  }
}
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"))
if (!pkg.dependencies?.lattish || !pkg.dependencies?.["@tish-desktop/ui-theme"]) {
  console.error("package.json must depend on lattish + @tish-desktop/ui-theme")
  failed = true
}
const hybrid = pkg.scripts?.["dev:hybrid"] ?? ""
if (!String(hybrid).includes("dev-hybrid") && !String(hybrid).includes("shell")) {
  console.error("dev:hybrid must start shell (got:", hybrid, ")")
  failed = true
}
const appleMain = readFileSync(join(root, "src/main.apple.tish"), "utf8")
if (!appleMain.includes('kind: "native"') || !appleMain.includes("Shell")) {
  console.error("main.apple.tish must create a native surface with Shell root")
  failed = true
}
if (!appleMain.includes("outerHost: true") || !appleMain.includes("clipboard: true")) {
  console.error("main.apple.tish must enable Tauri outerHost + clipboard plugin")
  failed = true
}
if (!appleMain.includes("tauri-ext") && !appleMain.includes("extensions.html")) {
  console.error("main.apple.tish must create a Tauri extensions webview surface")
  failed = true
}
const shell = readFileSync(join(root, "app/Shell.macos.tish"), "utf8")
if (!shell.includes("<split") || !shell.includes("webview")) {
  console.error("Shell.macos.tish must show Native ‖ Webview (<split> + webview)")
  failed = true
}
if (shell.includes("setMode")) {
  console.error("Shell.macos.tish should not toggle modes — both panes at once")
  failed = true
}
const native = readFileSync(join(root, "app/NativePane.macos.tish"), "utf8")
for (const needle of ["Increment", "toggler", "textinput", "demo.tish"]) {
  if (!native.includes(needle)) {
    console.error("NativePane.macos.tish must include", needle)
    failed = true
  }
}
const demoUi = readFileSync(join(root, "ui/DemoPane.tish"), "utf8")
for (const needle of ["lattish", "@tish-desktop/ui-theme", "Increment", "DEMO"]) {
  if (!demoUi.includes(needle)) {
    console.error("DemoPane.tish must include", needle)
    failed = true
  }
}
if (failed) process.exit(1)
console.log("hybrid tree + scripts ok")
