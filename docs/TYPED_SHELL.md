# Typed Tish for shell / native performance

On `--native-backend rust`, typed annotations drive codegen (unbox to `f64`, `Vec<f64>`, structs). Opt out with `TISH_NATIVE_OPT=0` only when debugging.

| Layer | Guidance |
|-------|----------|
| Shell (`src/main.tish`, handlers) | Prefer typed params/returns; `tish build --check warn` (CI: `error`) |
| Native UI (`*.macos.tish` / apple) | Same — Rust-backed hosts benefit |
| Webview / web UI | Gradual typing OK; JS emit does not get the same unbox wins |
| Broker / `state.*` / `invoke` | Dynamic boundary — keep typed logic *inside* a surface |

Scaffold shells should include at least one typed handler example. Gaps in unbox/checker → issues on **tish**, not desktop workarounds.
