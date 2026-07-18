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
| `native` | tish-apple (macOS/iOS) / future ms/lin | `--surface native` |
| `webview` | Tauri or WKWebView + bridge | `--surface webview` |
| `web` | Vite only | `--surface web` + `web-bridge.js` |

## State

- **`state.*`** — shared microfrontend memory + `state:changed` (BrokerCore)
  - `invoke("state.get|set|patch", { path, value })` → `{ ok, path, value, revision }`
  - event: `{ path, value, revision, source }`
- **`store.*`** — persisted KV (`store.json`) via Tauri plugin — unchanged

Dispatch order: shell `handle` → `state.*` → CapProviders → legacy modules.

## Caps

App code calls `invoke("notification.show", …)`. Desktop uses Tauri-backed CapProviders; pure web uses stubs (`code: "unsupported"` when unavailable).

## Platform files

Owned by **tish** (`--platform` / `--surface`, `tish resolve-id`). Desktop does not reimplement resolve. See [tish LANGUAGE.md](../../tish/docs/LANGUAGE.md) and [UPSTREAM.md](./UPSTREAM.md).

## Hybrid (v1)

Dual coordinated windows until WK script bridge lands in tish-apple:

```tish
import { run, createSurface, stateSet } from "cargo:tish_app"

createSurface({ id: "main", kind: "webview", url: "http://localhost:5173/" })
// Native chrome: macos.attach(Sidebar, { outerHost: true }) when platform-apple is enabled

run({ plugins: { notification: true, store: true } })
```

## Typed shell

Prefer typed params/returns in shell / native modules; build with `tish build --check warn`. Broker/`state.*` stay a dynamic JSON edge. See [TYPED_SHELL.md](./TYPED_SHELL.md).

## Lattish

Optional. React-like hooks/components/adapters live in **lattish**, not tish upstream. Core never depends on lattish. See [LATTISH.md](./LATTISH.md).
