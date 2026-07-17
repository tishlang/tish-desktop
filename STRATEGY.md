# Tish Desktop Strategy

Tish-first desktop runtime on **Tauri 2**. Shell Tish owns application logic; Tauri owns the OS event loop inside `cargo:tish_desktop`. UI is **lattish + tish-tailwind** in platform webviews. Dual entrypoints sync across a broker (`desktop/v1`).

## Ownership

| Concern | Owner |
|---------|--------|
| App config, handlers, extensions | Shell Tish |
| Event loop, webviews, plugins | Tauri 2 (host crate) |
| Presentation state | UI Tish (lattish per window) |
| Tray / dialogs / menus / deep links | Tauri plugins → `desktop.*` / broker |
| Custom domain (FS sandbox) | Host + optional Tish/Rust extensions |

## Dual entrypoints

- **Shell:** `tish build --target native --native-backend rust` — may import `cargo:tish_desktop`
- **UI:** Vite + `@tishlang/vite-plugin-tish` — pure Tish + lattish; talks via `window.__TISH_DESKTOP__`
- **Shared:** `packages/shared` — no native imports

## Broker / state (microfrontend)

UI state stays in the webview. Domain/OS truth stays on the shell/host. Sync is `invoke` / `listen` only — no shared lattish store across heaps.

| Pattern | Use |
|---------|-----|
| Command | UI needs capability result |
| Event | Shell pushes (`tick`, `fs:changed`, `tray:action`) |
| Snapshot | Rehydrate after reload |

Protocol version: **`desktop/v1`**.

## Multi-window

`createWindow` / `window.focus` / `window.close` / `window.list` via host. Events may include window labels; UI `getCurrentWindowLabel()`.

## Tauri plugins

Wired in host: `dialog`, `tray-icon`, `menu`, `deep-link`, `opener`, `single-instance`, `notification`. Broker commands also cover dock badge/label, window progress/attention/title/titleBarStyle/decorations/shadow/startDragging, tray tooltip/title, context `menu.context`, `notification.show|requestPermission|permissionState`, and `opener.open`. Windows default to macOS `titleBarStyle: "transparent"` (blends with window bg). Starter capabilities in `crates/tish_desktop/capabilities/default.json` (includes `remote.urls` for Vite `localhost` / `127.0.0.1`, plus `allow-desktop-*` / `notification:default` / window drag+chrome — required because Tauri 2 enforces ACL on remote origins).

## Extensions

- **Tish:** `package.json` → `tish.desktop` manifest (`id`, `shell`, `ui`, `commands`, `permissions`)
- **Rust:** Tauri plugins and/or `cargo:` Value-ABI modules (`tish_desktop_sample_ext`)
- Registry: `useExtensions` / `registerRustExtension`; namespaced commands; permission gate

## Security (MVP)

- Treat UI as untrusted
- FS only under allowlisted `fsRoot` (canonicalize, reject escapes)
- Read cap 256 KiB; list cap 2000 entries
- Release must not ship Vite `devUrl` (use dist / custom protocol)
- DevTools only in debug builds

## Performance

- Debounce `fs:changed` in UI (~150ms)
- Prefer host-native FS (serde → JSON once)
- Keyed list rows in file-browser

## Examples

- **`examples/basic`** — ping, tick, second window, shell handler, Rust sample ext
- **`examples/file-browser`** — sandboxed fixture browse + watch sync
- **`examples/native-chrome`** — dock badge, tray/status title, context menus, notifications, progress, attention

## Reuse

- Cluster A (dune / tish-audio): Vite HMR, bridge façades, Tauri plugins — flip ownership to shell Tish
- Cluster B (tish-drop): `rustDependencies` + `cargo:` entry
- Docs: `cargo:` is native-only; bindgen does not wrap Tauri builders

## Dry-run summary

Host embeds Tauri (not bare wry). FS watch via `notify` (not `tish:fs`). Extension registry from day one. macOS-first; Win/Linux follow-on.
