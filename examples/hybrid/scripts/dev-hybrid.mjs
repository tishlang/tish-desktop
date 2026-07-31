#!/usr/bin/env node
/**
 * SC4: Vite UI (lattish DemoPane) + apple shell (Native ‖ Webview).
 * Reuses dist/hybrid-shell-apple when present (pass --rebuild to force).
 */
import { spawn, spawnSync } from "node:child_process"
import { existsSync, statSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { createServer } from "node:net"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const shellBin = join(root, "dist", "hybrid-shell-apple")
const forceRebuild = process.argv.includes("--rebuild")
const port = 5177
const baseUrl = `http://localhost:${port}/`

let vite = null
let shell = null

function log(msg) {
  console.log(`[dev:hybrid] ${msg}`)
}

function shutdown(code) {
  try {
    if (shell && !shell.killed) shell.kill("SIGTERM")
  } catch {
    /* ignore */
  }
  try {
    if (vite && !vite.killed) vite.kill("SIGTERM")
  } catch {
    /* ignore */
  }
  process.exit(code ?? 0)
}

process.on("SIGINT", () => shutdown(0))
process.on("SIGTERM", () => shutdown(0))

function sleep(ms) {
  spawnSync(process.execPath, ["-e", `setTimeout(() => {}, ${ms})`], {
    stdio: "ignore",
  })
}

async function waitForHttp(url, timeoutMs = 60000) {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url, { signal: AbortSignal.timeout(1500) })
      if (res.status >= 200 && res.status < 500) return true
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 250))
  }
  return false
}

function shellNeedsRebuild() {
  if (forceRebuild) return true
  if (!existsSync(shellBin)) return true
  const shellMtime = statSync(shellBin).mtimeMs
  const watch = [
    join(root, "src", "main.apple.tish"),
    join(root, "app", "Shell.macos.tish"),
    join(root, "app", "NativePane.macos.tish"),
    join(root, "app", "demo.tish"),
    join(root, "package.apple.json"),
    join(root, "scripts", "build-shell-apple.mjs"),
  ]
  for (const p of watch) {
    if (existsSync(p) && statSync(p).mtimeMs > shellMtime) return true
  }
  return false
}

function buildAppleShell() {
  log("building apple shell → dist/hybrid-shell-apple")
  const r = spawnSync(process.execPath, [join(root, "scripts", "build-shell-apple.mjs")], {
    cwd: root,
    stdio: "inherit",
  })
  if (r.status !== 0) {
    log(`apple shell build failed (exit ${r.status ?? 1})`)
    shutdown(r.status ?? 1)
  }
  if (!existsSync(shellBin)) {
    log("apple shell binary missing after build")
    shutdown(1)
  }
}

function portFree(p) {
  return new Promise((resolve) => {
    const s = createServer()
    s.once("error", () => resolve(false))
    s.once("listening", () => {
      s.close(() => resolve(true))
    })
    s.listen(p, "127.0.0.1")
  })
}

async function main() {
  log("starting Vite :" + port)
  const free = await portFree(port)
  if (!free) {
    log(`port ${port} already in use — assuming Vite is up`)
  } else {
    vite = spawn(
      process.platform === "win32" ? "npx.cmd" : "npx",
      ["vite", "--port", String(port), "--strictPort"],
      {
        cwd: root,
        stdio: "inherit",
        shell: process.platform === "win32",
        env: {
          ...process.env,
          TISH_PLATFORM: "macos",
          TISH_SURFACE: "webview",
        },
      },
    )
    vite.on("exit", (code) => {
      if (code !== 0 && code !== null) {
        log(`Vite exited (${code})`)
        shutdown(code)
      }
    })
  }

  log(`waiting for ${baseUrl}`)
  if (!(await waitForHttp(baseUrl))) {
    log(`timed out waiting for ${baseUrl}`)
    shutdown(1)
  }
  log(`Vite ready at ${baseUrl}`)

  if (shellNeedsRebuild()) {
    if (existsSync(shellBin) && !forceRebuild) {
      log("sources newer than shell — rebuilding (or pass --rebuild)")
    }
    buildAppleShell()
  } else {
    log("using existing dist/hybrid-shell-apple (pass --rebuild to recompile)")
  }

  log("launching apple shell (Native ‖ Webview)")
  shell = spawn(shellBin, [], { cwd: root, stdio: "inherit" })
  shell.on("exit", (code) => {
    log(`apple shell exited (code=${code ?? 0})`)
    shutdown(code ?? 0)
  })
}

main().catch((err) => {
  console.error("[dev:hybrid]", err)
  shutdown(1)
})
