#!/usr/bin/env node
/* eslint-env node */
/**
 * Dual-process dev: Vite UI + native Tish/Tauri shell.
 * Usage: node scripts/dev-example.mjs basic|file-browser|native-chrome [--rebuild]
 */
import { spawn, spawnSync } from "node:child_process"
import fs from "node:fs"
import http from "node:http"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, "..")
const args = process.argv.slice(2)
const name = args.find((a) => !a.startsWith("--")) || "basic"
const forceRebuild = args.includes("--rebuild")
const exampleDir = path.join(root, "examples", name)
const PORTS = { basic: 5173, "file-browser": 5174, "native-chrome": 5175 }
const SHELLS = {
  basic: "dist/basic-shell",
  "file-browser": "dist/file-browser-shell",
  "native-chrome": "dist/native-chrome-shell",
}
const port = PORTS[name] || 5173
const shellOut = SHELLS[name] || `dist/${name}-shell`
const shellPath = path.join(exampleDir, shellOut)

function resolveTish() {
  const fromPath = spawnSync("which", ["tish"], { encoding: "utf8" })
  if (fromPath.status === 0 && fromPath.stdout.trim()) {
    return fromPath.stdout.trim()
  }
  const home = process.env.HOME || ""
  const cargo = path.join(home, ".cargo", "bin", "tish")
  if (fs.existsSync(cargo)) return cargo
  return "tish"
}

function run(cmd, cmdArgs, opts = {}) {
  const child = spawn(cmd, cmdArgs, {
    stdio: "inherit",
    cwd: opts.cwd || exampleDir,
    env: { ...process.env, ...opts.env },
    shell: opts.shell ?? false,
  })
  child.on("error", (err) => {
    console.error(`[dev] failed to start ${cmd}:`, err.message)
    process.exit(1)
  })
  return child
}

function waitForHttp(url, timeoutMs = 60000) {
  const start = Date.now()
  return new Promise((resolve, reject) => {
    const tick = () => {
      const req = http.get(url, (res) => {
        res.resume()
        resolve()
      })
      req.on("error", () => {
        if (Date.now() - start > timeoutMs) {
          reject(new Error(`timed out waiting for ${url}`))
          return
        }
        setTimeout(tick, 250)
      })
    }
    tick()
  })
}

function newestMtimeMs(dir) {
  let newest = 0
  if (!fs.existsSync(dir)) return 0
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name)
    if (ent.isDirectory()) {
      if (ent.name === "target" || ent.name === "gen") continue
      newest = Math.max(newest, newestMtimeMs(p))
    } else {
      newest = Math.max(newest, fs.statSync(p).mtimeMs)
    }
  }
  return newest
}

function shellNeedsRebuild() {
  if (!fs.existsSync(shellPath)) return true
  const shellMtime = fs.statSync(shellPath).mtimeMs
  const hostSrc = path.join(root, "crates/tish_desktop/src")
  const hostToml = path.join(root, "crates/tish_desktop/Cargo.toml")
  const shellSrc = path.join(exampleDir, "src")
  const newest = Math.max(
    newestMtimeMs(hostSrc),
    fs.existsSync(hostToml) ? fs.statSync(hostToml).mtimeMs : 0,
    newestMtimeMs(shellSrc)
  )
  return newest > shellMtime
}

function buildShell(tishBin) {
  console.log(`[dev] building shell with ${tishBin} → ${shellOut}`)
  const result = spawnSync(
    tishBin,
    [
      "build",
      "--target",
      "native",
      "--native-backend",
      "rust",
      "src/main.tish",
      "-o",
      shellOut,
    ],
    { cwd: exampleDir, stdio: "inherit", env: process.env }
  )
  if (result.error) {
    console.error(`[dev] shell build spawn error:`, result.error.message)
    process.exit(1)
  }
  if (result.status !== 0) {
    console.error(`[dev] shell build failed (exit ${result.status})`)
    process.exit(result.status ?? 1)
  }
  if (!fs.existsSync(shellPath)) {
    console.error(`[dev] shell binary missing after build: ${shellPath}`)
    process.exit(1)
  }
}

