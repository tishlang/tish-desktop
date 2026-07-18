# Cross-device Tish app runtime

Umbrella repo: **tish-desktop**. Public shell entry: `cargo:tish_app` (alias of `cargo:tish_desktop`).

## Layers

| Layer | What | UI kit? |
|-------|------|---------|
| Optional packs | lattish, `@tish-desktop/ui-theme`, scaffold templates | Yes |
| App / BYO UI | Your components | Your choice |
| Core API | `@tish-desktop/app-api`, bridge / web-bridge, `@tish-desktop/shared` | **No** |
| Core Rust | BrokerCore, `state.*`, caps, `createSurface`, platform adapters | **No** |

## Surfaces

| Surface | Host | Build |
|---------|------|-------|
| `native` | tish-apple (macOS/iOS), tish-ms, tish-lin, tish-android | `--surface native` |
| `webview` | Tauri (desktop) or WKWebView + bridge | `--surface webview` |
| `web` | Vite only | `--surface web` + `@tish-desktop/app-api/web` |

**Authoring layers** (canonical BYO — no required JSX Surface components):

1. **Shell registration** — `createSurface({ kind: "native"|"webview"|"web", id, url?, root? })` then `run()` (desktop).
2. **In-tree embed** — host tag `<webview bridge … />` inside a native tree (macOS / iOS).

Use **Surface** vocabulary (`SurfaceKind`, `createSurface`, `--surface`). Do **not** name product APIs `NativeView` / `WebView`. Optional later lattish sugar: `NativeSurface` / `WebSurface` — see [LATTISH.md](./LATTISH.md).

| Mode | macOS | iOS (no Tauri) |
|------|-------|----------------|
| Full native | `createSurface(native)` / `macos.run` | `ios.run` host tags |
| Full webview | `createSurface(webview)` via Tauri | `ios.run` root `<webview bridge>` |
| Hybrid multi-window | Dual Tauri webviews + `state.*` | n/a |
| Hybrid SC4 | Native ‖ Webview (`dev:hybrid`) | AppKit `NativePane` + lattish/`ui-theme` `DemoPane` |

Details: [HYBRID.md](./HYBRID.md). **Parity matrix:** [PARITY.md](./PARITY.md).

## State

- **`state.*`** — shared microfrontend memory + `state:changed` (BrokerCore)
  - `invoke("state.get|set|patch", { path, value })` → `{ ok, path, value, revision }`
  - event: `{ path, value, revision, source }`
- **`store.*`** — persisted KV (`store.json`) via Tauri plugin — unchanged

Dispatch order: shell `handle` → `state.*` → CapProviders → legacy modules.

In-process (shell / WK bridge): `brokerInvoke(cmd, args)` — same handlers + `state.*` without Tauri IPC.

## Caps

App code calls `invoke("notification.show", …)`. Desktop CapProviders wrap `notification.*`, `dialog.*`, `store.*`, `webview.*`. Trait method is `supported()` (plan sketch name: `support`) returning `CapSupport` (`Full` | `Partial` | `Unsupported`). Pure web stubs return:

```json
{ "ok": false, "code": "unsupported", "capability": "tray", "platform": "web", "message": "…" }
```

**iOS profile:** BrokerCore is linked via `tish-ios` — no Tauri. Hosts may path-depend on the `tish_broker` crate only, not on `tish_desktop`.

## Pure web entry

```tish
import { installWebBridge } from "@tish-desktop/app-api/web"
// compat: @tish-desktop/app-api/web-bridge
installWebBridge()
mount(document.getElementById("root"), App)
```

Example: [`examples/hybrid`](../examples/hybrid) `ui/web-boot.js` + `dev:web`.

## Platform files

Owned by **tish** (`--platform` / `--surface`, `tish resolve-id`). Vite plugin also reads `package.json` `tish.platform` / `tish.surface` (or `tish.desktop.*`) when opts/env unset. See [UPSTREAM.md](./UPSTREAM.md).

## Hybrid

| Script | Proves |
|--------|--------|
| `dev:multi` / `build:shell` | Dual webviews share `state.*` (0b) |
| `dev:hybrid` / `build:shell:apple` | SC4 Native ‖ Webview panes share `demo.*` / `selection.docId` |
| `dev:web` / `build:web` | Pure web + `app-api/web` |

Scaffold BYO: `tish-desktop init --ui none` → `app/App.tish` + `Button.{tish,web,webview}.tish` + typed shell.

See [HYBRID.md](./HYBRID.md).

## Typed shell

Prefer typed params/returns in shell / native modules; build with `tish build --check warn`. Broker/`state.*` stay a dynamic JSON edge. See [TYPED_SHELL.md](./TYPED_SHELL.md).

## Doctor

`tish-desktop doctor` (plan alias: `tish app doctor`) prints platform/surface, resolve probe, and a Cap matrix sample (`Full` | `Unsupported`).

## Success criteria checklist (2026-07)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Platform resolve macos/ios + surfaces | DONE |
| 2 | Pure `--surface web` never native; webview reuses `.web` | DONE |
| 3 | Vite + `tish build` same resolve | DONE |
| 4 | `state.*` shared by native and webview (SC4) | DONE — `build:shell:apple` / `dev:hybrid` |
| 5 | `notification.show` Tauri + apple | DONE |
| 6 | tish-apple standalone; runtime depends for hybrid | DONE |
| 7 | Documented web→webview→native; `Detail` + `Sidebar.web` | DONE |
| 8 | Webview-only examples keep working | DONE — `examples/basic` `build:shell` in CI |
| 9 | BYO no lattish in deps | DONE |
| 10 | Core packages no edge to lattish/ui-theme | DONE |
| 11 | Typed shell + CI `--check warn` | DONE |
| 12 | Upstream-first resolve/attach/WK | DONE |

## Lattish

Optional. React-like hooks/components/adapters live in **lattish**, not tish upstream. Core never depends on lattish. See [LATTISH.md](./LATTISH.md).
