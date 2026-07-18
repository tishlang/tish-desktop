# Upstream work tracking (unified app runtime)

Desktop is a **consumer + thin adapter**. Hard problems land in owning repos.

| Work | Repo | Status | Notes |
|------|------|--------|-------|
| Platform file resolve | [tish](https://github.com/tishlang/tish) | Landed (local) | `platform_resolve`, `--platform` / `--surface`, `tish resolve-id`, eval uses same cascade |
| Vite same resolve rules | tish (`vite-plugin-tish`) | Landed (local) | Calls `tish resolve-id`; golden test in `tish_compile/tests/platform_resolve_cli.rs` |
| Typed-native / `TISH_CHECK` | tish | Existing | Prefer typed shell; see docs/TYPED_SHELL.md |
| AppKit attach / embed | [tish-apple](https://github.com/tishlang/tish-apple) | Landed (local) | `macos.attach`, `outerHost` / `skipMainMenu` / `skipTimerPump` |
| WKWebView script bridge | tish-apple | **Open** | Need `WKUserContentController` + evaluate JS parity with [`bridge.js`](../packages/desktop-api/src/bridge.js). Blocks one-window hybrid. File issue on tish-apple when publishing. |
| BrokerCore + `state.*` | tish-desktop | Landed | `path` / `revision` / `source` contract |
| CapabilityProvider | tish-desktop | Landed | notification wrap + web stub; `webview.*` partial |

**Policy:** no dual-process hybrid, no desktop-local resolver fork, no default `TISH_NATIVE_OPT=0` in templates.

**License note:** tish-apple may pull Apple PIF / SDK-related constraints; redistributors of hybrid builds with `platform-apple` must review tish-apple LICENSE for their distribution channel.

**React-like UI** → **lattish**, not tish upstream.
