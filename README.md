# tish-desktop

Tish-first desktop runtime on **Tauri 2**: shell Tish owns app logic; platform webviews host **lattish + tish-tailwind** UI. Broker protocol `desktop/v1` syncs the two heaps.

See [STRATEGY.md](./STRATEGY.md) for architecture, command inventory, OAuth URIs, security, and dry-run findings. See [docs/TISH_FIRST_DX.md](./docs/TISH_FIRST_DX.md) for the Tish-only example rules.

## Quick start

```bash
# From this repo
cargo check -p tish_desktop

# Create / run apps via the Tish CLI (preferred)
node cli/bin/tish-desktop.js help
node cli/bin/tish-desktop.js init my-app
node cli/bin/tish-desktop.js dev --example native-chrome

# Or use npm wrappers (examples call the CLI under the hood)
npm run example:basic
npm run example:file-browser
npm run example:native-chrome

# Force-rebuild the native shell after Rust changes
npm run example:native-chrome:rebuild
```

`example:*` is a **dual-process** flow: Vite for the webview UI, then the compiled Tish/Tauri shell. If you only see the Vite URL, check the `[dev] launching desktop app` log line.

Legacy Node orchestrator (fallback): `npm run example:basic --` with `dev:legacy` in each example.

## Create an app

```bash
# Scaffold (shell + UI + Vite aliases + package.json)
npx tish-desktop init my-app   # or: node cli/bin/tish-desktop.js init my-app
cd my-app
npm install
npx tish-desktop dev
npx tish-desktop build
```

Install the CLI globally later via npm (`@tish-desktop/cli`) or use the Rust `tish-desktop` launcher binary (PATH / `TISH_DESKTOP_CLI` / `npx` fallback).

## Layout

| Path | Role |
|------|------|
| `crates/tish_desktop` | Public `cargo:tish_desktop` Tauri host + Value ABI |
| `crates/tish_desktop_sample_ext` | Private sample Rust extension (not published) |
| `cli/` | `@tish-desktop/cli` — `init\|dev\|build\|info\|icon\|distribute` |
| `packages/shared` | Pure Tish helpers + event name constants |
| `packages/desktop-api` | UI `desktopHost` + JS bridge |
| `packages/ui-theme` | Design tokens + lattish primitives |
| `examples/basic` | Ping/tick + second window (plain styles) |
| `examples/file-browser` | Sandboxed FS browser (ui-theme) |
| `examples/native-chrome` | Full OS chrome / broker demo (ui-theme) |
| `scripts/distribute/` | Release build, sign, notarize, updater, GH release |
| `.github/workflows/` | CI + draft release / crates publish |

## Broker highlights

UI calls `invoke(cmd, args)` / `listen(event)`. Full table in [STRATEGY.md](./STRATEGY.md). Highlights:

- **Chrome:** dock, tray, menus, notifications, titlebar styles, window geometry
- **I/O:** clipboard, global shortcuts, dialogs (incl. folder picker), reveal/openPath
- **Persist:** store prefs, autostart, updater (configure `tauri.conf.json` endpoints + pubkey)
- **Auth:** keyring secrets + OAuth2 PKCE (`http://127.0.0.1:<port>/callback` or `tish-desktop://oauth/callback`)
- **Advanced:** `file-drop`, power sleep block, process, allowlisted `shell.exec` / `http.fetch`, print

## Distribution

```bash
npm run distribute:build      # platform release artifacts → dist/release/
npm run distribute:sign       # macOS codesign (needs env — see scripts/distribute/env.example)
npm run distribute:notarize
npm run distribute:updater    # publish latest.json for tauri-plugin-updater
npm run distribute:release    # build + GitHub release helper
```

**Store prerequisites (high level):**

| Target | Needs |
|--------|--------|
| macOS DMG / direct | Apple Developer ID, notarization credentials, entitlements |
| Mac App Store | App Store provisioning, MAS entitlements, manual approve workflow |
| Windows / MS Store | Code signing cert, MSIX packaging, Partner Center |
| Linux | AppImage/deb via package-linux; optional Flatpak stub under `distribute/flatpak/` |
| Updater | Tauri updater keypair, HTTPS `endpoints`, `plugins.updater.active` |

Draft GitHub Actions under `.github/workflows/release-*.yml` document required secrets in comments. CI also runs `format:check`, `lint:tish`, and crates.io dry-run.

### Production protocol

For release builds: set `frontendDist` to the real UI `dist/`, leave Vite localhost out of CSP/remote ACL for shipping configs, enable `bundle.active`, and fill updater `pubkey` + `endpoints`. Deep link / OAuth scheme `tish-desktop` is registered via `Info.plist` + `plugins.deep-link`.

## Tooling

```bash
npm run format          # @tishlang/tish-format
npm run format:check
npm run lint:tish       # @tishlang/tish-lint
npm run lint:rust       # cargo fmt --check + clippy -D warnings
```

## Requirements

- Rust toolchain
- `tish` CLI (`tish build --target native --native-backend rust`)
- Node 20+ for Vite examples / CLI shim
- Sibling checkouts of `tish`, `lattish`, `tish-tailwind` for local path aliases (examples)
