# Tish Desktop Strategy

Tish-first desktop runtime on **Tauri 2**. Shell Tish owns application logic; Tauri owns the OS event loop inside `cargo:tish_desktop`. UI is **lattish + tish-tailwind** (+ optional `@tish-desktop/ui-theme`) in platform webviews. Dual entrypoints sync across a broker (`desktop/v1`).

See also [docs/TISH_FIRST_DX.md](./docs/TISH_FIRST_DX.md).

## Ownership

| Concern | Owner |
|---------|--------|
| App config, handlers, extensions | Shell Tish |
| Event loop, webviews, plugins | Tauri 2 (host crate) |
| Presentation state | UI Tish (lattish per window) |
| Tray / dialogs / menus / deep links / OS APIs | Tauri plugins → broker |
| Custom domain (FS sandbox) | Host + optional Tish/Rust extensions |

## Dual entrypoints

- **Shell:** `tish build --target native --native-backend rust` — may import `cargo:tish_desktop`
- **UI:** Vite + `@tishlang/vite-plugin-tish` — pure Tish + lattish; talks via `window.__TISH_DESKTOP__`
- **Shared:** `packages/shared` — no native imports
- **CLI:** `@tish-desktop/cli` (`tish-desktop init|dev|build|info|icon|distribute`) — product DX, not example app code

## Broker / state (microfrontend)

UI state stays in the webview. Domain/OS truth stays on the shell/host. Sync is `invoke` / `listen` only — no shared lattish store across heaps.

| Pattern | Use |
|---------|-----|
| Command | UI needs capability result |
| Event | Shell pushes (`tick`, `fs:changed`, `tray:action`, …) |
| Snapshot | Rehydrate after reload |

Protocol version: **`desktop/v1`**.

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
| `auth.login` / `logout` / `status` / `getAccessToken` | `auth` + `auth.tokenHosts`; PKCE loopback or `tish-desktop://oauth/callback` |
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

- **`tish_desktop`** — publishable library (`repository` / `readme` metadata). Locally depends on **path** `tishlang_core` so `tish build` shares one `Value` type with the CLI. For crates.io publish, switch to a versioned dep matching the CLI’s release train (do not mix path + crates.io `Value`s). Sample ext stays private.
- **`tish-desktop` bin** — thin launcher → PATH / `TISH_DESKTOP_CLI` / `npx @tish-desktop/cli`
- **`@tish-desktop/cli`** — Tish CLI (`cli/`) mirroring Tauri’s `init|dev|build|info|icon|distribute`

## UI theme

`packages/ui-theme`: `theme.css` + `theme-utilities.css` (maps `bg-background` etc. to CSS vars) + `fonts.css` + lattish primitives (`Button`, `Card`, `Section`, …). Adopted by `file-browser` and `native-chrome`; `basic` stays unthemed.

## Distribution

- Scripts: `scripts/distribute/*` (`npm run distribute:build|sign|notarize|updater|release`)
- Draft GHA: `.github/workflows/release-*.yml`, `ci.yml`, `crates-release.yml`
- Production: point `frontendDist` at built UI; do **not** ship Vite localhost; enable bundle + updater pubkey/endpoints when ready

## Extensions

- **Tish:** `package.json` → `tish.desktop` manifest
- **Rust:** Tauri plugins and/or `cargo:` Value-ABI (`tish_desktop_sample_ext` private)
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
- **`examples/native-chrome`** — full broker demo surface

## Dry-run summary

Host embeds Tauri (not bare wry). Path deps blocked for crates.io → versioned + patch. Theme needs `theme-utilities.css`. `dialog.open` + `directory:true` uses folder picker. Notification/plugin crashes fixed via all-true `PluginFlags` default + `try_state`.
