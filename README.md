# tish-desktop

Tish-first desktop runtime on **Tauri 2**: shell Tish owns app logic; platform webviews (WKWebView on macOS) host **lattish + tish-tailwind** UI. Broker protocol `desktop/v1` syncs the two heaps.

See [STRATEGY.md](./STRATEGY.md) for architecture, extensions, security, and dry-run findings.

## Quick start

```bash
# From this repo
cargo check -p tish_desktop

# Basic example — starts Vite :5173, then launches the native desktop window
npm run example:basic

# File browser — Vite :5174 + native desktop window (fixture/ sandbox)
npm run example:file-browser

# Native chrome — dock badge, tray status, context menus, notifications (:5175)
npm run example:native-chrome

# Force-rebuild the native shell (first run after Rust changes)
npm run example:basic:rebuild
npm run example:file-browser:rebuild
npm run example:native-chrome:rebuild
```

`example:*` is a **dual-process** flow: Vite for the webview UI, then the compiled Tish/Tauri shell (`.app` window). If you only see the Vite URL, the shell failed to start — check the `[dev] launching desktop app` log line.

## Layout

| Path | Role |
|------|------|
| `crates/tish_desktop` | `cargo:tish_desktop` Tauri host + Value ABI |
| `crates/tish_desktop_sample_ext` | Sample Rust extension |
| `packages/shared` | Pure Tish shared helpers |
| `packages/desktop-api` | UI `desktopHost` + JS bridge |
| `examples/basic` | Ping/tick + second window |
| `examples/file-browser` | Sandboxed FS browser |
| `examples/native-chrome` | Dock badge, tray, context menu, notifications |
| `extensions/*` | Tish extension manifests |

## Requirements

- Rust toolchain
- `tish` CLI (`tish build --target native --native-backend rust`)
- Node 20+ for Vite examples
