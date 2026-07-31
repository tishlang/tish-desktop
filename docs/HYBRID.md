# Hybrid native + webview

Surfaces are microfrontends over BrokerCore **`state.*`**. Authoring uses **Surface** vocabulary (`createSurface`, `--surface`, `SurfaceKind`) — not `NativeView` / `WebView` (those collide with RN layout adapters).

## Two authoring layers

| Layer | Canonical API | When |
|-------|---------------|------|
| Shell registration | `createSurface({ kind: "native"\|"webview", id, url?, root? })` then `run()` | Top-level panes / windows (desktop) |
| In-tree embed | Host tag `<webview bridge id src onBridgeInvoke />` | Nested WK pane inside a native tree (macOS / iOS) |
| Parent → pane | `invoke("webview.load"\|"postMessage"\|"eval"\|"list", …)` | Same commands for Tauri panes and host WK (`surfaceId`) |

BYO apps need only these. Optional later lattish sugar (`NativeSurface` / `WebSurface`) may wrap them — never required by core. See [UNIFIED_APP.md](./UNIFIED_APP.md) · [LATTISH.md](./LATTISH.md).

## Modes (examples/hybrid)

| Mode | Scripts | Surfaces |
|------|---------|----------|
| **Multi-window (v1)** | `npm run dev:multi` · `build:shell` | Dual Tauri `createSurface({ kind: "webview" })` — chrome + main share `state.*` |
| **Native ‖ webview (SC4)** | `npm run dev:hybrid` · `build:shell:apple` | AppKit + nested WK + **Tauri outerHost** (clipboard/dialog/os plugins) |
| **Pure web** | `npm run dev:web` · `build:web` | Vite only — `installWebBridge` from `@tishlang/tish-app-api/web`, `Button.web` / `Sidebar.web` |

Dual-window does **not** satisfy SC4. SC4 is the apple shell path above — both surfaces visible in one window; both share BrokerCore `state.*`.

```bash
cd examples/hybrid
npm install
npm run check:tree          # Phase 2 file tree + script contracts
npm run build:shell         # dual-webview shell
npm run build:shell:apple   # SC4 native+webview (macOS + platform-apple)
npm run dev:multi           # Vite + dual-webview shell
npm run dev:hybrid          # Vite + native chrome + webview (macOS)
npm run build:web           # pure web profile
```

Doctor (plan name `tish app doctor` ≡ `mode doctor`):

```bash
node cli/bin/mode.js doctor --platform macos --surface webview --resolve ./Button
```

### Attach (tish-apple) — SC4

```tish
run({
  profile: "desktop",
  platformAttach: {
    apple: { outerHost: true, autoRunEventLoop: false },
  },
  plugins: { notification: true, store: true },
})
```

Enable Cargo feature `platform-apple` and path-depend tish-apple. CI builds `dist/hybrid-shell-apple` on macOS.

## Web → webview → native

1. `*.web.tish` + `npm run build:web` / `dev:web` (`--surface web`, `@tishlang/tish-app-api/web`)
2. Shell + `createSurface({ kind: "webview" })`; UI with `--surface webview` — `.web.tish` still resolves
3. `*.macos.tish` + `createSurface({ kind: "native", root })` + `platformAttach.apple` (`dev:hybrid`)

Platform resolve: `import { Button } from "./Button"` → `Button.webview.tish` / `Button.web.tish` / `Button.macos.tish`. Vite also reads `package.json` `tish.platform` / `tish.surface` when env/opts are unset.

## One-window nested WK (SC4 parallel panes)

`examples/hybrid` SC4 shows **Native ‖ Webview** in a `<split>`, with Tauri as **outerHost** so desktop plugins work:
- Left: `app/NativePane.macos.tish` (AppKit + `brokerInvoke` → CapProviders)
- Right: bridged WK → `ui/DemoPane.tish` (lattish / ui-theme; same plugin cmds)
- Companion: `extensions.html` — direct Tauri bridge
- Shared paths: `app/demo.tish`

Invoke routes through **`brokerInvoke`**; webview `state.set` hydrates the native pane.

```tish
import { brokerInvoke } from "cargo:tishlang_app"

<webview
  id="wk-main"
  bridge={true}
  src={"http://localhost:5177/"}
  onBridgeInvoke={(msg) => brokerInvoke(msg.cmd, msg.args)}
/>
```

**HMR:** editing platform files hot-reloads under Vite; changing `TISH_PLATFORM` / `TISH_SURFACE` requires a Vite restart.

## iOS (no Tauri)

All modes use **`ios.run(App)`** in one process with BrokerCore (`ios.invoke` / `state.*`).

**hello-ios** proves native ↔ webview parity. See [UNIFIED_APP.md](./UNIFIED_APP.md).

Parity demo: `npm run example:hello-ios`. Pure-native host hello: sibling tish-apple `examples/hello-ios`.
