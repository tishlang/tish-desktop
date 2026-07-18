# Hybrid example

Proves **0b** (shared `state.*`) and **SC4** (native ‖ webview on macOS) without reinventing UI kits.

## Layers

| Layer | Path | Stack |
|-------|------|--------|
| **Contract** | `app/demo.tish` | Shared `state.*` paths both panes use |
| **Shell (SC4)** | `src/main.apple.tish` → `app/Shell.macos.tish` | `cargo:tish_app` + AppKit `<split>` + `<webview bridge>` |
| **Native pane** | `app/NativePane.macos.tish` | AppKit host tags (`tish:macos`) |
| **Webview pane** | `ui/DemoPane.tish` | **lattish** + **`@tish-desktop/ui-theme`** + app-api |
| **Multi-window** | `src/main.tish` | Dual Tauri webviews (same Vite UI) |

Native and webview deliberately use **different** UI stacks (AppKit vs lattish). They share **behavior** via `app/demo.tish` + BrokerCore — not a single component tree.

```bash
npm install
npm run build:css
npm run check:tree
npm run dev:hybrid          # Vite + apple shell (both panes)
npm run dev:hybrid:rebuild  # rebuild native shell
npm run dev:multi           # dual Tauri webviews
npm run dev:web             # pure web + web-bridge
```

See [HYBRID.md](../../docs/HYBRID.md) · [LATTISH.md](../../docs/LATTISH.md).