console.log(`[dev] ${name} — starting Vite :${port}, then native desktop shell`)

// CSS for Vite (best-effort)
const css = spawnSync("tish", ["run", "--feature", "fs", "scripts/build-css.tish"], {
  cwd: exampleDir,
  stdio: "inherit",
  env: process.env,
})
if (css.status !== 0) {
  console.warn("[dev] build-css failed (continuing; styles may be stale)")
}

const vite = run("npx", ["vite", "--port", String(port), "--strictPort"])

const shutdownKids = []
const shutdown = () => {
  for (const c of shutdownKids) {
    try {
      c.kill("SIGTERM")
    } catch {
      /* ignore */
    }
  }
  try {
    vite.kill("SIGTERM")
  } catch {
    /* ignore */
  }
  process.exit(0)
}
process.on("SIGINT", shutdown)
process.on("SIGTERM", shutdown)

// Vite binds IPv6 localhost by default — use localhost, not 127.0.0.1.
try {
  await waitForHttp(`http://localhost:${port}/`)
  console.log(`[dev] Vite is ready at http://localhost:${port}/`)
} catch (e) {
  console.error(`[dev] ${e.message}`)
  vite.kill()
  process.exit(1)
}

/** Pre-transform the UI module graph so the webview isn't waiting on cold compiles. */
async function warmupVite(baseUrl) {
  const modHeaders = {
    Accept: "text/javascript",
    "Sec-Fetch-Dest": "script",
    "Sec-Fetch-Mode": "cors",
  }
  const fetchText = (url, headers = {}) =>
    new Promise((resolve, reject) => {
      const req = http.get(url, { headers }, (res) => {
        const chunks = []
        res.on("data", (c) => chunks.push(c))
        res.on("end", () => {
          if (res.statusCode && res.statusCode >= 400) {
            reject(new Error(`${url} → ${res.statusCode}`))
            return
          }
          resolve(Buffer.concat(chunks).toString("utf8"))
        })
      })
      req.on("error", reject)
    })

  const t0 = Date.now()
  await fetchText(`${baseUrl}/`)
  await fetchText(`${baseUrl}/assets/app.css`).catch(() => "")
  await fetchText(`${baseUrl}/bridge-boot.js`).catch(() => "")

  const seen = new Set()
  const queue = [`${baseUrl}/ui/main.tish`]
  while (queue.length) {
    const url = queue.shift()
    if (seen.has(url)) continue
    seen.add(url)
    let body
    try {
      body = await fetchText(url, modHeaders)
    } catch {
      continue
    }
    for (const m of body.matchAll(/from\s*["']([^"']+)["']/g)) {
      const spec = m[1]
      let next = null
      if (spec.startsWith("/")) next = `${baseUrl}${spec}`
      else if (spec.startsWith(".")) next = new URL(spec, url).href
      if (next && !seen.has(next)) queue.push(next)
    }
  }
  console.log(`[dev] warmed ${seen.size} Vite modules in ${Date.now() - t0}ms`)
}

await warmupVite(`http://localhost:${port}`)

const tishBin = resolveTish()
if (forceRebuild || shellNeedsRebuild()) {
  if (!forceRebuild && fs.existsSync(shellPath)) {
    console.log(`[dev] host/shell sources newer than ${shellOut} — rebuilding`)
  }
  buildShell(tishBin)
} else {
  console.log(`[dev] using existing shell ${shellOut} (pass --rebuild to recompile)`)
}

console.log(`[dev] launching desktop app: ${shellOut}`)
const app = run(shellPath, [], { cwd: exampleDir })
shutdownKids.push(app)

app.on("exit", (code, signal) => {
  console.log(`[dev] desktop app exited (code=${code}, signal=${signal})`)
  vite.kill()
  process.exit(code ?? 0)
})
