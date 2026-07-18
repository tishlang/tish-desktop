# Hybrid example

Proves plan gates **0b** (shared `state.*` across two webviews) and **2** scaffolding (native surface slot + `platformAttach`).

| Surface | Role |
|---------|------|
| `chrome` webview | Sidebar stand-in — picks docs → `state.set` / `app.openDoc` |
| `main` webview | Detail — listens `state:changed`, platform-resolved `Button.*` |
| `native-chrome` | Queued `kind: "native"` for `macos.attach` when `platform-apple` is enabled |

**Honest v1:** dual coordinated windows until WK script bridge lands in tish-apple ([UPSTREAM.md](../../docs/UPSTREAM.md)).

```bash
npm install
npm run dev          # Vite :5177 + shell
npm run build:web    # pure web profile (Button.web.tish)
```

Platform resolve: `import { Button } from "./Button"` → `Button.webview.tish` under `--surface webview`, `Button.web.tish` under `--surface web`.
