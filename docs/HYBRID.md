# Hybrid native + webview

Surfaces are microfrontends over BrokerCore **`state.*`**. Authoring uses **Surface** vocabulary (`createSurface`, `--surface`, `SurfaceKind`) — not `NativeView` / `WebView` (those collide with RN layout adapters).

## Two authoring layers

| Layer | Canonical API | When |
|-------|---------------|------|
| Shell registration | `createSurface({ kind: "native"\|"webview", id, url?, root? })` then `run()` | Top-level panes / windows (desktop) |
| In-tree embed | Host tag `<webview bridge id src onBridgeInvoke />` | Nested WK pane inside a native tree (macOS / iOS) |

BYO apps need only these. Optional later lattish sugar (`NativeSurface` / `WebSurface`) may wrap them — never required by core. See [UNIFIED_APP.md](./UNIFIED_APP.md) · [LATTISH.md](./LATTISH.md).

## Modes

| Mode | macOS | iOS (no Tauri) |
|------|-------|----------------|
| Full native | `createSurface({ kind: "native", root })` + attach, or `macos.run` | `ios.run(App)` with host tags only |
| Full webview | `createSurface({ kind: "webview", url })` (Tauri) | `ios.run` root `<webview bridge>` |
| Hybrid multi-window | Dual `createSurface({ kind: "webview" })` + optional native slot | n/a (single window) |
| Hybrid one-window | Native chrome + nested `<webview bridge>` | Same: native tags + nested `<webview bridge>` |

## v1 shipped — multi-window (desktop)

Dual coordinated Tauri windows sharing BrokerCore **`state.*`** (plan gate **0b**):

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

## One-window split (native + WK)

WK script bridge is in tish-apple. Hybrid `Sidebar.macos.tish` hosts a bridged detail pane and routes invoke through **`brokerInvoke`** (in-process BrokerCore: handlers + `state.*`).

```tish
import { brokerInvoke } from "cargo:tish_app"

<webview
  id="wk-main"
  bridge={true}
  src={url}
  onBridgeInvoke={(msg) => brokerInvoke(msg.cmd, msg.args)}
/>
```

Build shell with platform flags so `Sidebar.macos.tish` resolves:

```bash
npm run build:shell   # --platform macos --surface native
```

Enable Cargo feature `platform-apple` so `run({ platformAttach: { apple: { outerHost: true } } })` calls `tish_macos::attach_app` on queued roots. Dual coordinated Tauri webviews remain the default path without that feature. Dual-process hybrid is **out of policy**.

**HMR:** editing platform files hot-reloads under Vite; changing `TISH_PLATFORM` / `TISH_SURFACE` requires a Vite restart.

## iOS (no Tauri, no `createSurface` yet)

All modes use **`ios.run(App)`** in one process with BrokerCore (`ios.invoke` / `state.*`).

**hello-ios** proves native ↔ webview parity with an in-app switcher (same features on both panes; shared `demo.*` / `selection.docId` in the broker):

| Mode | Shape |
|------|--------|
| Full native | Host tags (`textinput`, `toggler`, `button`, …) + `invoke` |
| Full webview | Pane-filling `<webview bridge>` + `__TISH_APP__` → same caps |
| Hybrid chrome | Native **Native / Webview** mode bar wrapping either pane |

Parity demo (**tish-desktop**): `npm run example:hello-ios`. Pure-native host hello (**tish-apple**): `cd ../tish-apple/examples/hello-ios && npm run run`. Same `__TISH_APP__` contract as macOS WK; `state:changed` broadcasts to bridged panes.
