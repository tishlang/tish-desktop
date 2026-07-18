# BYO UI example

Classic desktop surface **without lattish**: shell + Tauri webview + plain DOM Tish UI.

- Shared state: `stateSet` / `stateGet` / `state:changed`
- Public imports: `cargo:tish_app`, `@tish-desktop/app-api`
- See also [docs/HYBRID.md](../../docs/HYBRID.md) and [docs/LATTISH.md](../../docs/LATTISH.md)

```bash
npm install
npm run dev
```

Pure web profile (stub bridge):

```bash
# point index at web-bridge in a web-only entry if desired
npm run build:web
```
