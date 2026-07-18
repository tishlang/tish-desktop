# Hybrid native + webview

## v1 (shipped)

Dual coordinated windows sharing BrokerCore **`state.*`** (plan gate **0b**):

| Window | Role |
|--------|------|
| `chrome` | Sidebar stand-in (webview) — writes `selection.docId` |
| `main` | Detail (webview) — listens `state:changed` |
| `native-chrome` | Queued `createSurface({ kind: "native" })` for AppKit attach |

Example: [`examples/hybrid`](../examples/hybrid).

```bash
npm run example:hybrid
# or
node cli/bin/tish-desktop.js doctor --platform macos --surface webview --resolve ./Button
```

### Attach (tish-apple)

```tish
run({
  profile: "desktop",
  platformAttach: {
    apple: { outerHost: true, autoRunEventLoop: false },
  },
  plugins: { notification: true },
})
```

Shell (or adapter) calls **`macos.attach(App, { outerHost: true })`** so menus/timers are not clobbered. Enable Cargo feature `platform-apple` and path-depend tish-apple (CI checks this out).

## Web → webview → native

1. `*.web.tish` + `npm run build:web` / `dev:web` (`--surface web`, `web-bridge.js`)
2. Shell + `createSurface({ kind: "webview" })`; UI with `--surface webview` — `.web.tish` still resolves
3. `*.macos.tish` + `createSurface({ kind: "native", root })` + `macos.attach`

## One-window split

Blocked on WKWebView **script bridge** in tish-apple (parity with `bridge.js`). See [UPSTREAM.md](./UPSTREAM.md). Dual-process hybrid is **out of policy**.
