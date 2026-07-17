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

## Publish notes (crates.io)

**Local / monorepo (default):** this crate depends on

```toml
tishlang_core = { path = "../../../tish/crates/tish_core", features = ["send-values"] }
```

so `tish build --native-backend rust` shares one physical `Value` type with the `tish` CLI. Mixing path `0.1.x` with crates.io `2.39.x` produces dual-`Value` link errors.

**Automated publish:** promote a GitHub prerelease to a full release → [`.github/workflows/crates-release.yml`](../../.github/workflows/crates-release.yml) sets the crate version from the tag, rewrites the path dep to `tishlang_core = { version = "2.39", … }` (override via workflow input), then `cargo publish`. See [`docs/RELEASE.md`](../../docs/RELEASE.md).

**Manual / local publish:**

1. Align with the published `tish` / `tishlang_core` train (ecosystem **2.39.x** today). Temporarily switch this dep to `tishlang_core = { version = "2.39", features = ["send-values"] }` **and** ensure the CLI you ship was built against the same crate.
2. Confirm crate metadata (`repository`, `homepage`, `readme`, `keywords`, `categories`) is present.
3. Run `cargo publish -p tish_desktop --dry-run`, then publish.
4. Do **not** publish `tish_desktop_sample_ext` — `publish = false`.

Consumers after publish: path or versioned `tish.rustDependencies.tish_desktop` in app `package.json`.

## Auth notes

`auth.login` supports PKCE + loopback or `redirectMode: "scheme"`. When scopes include `openid` (or `oidc: true`), a `nonce` is sent and checked against the `id_token` payload (claim check; JWKS signature verification is a follow-up). Optional `revocationEndpoint` is used on `auth.logout` (best-effort).
