# Tish-first DX

Example and app code stay **Tish end-to-end**. Native/OS work lives in `crates/tish_desktop` and is exposed via the `desktop/v1` broker (`invoke` / `listen`).

| Layer | Allowed | Not allowed |
|-------|---------|-------------|
| Shell | `src/**/*.tish` + `cargo:tish_desktop` | Per-example `.rs`, inline Rust |
| UI | `ui/**/*.tish` + lattish + `@tish-desktop/*` | React/Vue/feature JS |
| Styles | `build-css.tish` + tish-tailwind / ui-theme | Large hand-rolled CSS frameworks outside theme |
| Host | `crates/tish_desktop` only | Copy-pasted Rust in `examples/*` |

**Minimal exceptions:** `vite.config.mjs`, `index.html`, tiny `bridge-boot.js` (`installBridge` only). Do not add new example `.js` for app logic.

**Tooling** (outside examples): `cli/`, `scripts/distribute/`, GitHub Actions may use Node/shell for packaging.
