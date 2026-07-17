# tish_desktop

Tauri 2 host library for **Tish-first** desktop apps (`cargo:tish_desktop`).

## Public Value-ABI surface

Imported from shell Tish as `cargo:tish_desktop`:

| Export | Role |
|--------|------|
| `run(config?)` | Start Tauri event loop (blocks) |
| `createWindow(spec)` | Queue / create a webview window |
| `handle(name, fn)` | Register a shell command handler |
| `useExtensions(ids)` | Enable extension ids |
| `registerRustExtension(name)` | Mark a Rust `cargo:` module loaded |
| `protocol` | `"desktop/v1"` |

UI talks over the broker via `desktop_invoke` / events (see repo `packages/desktop-api`).

## CLI binary

`cargo install tish_desktop` installs a thin `tish-desktop` launcher that prefers the Tish CLI on `PATH` (from `@tish-desktop/cli`). Set `TISH_DESKTOP_CLI` to override.

## Publish notes

Depends on `tishlang_core`. Local workspace uses `[patch.crates-io]`. Before crates.io publish, align versions with the tish release train and ensure the patched path is not required by consumers.

`tish_desktop_sample_ext` is **private** / examples-only and is not published.
