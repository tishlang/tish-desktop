# tish-desktop

Cross-device **Tish app runtime** (umbrella): shell Tish + BrokerCore (`state.*` / `desktop/v1`) + Tauri desktop webviews. **Lattish is optional** — use BYO UI or `init --template bare`. Apple native stays on **tish-apple**. Public shell entry: `cargo:tishlang_app` (alias of `cargo:tishlang_desktop`).

This GitHub repo is the **tish-mode** monorepo. Outside the monorepo, install the published **tish-desktop** packages — you do not need sibling checkouts.

See [docs/UNIFIED_APP.md](./docs/UNIFIED_APP.md), [docs/HYBRID.md](./docs/HYBRID.md) (native / hybrid / webview surfaces), [docs/PARITY.md](./docs/PARITY.md) (web · desktop · mobile matrix), [STRATEGY.md](./STRATEGY.md), [docs/UPSTREAM.md](./docs/UPSTREAM.md).

## Quick start (standalone)

```bash
# Requires: Rust, Node 20+, and the `tish` CLI (npm i -g @tishlang/tish)
npx @tishlang/tish-desktop init my-app
# or BYO UI (no lattish):
npx @tishlang/tish-desktop init my-app --ui none
cd my-app
npm install
npx mode dev
npx mode build
```

## Quick start (tish-mode monorepo)

```bash
# From this repo (path deps on sibling tish / lattish / …)
cargo check -p tishlang_desktop

node cli/bin/mode.js help
node cli/bin/mode.js init my-app
node cli/bin/mode.js doctor --platform macos --surface webview
node cli/bin/mode.js dev --example byo-ui

# Or use npm wrappers (examples call the CLI under the hood)
npm run example:basic
npm run example:file-browser
npm run example:native-chrome
npm run example:byo-ui
npm run example:hybrid
# Hybrid modes (from examples/hybrid):
#   npm run dev:multi    — dual Tauri webviews + state.*
#   npm run dev:hybrid   — SC4 Native/Webview switcher (macOS, hello-ios parity)
#   npm run dev:web      — pure web + @tishlang/tish-app-api/web

# Force-rebuild the native shell after Rust changes
npm run example:native-chrome:rebuild
```

`example:*` is a **dual-process** flow: Vite for the webview UI, then the compiled Tish/Tauri shell. If you only see the Vite URL, check the `[dev] launching desktop app` log line.

Legacy Node orchestrator (fallback): `npm run example:basic --` with `dev:legacy` in each example.

Install the CLI via npm (`npx @tishlang/tish-desktop` / `npm i -g @tishlang/tish-desktop`, bin: **`mode`**) or use the Rust `mode` launcher (`TISH_MODE_CLI` / `TISH_DESKTOP_CLI` / `npx` fallback). Package release steps: [docs/RELEASE.md](./docs/RELEASE.md).

## Layout

| Path | Role |
|------|------|
| `crates/tish_desktop` | `cargo:tishlang_desktop` host + BrokerCore + caps |
| `crates/tish_app` | `cargo:tishlang_app` alias re-export |
| `crates/tish_broker` | Standalone BrokerCore (published) |
| `crates/tish_desktop_sample_ext` | Private sample Rust extension (not published) |
| `cli/` | `@tishlang/tish-desktop` — `init\|dev\|build\|info\|icon\|distribute` |
| `packages/shared` | Protocol, events, `Platform` helpers (no UI kit) |
| `packages/desktop-api` / `app-api` | invoke / listen / `state.*` + bridge / web-bridge |
| `packages/ui-theme` | Optional design tokens + lattish primitives |
| `examples/basic` | Ping/tick + second window |
| `examples/file-browser` | Sandboxed FS browser (ui-theme) |
| `examples/native-chrome` | Full OS chrome / broker demo (lattish) |
| `examples/byo-ui` | Shell + webview + plain DOM (no lattish) |
| `examples/hybrid` | Dual webviews **and** SC4 Native/Webview switcher; `Detail` / `Sidebar.web`; platform `Button.*` |
| `scripts/distribute/` | Release build, sign, notarize, updater, GH release |
| `.github/workflows/` | CI (+ tish-apple checkout) + crates/npm release |

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

For release builds: set `frontendDist` to the real UI `dist/`, leave Vite localhost out of CSP/remote ACL for shipping configs, enable `bundle.active`, and fill updater `pubkey` + `endpoints`. Deep link / OAuth scheme `mode` is registered via `Info.plist` + `plugins.deep-link`.

## Tooling

```bash
npm run format          # @tishlang/tish-format
npm run format:check
npm run lint:tish       # @tishlang/tish-lint
npm run lint:rust       # cargo fmt --check + clippy -D warnings
```

## Requirements

- Rust toolchain
- `tish` CLI (`tish build --target native --native-backend rust`) — install via `@tishlang/tish`
- Node 20+ for Vite examples / CLI shim
- **Monorepo only:** sibling checkouts of `tish`, `lattish`, `tish-tailwind` for local path aliases (examples)
