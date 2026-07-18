# Upstream work tracking (unified app runtime)

Desktop is a **consumer + thin adapter**. Hard problems land in owning repos.

| Work | Repo | Status | Notes |
|------|------|--------|-------|
| Platform file resolve | [tish](https://github.com/tishlang/tish) | Landed (local) | `platform_resolve`, `--platform` / `--surface` (incl. `android` + `mobile` family), `tish resolve-id` |
| Android host stub | [tish-android](../tish-android) | Stub | `attach_native` + notify placeholders; see [PARITY.md](./PARITY.md) |
| Vite same resolve rules | tish (`vite-plugin-tish`) | Landed (local) | Calls `tish resolve-id`; golden test in `tish_compile/tests/platform_resolve_cli.rs` |
| Typed-native / `TISH_CHECK` | tish | Existing | Prefer typed shell; see docs/TYPED_SHELL.md |
| AppKit attach / embed | [tish-apple](https://github.com/tishlang/tish-apple) | Landed (local) | `macos.attach`, `outerHost` / `skipMainMenu` / `skipTimerPump` |
| WKWebView script bridge | tish-apple | Landed (local) | macOS + iOS: `bridge={true}` → `__TISH_APP__`; `macos`/`ios`.webviewEval / webviewPostMessage. Demo: tish-desktop `examples/hello-ios`. |
| BrokerCore crate | tish-desktop `crates/tish_broker` | Landed (local) | Standalone app-runtime crate (not language); hosts may path-dep the crate only |
| BrokerCore + `state.*` | tish-desktop | Landed | `path` / `revision` / `source` contract |
| CapabilityProvider | tish-desktop | Landed | notification wrap + web stub; `webview.*` partial |
| Desktop → `attach_app` | tish-desktop | Landed (local) | `run(platformAttach.apple)` drains `PENDING_NATIVE_ROOTS` via `tish_macos::attach_app` |
| WK → BrokerCore | tish-desktop | Landed (local) | `brokerInvoke` + `onBridgeInvoke`; `state:changed` also `broadcast_event` to WK |
| LSP / wasm resolve | tish | Open | See [UPSTREAM_OPEN.md](./UPSTREAM_OPEN.md) |

**Policy:** no dual-process hybrid, no desktop-local resolver fork, no default `TISH_NATIVE_OPT=0` in templates.

**License note:** tish-apple may pull Apple PIF / SDK-related constraints; redistributors of hybrid builds with `platform-apple` must review tish-apple LICENSE for their distribution channel.

**React-like UI** → **lattish**, not tish upstream.
