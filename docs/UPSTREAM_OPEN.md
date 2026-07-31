# Upstream open items (unified app runtime)

Tracked gaps that belong in **tish** / **tish-apple** / sibling hosts, not durable desktop forks.

Most resolve / attach / WK / broker items from the unified host plan are **landed**. Remaining work is polish and out-of-scope PARITY wishlists — not desktop-local forks.

| Item | Repo | Status | Notes |
|------|------|--------|-------|
| LSP platform/surface-aware resolve | tish (`tish-lsp`) | Landed | `initialization_options.platform` / `surface` + `TISH_*` env |
| Wasm / compiler-wasm resolve cascade | tish | Landed | `resolve_virtual` + env |
| Resolve-id CLI hard CI gate | tish | Landed | Fail if no `tish` / lacks `resolve-id` |
| Vite `package.json` platform/surface | tish `vite-plugin-tish` | Landed | `tish.platform` / `tish.surface` or `tish.desktop.*` |
| Apple UNUserNotificationCenter | tish-apple | Landed | `macos.notification*` + desktop `local_invoke` |
| SC4 hybrid native+webview | tish-desktop | Landed | `examples/hybrid` `dev:hybrid` / `build:shell:apple` |
| Tauri-free BrokerCore | tish-desktop `crates/tish_broker` | Landed | Hosts path-dep the crate only |
| iOS broker profile | tishlang_broker + tish-apple | Landed | demo in `examples/hello-ios` |
| iOS CLI sim launch | tish-desktop | Landed | `mode ios` |
| Broker `webview.*` ↔ WK | tish-desktop + tish-apple | Landed | load/postMessage/list/eval |
| ms/lin / android attach stubs | sibling hosts | Landed | stubs; not full product hosts |
| Lattish `NativeSurface` / `WebSurface` sugar | lattish | Later / external | Not required by core |
| PARITY iOS tag wishlist (radio/pick_list/…) | tish-apple | Out of scope here | See [PARITY.md](./PARITY.md) |
| Android real host beyond stub | tish-android | Out of scope here | Stub only |

Shims allowed only with: linked issue, owner repo, removal criteria. See [UPSTREAM.md](./UPSTREAM.md).
