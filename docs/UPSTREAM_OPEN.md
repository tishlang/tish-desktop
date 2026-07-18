# Upstream open items (unified app runtime)

Tracked gaps that belong in **tish** / **tish-apple** / sibling hosts, not durable desktop forks.

| Item | Repo | Status | Notes |
|------|------|--------|-------|
| LSP platform/surface-aware resolve | tish (`tish-lsp`) | Landed (local) | `initialization_options.platform` / `surface` + `TISH_*` env |
| Wasm / compiler-wasm resolve cascade | tish | Landed (local) | `resolve_virtual` uses `platform_virtual_keys` + env |
| Resolve-id CLI hard CI gate | tish | Landed (local) | Fail if no `tish` / lacks `resolve-id` |
| Apple UNUserNotificationCenter | tish-apple | Landed (local) | `macos.notification*` + desktop `local_invoke` |
| One-window hybrid polish | tish-desktop + apple | Landed (local) | `build:shell:apple` |
| Tauri-free BrokerCore | tish-desktop `crates/tish_broker` | Landed (local) | App runtime (not language); hosts path-dep the crate only |
| iOS broker profile | tish_broker + tish-apple `tish-ios` | Landed (local) | demo in tish-desktop `examples/hello-ios` |
| iOS CLI sim launch | tish-desktop | Landed (local) | `examples/hello-ios` + `tish-desktop ios` — xcodebuild + simctl |
| Native `undefined` codegen | tish | Landed (local) | Prelude + Ident → `Value::Null`; `regr_undefined_ident` |
| iOS WKWebView + bridge | tish-apple `tish-ios` | Landed (local) | Local UIView `WKWebView` bind; `__TISH_APP__` → `onBridgeInvoke` → broker |
| iOS `state:changed` → WK | tish-apple `tish-ios` | Landed (local) | Broker broadcasts after `state.set`/`patch`/`delete` |
| iOS `dialog.*` | tish-apple `tish-ios` | Landed (local) | `message` / `confirm` / `ask` via UIAlertController |
| ms/lin native hosts | tish-ms / tish-lin | Landed (local) | `attach_native` + notify; `platform-ms` / `platform-lin` |
| Lattish RN tag adapters | lattish | Landed (local) | `@tishlang/lattish/adapters/rn` — View/Text/Pressable/Button |
| Android platform token + stub host | tish + tish-android | Landed (local) | `.android` / `.mobile` cascade; `attach_native` stub |
| Cross-platform parity doc | tish-desktop | Landed | [PARITY.md](./PARITY.md) — keep updated |
| iOS host tags (inputs) | tish-apple `tish-ios` | Landed (local) | `textinput` / `password` / `toggler` / `checkbox` / `image` / `space` / `rule` |
| iOS incremental patch | tish-apple `tish-ios` | Landed (local) | `try_patch_vtree` keeps `UITextField` focus across controlled `onChange` re-renders |
| Surface authoring vocab | tish-desktop docs | Landed (local) | Surface / `createSurface` / host `<webview>`; reject NativeView/WebView — [HYBRID.md](./HYBRID.md) |
| Broker `webview.*` ↔ WK | tish-desktop + tish-apple | Open | Unify `webview.load`/`postMessage` with `macos`/`ios` webviewEval / webviewPostMessage |

Shims allowed only with: linked issue, owner repo, removal criteria. See [UPSTREAM.md](./UPSTREAM.md).
