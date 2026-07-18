# Cross-platform parity (components + libraries)

Living matrix for the unified Tish app runtime. **Update this file when a cap, host tag, or package lands or regresses.**

Surfaces: `web` · `webview` (shell-hosted DOM) · `native` (OS widgets).  
Desktop OS: **macOS** · **Windows** · **Linux**.  
Mobile OS: **iOS** · **Android**.

**Authoring:** shell `createSurface({ kind })` + host tag `<webview bridge>` (BYO). Product names use **Surface** — not `NativeView` / `WebView`. See [HYBRID.md](./HYBRID.md) · [UNIFIED_APP.md](./UNIFIED_APP.md).

Status: **full** · **partial** · **stub** · **unsupported** · **missing** · **n/a**

Owners: [UPSTREAM.md](./UPSTREAM.md) · open gaps: [UPSTREAM_OPEN.md](./UPSTREAM_OPEN.md) · architecture: [UNIFIED_APP.md](./UNIFIED_APP.md)

---

## 1. Platforms & hosts

| Target | Repo / crate | Entry | Maturity |
|--------|--------------|-------|----------|
| Web (browser) | `tish-desktop` `@tish-desktop/desktop-api` `web-bridge.js` | Vite + `--surface web` | **full** (subset caps) |
| Desktop webview | `tish_desktop` / `tish_app` + Tauri | `cargo:tish_app` `run()` | **full** |
| macOS native | `tish-apple` `tish-macos` | `macos.run` / `macos.attach` | **full** |
| iOS native | `tish-apple` `tish-ios` | `ios.run` + staticlib | **partial** (vs AppKit) |
| Windows native | `tish-ms` | `attach_native` | **stub** (window + notify) |
| Linux native | `tish-lin` | `attach_native` | **stub** (window + notify) |
| Android native | `tish-android` | `attach_native` stub | **stub** (scaffold) |

Desktop Tauri covers macOS / Windows / Linux **webview** shells. Native attach on win/lin/android does **not** yet mean JSX host parity with AppKit/UIKit.

---

## 2. Library / package matrix

| Package | Layer | web | desktop webview | macos native | win/lin native | ios | android |
|---------|-------|-----|-----------------|--------------|----------------|-----|---------|
| `@tish-desktop/shared` | Protocol, `Platform`, events | full | full | full | full | full | stub facts |
| `@tish-desktop/app-api` | `invoke` / `listen` / `state.*` | full | full | via shell | via shell | via `tish:ios` | planned |
| `@tish-desktop/desktop-api` | `bridge.js` + `web-bridge.js` | full | full | — | — | WK `__TISH_APP__` | planned |
| `@tish-desktop/ui-theme` | Optional DOM UI | full | full | n/a | n/a | n/a | n/a |
| `@tishlang/lattish` | Optional React-like UI | full | full | optional | — | optional | planned |
| `@tishlang/lattish/adapters/rn` | RN-ish tags → DOM | full | full | **not** native | — | **not** native | missing |
| `cargo:tish_app` / `tish_desktop` | Shell + BrokerCore | — | full | attach | attach stub | — | attach stub |
| `tish_broker` | Tauri-free `state.*` | via JS | full | full | full | full | planned |
| `tish:macos` | AppKit host | — | — | full | — | — | — |
| `tish:ios` | UIKit host + broker | — | — | — | — | full | — |
| `tish-ms` / `tish-lin` / `tish-android` | Sibling native hosts | — | — | — | stub | — | stub |

**Rule:** core Cargo/npm packages never depend on lattish / ui-theme. Platform files (`.macos.tish`, `.ios.tish`, …) are the native UI adaptation layer.

---

## 3. Platform file resolve

Owned by **tish** (`--platform` / `--surface`, `tish resolve-id`). Cascade tokens:

| Token | Meaning |
|-------|---------|
| `web` | Pure browser |
| `webview` | DOM inside shell (falls back to `web`) |
| `macos` / `ios` / `windows` / `linux` / `android` | OS-native |
| `desktop` | macos \| windows \| linux |
| `mobile` | ios \| android |
| `native` | Any native surface |

Examples:

| Build | Cascade (for `./Button`) |
|-------|--------------------------|
| `--platform web` | `.web` → base |
| `--platform macos --surface webview` | `.webview` → `.web` → `.desktop` → base |
| `--platform macos --surface native` | `.macos` → `.desktop` → `.native` → base |
| `--platform ios --surface native` | `.ios` → `.mobile` → `.native` → base |
| `--platform android --surface native` | `.android` → `.mobile` → `.native` → base |
| `--platform windows --surface native` | `.windows` → `.desktop` → `.native` → base |

---

## 4. Capability matrix (`invoke`)

Legend applies per cell. Desktop columns = Tauri shell on that OS unless noted.  
`local_invoke` / WK without AppHandle: handlers + `state.*` + `notification.*` (+ apple dialogs on iOS); other prefixes → `{ code: "unsupported" }`.

### 4.1 `state.*` (BrokerCore — shared memory)

| Command | web | desktop (mac/win/lin) | ios | android |
|---------|-----|----------------------|-----|---------|
| `state.get` / `set` / `patch` / `keys` / `delete` | full | full | full | missing |
| `state.surfaces` | full | full | full | missing |
| event `state:changed` | full | full | full (→ WK) | missing |

### 4.2 `store.*` (persisted KV)

| Command | web | desktop | ios | android |
|---------|-----|---------|-----|---------|
| `store.get/set/delete/keys/clear` | unsupported | full | full† | missing |

†iOS: `NSUserDefaults` JSON blob per `path` (default `store.json`); same args as desktop.

### 4.3 `notification.*`

