# Lattish is optional

Core runtime (`cargo:tish_app`, `@tish-desktop/app-api`, BrokerCore, caps, bridges) has **zero** dependency on lattish or `@tish-desktop/ui-theme`.

| Product shape | UI |
|---------------|-----|
| Classic desktop BYO | Any Tish UI / plain DOM |
| Classic + lattish | Examples like `native-chrome` |
| Pure web | Vite + web-bridge + BYO or lattish |
| Pure native | tish-apple only |

**React-like library surface** (hooks ergonomics, component primitives, tag adapters) is owned by **lattish**, not the tish compiler or desktop core.

Templates:

- Default examples may use lattish
- `init --template bare` / `examples/byo-ui` — no lattish
