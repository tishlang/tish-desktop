# Hybrid example

Proves **0b** (shared `state.*`) and **SC4** (native ‖ webview on macOS) without reinventing UI kits.

## Layers

| Layer | Path | Stack |
|-------|------|--------|
| **Contract** | `app/demo.tish` | Shared `state.*` paths both panes use |
| **Shell (SC4)** | `src/main.apple.tish` → `app/Shell.macos.tish` | AppKit `<split>` + nested WK; **Tauri outerHost** + plugins |
| **Native pane** | `app/NativePane.macos.tish` | AppKit tags + `brokerInvoke` → Tauri CapProviders |
| **Webview pane** | `ui/DemoPane.tish` | lattish + ui-theme + same plugin cmds |
| **Tauri companion** | `ui/Extensions.tish` | Direct Tauri bridge (`extensions.html`) |
| **Multi-window** | `src/main.tish` | Dual Tauri webviews (same Vite UI) |

Native and webview stay on different UI stacks (AppKit vs lattish). They share **behavior** via `demo.tish` + BrokerCore. On desktop, **Tauri plugins** (`clipboard`, `dialog`, `os`, …) are enabled with `platformAttach.apple.outerHost: true` — nested WK and the companion Extensions window both hit the same CapProviders.

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
