# Hybrid example

Proves plan gates **0b** (shared `state.*` across surfaces) and **2** scaffolding (native surface slot + `platformAttach`).

**Mode:** hybrid **multi-window** (desktop) — dual Tauri `createSurface({ kind: "webview" })` + optional native slot. Not one-window native+WK (that path uses host tag `<webview bridge>`; see [HYBRID.md](../../docs/HYBRID.md)).

| Surface | Role |
|---------|------|
| `chrome` webview | Sidebar stand-in — picks docs → `state.set` / `app.openDoc` |
| `main` webview | Detail — listens `state:changed`, platform-resolved `Button.*` |
| `native-chrome` | Queued `kind: "native"` + `root: Sidebar` for `macos.attach` (`platform-apple`) |

Canonical API: imperative `createSurface` + `run()` — no Surface JSX components required.

```bash
npm install
npm run dev          # Vite :5177 + shell
npm run build:web    # pure web profile (Button.web.tish)
npm run build:shell        # dual-webview shell (Sidebar.tish stub)
npm run build:shell:apple  # platform-apple + Sidebar.macos.tish + WK bridge (macOS)
```

Platform resolve: `import { Button } from "./Button"` → `Button.webview.tish` under `--surface webview`, `Button.web.tish` under `--surface web`.

Open upstream items: [UPSTREAM_OPEN.md](../../docs/UPSTREAM_OPEN.md).
