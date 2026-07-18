# Lattish is optional

Core runtime (`cargo:tish_app`, `@tish-desktop/app-api`, BrokerCore, caps, bridges) has **zero** dependency on lattish or `@tish-desktop/ui-theme`.

| Product shape | UI |
|---------------|-----|
| Classic desktop BYO | Any Tish UI / plain DOM |
| Classic + lattish | Examples like `native-chrome` |
| Pure web | Vite + web-bridge + BYO or lattish |
| Pure native | tish-apple only |

**React-like library surface** (hooks ergonomics, component primitives, tag adapters) is owned by **lattish**, not the tish compiler or desktop core.

RN-style web/webview adapters: `@tishlang/lattish/adapters/rn` (`View`, `Text`, `Pressable`, `Button`, `select`).

## Surfaces (not Views)

Shell / host composition is **core** vocabulary — do not replace it with RN-style `NativeView` / `WebView`:

| Canonical (core, required for BYO) | Optional later (lattish only) |
|------------------------------------|-------------------------------|
| `createSurface({ kind: "native"\|"webview", … })` | `NativeSurface` / root `WebSurface` sugar |
| Host tag `<webview bridge … />` | Nested `WebSurface` → same host tag |

If lattish adds `NativeSurface` / `WebSurface`, they must be thin aliases over the table above and must **never** be required by `cargo:tish_app` or `@tish-desktop/app-api`. RN `View` stays a **layout** adapter for DOM/webview UI, not a surface/window API.

See [HYBRID.md](./HYBRID.md) · [UNIFIED_APP.md](./UNIFIED_APP.md).

Templates:

- Default examples may use lattish
- `init --template bare` / `examples/byo-ui` — no lattish
