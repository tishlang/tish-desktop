# Tish Desktop Strategy

Cross-device **Tish app runtime** (repo name stays tish-desktop). Shell Tish owns application logic; **Tauri 2** owns the desktop webview event loop inside `cargo:tishlang_app` / `cargo:tishlang_desktop`. **Lattish is optional** — core is UI-kit agnostic. Surfaces (native / webview / web) sync via BrokerCore (`state.*` + `desktop/v1`). Apple native + iOS stay on **tish-apple**.

See [docs/UNIFIED_APP.md](./docs/UNIFIED_APP.md), [docs/UPSTREAM.md](./docs/UPSTREAM.md), [docs/TISH_FIRST_DX.md](./docs/TISH_FIRST_DX.md).

## Ownership

| Concern | Owner |
|---------|--------|
| App config, handlers, extensions | Shell Tish |
| Event loop, webviews, plugins | Tauri 2 (host crate) |
| Presentation state | UI Tish (lattish per window) |
| Tray / dialogs / menus / deep links / OS APIs | Tauri plugins → broker |
| Custom domain (FS sandbox) | Host + optional Tish/Rust extensions |

## Dual entrypoints

- **Shell:** `tish build --target native --native-backend rust` — may import `cargo:tishlang_desktop`
- **UI:** Vite + `@tishlang/vite-plugin-tish` — pure Tish + lattish; talks via `window.__TISH_DESKTOP__`
- **Shared:** `packages/shared` — no native imports
- **CLI:** `@tishlang/tish-desktop` (`mode init|dev|build|info|icon|distribute`) — product DX, not example app code

## Broker / state (microfrontend)

| API | Role |
|-----|------|
| **`state.*`** | Shared in-memory microfrontend state + `state:changed` (BrokerCore) |
| **`store.*`** | Persisted KV (`store.json`) — unchanged |
| Command / event | `invoke` / `listen` for caps and shell pushes |

Protocol version: **`desktop/v1`**. UI kit is irrelevant to the broker.

## Command inventory (`desktop/v1`)

Opt-in via `run({ plugins, shellAllow, httpAllow, auth })`. Host gates with `PluginFlags` → permissions + `try_state` / `ensure_*` before `PluginExt`.

### Core

| Command | Notes |
|---------|--------|
| `ping` | Protocol + timestamp |
| `window.list` / `focus` / `close` / `create` | Multi-window |
| `fs.list` / `readText` / `stat` / `watch` / `unwatch` | Sandboxed under `fsRoot` |
| `extensions.list` | Registered extensions |
| `dialog.message` | Blocking message |
| `dock.badge` / `dock.badgeLabel` | macOS dock |
| `window.title` / `titleBarStyle` / `decorations` / `shadow` / `startDragging` / `progress` / `attention` | Chrome |
| `tray.tooltip` / `tray.title` | Status item |
| `menu.context` / `menu.set` | Context + app menu → `menu:action` |
| `notification.*` | Permission + show |
| `opener.open` | Open URL |

### Phase 1 — I/O

| Command | Plugin flag |
|---------|-------------|
| `clipboard.readText` / `writeText` / `clear` / `readImage` / `writeImage` | `clipboard` |
| `shortcut.register` / `unregister` / `unregisterAll` / `isRegistered` | `globalShortcut` → `shortcut:{id}` |
| `dialog.open` / `save` / `confirm` / `ask` | `dialog` (`directory: true` → folder picker) |
| `shell.reveal` / `shell.openPath` | `opener` |

### Phase 2 — Window / OS

| Command | Plugin flag |
|---------|-------------|
| `window.minimize` / `maximize` / `unmaximize` / `fullscreen` / `isFullscreen` | — |
| `window.setPosition` / `getPosition` / `setSize` / `getSize` / `center` | — |
| `window.setAlwaysOnTop` / `setResizable` / `setFocus` / `print` / `reload` / `openDevtools` | — |
| `windowState.save` / `restore` / `enabled` | `windowState` |
| `os.info` / `os.theme` | `os` → `theme:changed` |
| `display.list` / `display.primary` | — |

### Phase 3 — Persist / update

| Command | Plugin flag |
|---------|-------------|
| `store.get` / `set` / `delete` / `keys` / `clear` | `store` (non-secret) |
| `autostart.isEnabled` / `enable` / `disable` | `autostart` |
| `updater.check` / `downloadAndInstall` / `currentVersion` | `updater` + `tauri.conf.json` endpoints |

### Phase 4–5 — Secrets / auth / advanced

| Command | Plugin flag / config |
|---------|----------------------|
| `secrets.set` / `get` / `delete` | `auth` (keyring service `com.tishlang.desktop`) |
| `auth.login` / `logout` / `status` / `getAccessToken` | `auth` + `auth.tokenHosts`; PKCE loopback or `tish-desktop://oauth/callback`; OIDC `nonce` when `openid` / `oidc:true`; optional `revocationEndpoint` on logout |
| `shell.exec` | `shell` + `shellAllow` deny-by-default |
| `http.fetch` | `http` + `httpAllow` |
| `power.preventSleep` / `allowSleep` | refcounted (`keepawake`) |
| `process.pid` / `exit` / `restart` | `process` |