| Command | web | desktop | ios | android |
|---------|-----|---------|-----|---------|
| `permissionState` | partial | full | full | stub* |
| `requestPermission` | partial | full | full | stub* |
| `show` | partial | full | full | stub* |

\*android host stub may report `granted` / no-op until a real backend lands.

### 4.4 `dialog.*`

| Command | web | desktop | ios | android |
|---------|-----|---------|-----|---------|
| `dialog.message` | unsupported | full | full | missing |
| `dialog.confirm` / `ask` | unsupported | full | full | missing |
| `dialog.open` / `save` | unsupported | full | unsupported | missing |

### 4.5 `webview.*` (broker commands)

| Command | web | desktop | ios | android |
|---------|-----|---------|-----|---------|
| `webview.load` / `postMessage` / `list` / `eval` | unsupported | full† | full† | missing |

†One invoke surface: desktop Tauri panes (`createSurface({ kind: "webview" })`) **and** host WK panes (`<webview bridge id=…>`). Host helpers `macos`/`ios`.webviewEval / `webviewPostMessage` remain thin aliases of `webview.eval` / `webview.postMessage`. Args: `surfaceId` (aliases `label` / `id`); `postMessage` uses `channel`/`event` + `body`/`payload`; `load` accepts `url` and/or `html`.

### 4.6 Window / chrome / desktop-only

| Family | web | desktop | ios | android |
|--------|-----|---------|-----|---------|
| `window.*` | unsupported | full | n/a | missing |
| `tray.*` / `dock.*` / `menu.*` | unsupported | full | n/a | missing |
| `fs.*` / `clipboard.*` / `shell.*` / `os.*` | unsupported | full | unsupported | missing |
| `shortcut.*` / `autostart.*` / `updater.*` | unsupported | full | n/a | missing |
| `secrets.*` / `auth.*` | unsupported | full | unsupported | missing |
| `ping` | full | handlers | handlers | stub |

Apps **must** branch on `{ ok: false, code: "unsupported" }` — never assume desktop-only caps exist on mobile/web.

---

## 5. Native host component (JSX tag) matrix

Canonical names: `tish-apple-common` `canonical_host_tag`.  
Win/lin/android native hosts have **no** JSX tag table yet (stub window only).

| Tag | web / webview (DOM) | macos AppKit | ios UIKit | win/lin/android native |
|-----|---------------------|--------------|-----------|-------------------------|
| `column` / `div` / `section` | full | full | full | missing |
| `row` | full | full | full† | missing |
| `zstack` | full | full | missing | missing |
| `scrollable` | full | full | full | missing |
| `button` | full | full | full | missing |
| `text` (h1–h6, p, …) | full | full | full | missing |
| `textinput` / `password` | full | full | full | missing |
| `text_editor` / `markdown_text` | full | full | text_editor‡ | missing |
| `checkbox` / `toggler` | full | full | full | missing |
| `slider` / `radio` / `pick_list` | full | full | slider‡ | missing |
| `progress_bar` / `list` / `tooltip` | full | full | missing | missing |
| `image` | full | full | full | missing |
| `tabs` / `split` / `visual_effect` | full | full | tabs‡ | missing |
| `space` / `rule` | full | full | full | missing |
| `webview` (+ bridge) | n/a (is the surface) | full | full | missing |
| `sidebar_window` / `macos_window` | n/a | full | n/a | missing |
| `scene_view` / `card_art` | n/a | missing | full | missing |

†iOS `row` supports equal columns + `gap` / `columnGap`.  
‡iOS: `slider` (`UISlider`), `tabs` (`UISegmentedControl` + panes), `text_editor` (`UITextView`). Still missing: `markdown_text`, `radio`, `pick_list`, `split`, `visual_effect`.

**Lattish / ui-theme components** (Button, Card, Input, …) target **web/webview**. Native parity is via platform files (`Button.macos.tish`, `Button.ios.tish`), not by running lattish inside AppKit/UIKit.

---

## 6. Bridge contract

| Surface | Global | Invoke path |
|---------|--------|-------------|
| Desktop Tauri webview | `window.__TISH_APP__` (`bridge.js`) | IPC → handlers → `state.*` → CapProviders |
| macOS WK | `__TISH_APP__` (`bridge={true}`) | `onBridgeInvoke` → `brokerInvoke` / handlers |
| iOS WK | same bootstrap | `onBridgeInvoke` → `invoke` → `tish_broker` |
| Pure web | `web-bridge.js` | in-memory `state.*` + Notification stub |

Protocol string: `desktop/v1` (kept for client reuse across hosts).

---

## 7. Priority gaps (parity roadmap)

1. **iOS host tags (remaining)** — `radio` / `pick_list` / `progress_bar` / `split` / `zstack` / `markdown_text` (`slider` / `tabs` / `text_editor` / row `gap` landed)  
2. **Web `store.*` persistence** (optional localStorage) — iOS `store.*` landed via UserDefaults  
3. **ms/lin** — real native surface or document “webview-only on win/lin”  
4. **Android** — grow `tish-android` past attach stub (JNI / Compose / WebView host)  
5. **Lattish** — optional native adapters + optional `NativeSurface`/`WebSurface` sugar; prefer platform files + `createSurface` / host `<webview>` for v1  
6. **Example platform files** — `.windows` / `.linux` / `.android` fixtures for resolve CI  

---

## 8. How to update this doc

When landing a feature:

1. Flip the cell(s) in §2–§5.  
2. Note the owning repo in [UPSTREAM_OPEN.md](./UPSTREAM_OPEN.md) if upstream.  
3. Prefer a smoke path: `examples/hybrid`, `examples/hello-ios` (this repo; host from tish-apple), or a new android example.

Last reviewed: 2026-07-17.