## Events catalog

| Event | Payload (typical) |
|-------|-------------------|
| `tick` | `{ ts }` |
| `menu:action` / `tray:action` | `{ id, … }` |
| `deep-link` | URL / path |
| `fs:changed` | watch notify |
| `theme:changed` | theme string |
| `file-drop` | `{ paths, label }` |
| `shortcut:{id}` | `{ id }` |
| `auth:changed` / `auth:error` | session / error |
| `updater:progress` | download progress (when configured) |
| `state:changed` | `{ key, value }` shared microfrontend state |

Constants live in `packages/shared` (`EVT_*`).

## OAuth redirect URIs

Register both with your IdP:

1. **Loopback (dev / desktop MVP):** `http://127.0.0.1:<port>/callback` (host picks a free port)
2. **Custom scheme (installed apps):** `tish-desktop://oauth/callback` — declared in `Info.plist` `CFBundleURLTypes` and Tauri `plugins.deep-link.desktop.schemes`

Token HTTP hosts must be listed in `run({ auth: { tokenHosts: [...] } })`.

## Multi-window

`createWindow` / `window.focus` / `window.close` / `window.list` via host. Events may include window labels; UI `getCurrentWindowLabel()`. Windows default to macOS `titleBarStyle: "transparent"`.

## Tauri plugins

Wired when flags are true: `dialog`, `tray-icon`, `menu`, `deep-link`, `opener`, `single-instance`, `notification`, `clipboard-manager`, `global-shortcut`, `window-state`, `os`, `store`, `autostart`, `updater`, `process`. ACL in `crates/tish_desktop/capabilities/default.json` (includes Vite `remote.urls`).

**Hardening:** never call `PluginExt` without `try_state` / `ensure_*`; never merge pending plugin flags from `createWindow`.

## Public crate + CLI

- **`tishlang_desktop`** — publishable library (`repository` / `readme` metadata). Locally depends on **path** `tishlang_core` so `tish build` shares one `Value` type with the CLI. Crates.io publish rewrites to a versioned dep in [`.github/workflows/crates-release.yml`](./.github/workflows/crates-release.yml) — see [`docs/RELEASE.md`](./docs/RELEASE.md). Sample ext stays `publish = false`.
- **`mode` bin** — thin launcher → PATH / `TISH_DESKTOP_CLI` / `npx @tishlang/tish-desktop`
- **`@tishlang/tish-desktop`** (+ `desktop-api` / `shared` / `ui-theme`) — npm OIDC publish via [`.github/workflows/npm-release.yml`](./.github/workflows/npm-release.yml)

## UI theme

`packages/ui-theme`: `theme.css` + `theme-utilities.css` (maps `bg-background` etc. to CSS vars) + `fonts.css` + lattish primitives (`Button`, `Card`, `Section`, …). Adopted by `file-browser` and `native-chrome`; `basic` stays unthemed.

## Distribution

- Scripts: `scripts/distribute/*` (`npm run distribute:build|sign|notarize|updater|release`)
- GHA: `.github/workflows/ci.yml`, `crates-release.yml`, `npm-release.yml`, draft `release-*.yml` for app distribute
- Production: point `frontendDist` at built UI; do **not** ship Vite localhost; enable bundle + updater pubkey/endpoints when ready

## Extensions

- **Tish:** `package.json` → `tish.desktop` manifest
- **Rust:** Tauri plugins and/or `cargo:` Value-ABI (`tishlang_desktop_sample_ext` private)
- Registry: `useExtensions` / `registerRustExtension`; namespaced commands; permission gate

## Security (MVP)

- Treat UI as untrusted
- FS only under allowlisted `fsRoot`
- Read cap 256 KiB; list cap 2000 entries
- `shell.exec` / `http.fetch` deny-by-default allowlists
- Release must not ship Vite `devUrl`
- DevTools only in debug builds

## Tooling

| Tool | Command |
|------|---------|
| Tish format | `npm run format` / `format:check` |
| Tish lint | `npm run lint:tish` |
| Rust fmt/clippy | `npm run lint:rust` (`cargo fmt` + `clippy -D warnings`) |

## Examples

- **`examples/basic`** — ping, tick, second window, shell handler, Rust sample ext, `windowState`
- **`examples/file-browser`** — sandboxed fixture browse + watch + ui-theme
- **`examples/native-chrome`** — full broker demo surface (lattish)
- **`examples/byo-ui`** — shell + webview + custom DOM UI, **no lattish**
- **`examples/hybrid`** — dual-webview (`dev:multi`) and SC4 native+webview (`dev:hybrid` / `build:shell:apple`); platform files + `Detail` / `Sidebar.web`; pure web via `app-api/web`
- Completion of the unified host plan: both hybrid shapes first-class; doctor / Vite `package.json` platform opts / bare `app/` stubs — see [docs/UNIFIED_APP.md](./docs/UNIFIED_APP.md)

## Dry-run summary

Host embeds Tauri (not bare wry). Path deps blocked for crates.io → versioned + patch. Theme needs `theme-utilities.css`. `dialog.open` + `directory:true` uses folder picker. Notification/plugin crashes fixed via all-true `PluginFlags` default + `try_state`.
