# tishlang_desktop

Tauri 2 host library for **Tish-first** desktop apps (`cargo:tishlang_desktop`).

## Public Value-ABI surface

Imported from shell Tish as `cargo:tishlang_desktop`:

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

`cargo install tishlang_desktop` installs a thin `mode` launcher that prefers the CLI on `PATH` (from `@tishlang/tish-desktop`). Set `TISH_MODE_CLI` or `TISH_DESKTOP_CLI` to override.

## Publish notes (crates.io)

**Local / monorepo (default):** this crate depends on

```toml
tishlang_core = { path = "../../../tish/crates/tish_core", features = ["send-values"] }
tishlang_broker = { path = "../tish_broker" }
```

so `tish build --native-backend rust` shares one physical `Value` type with the `tish` CLI. Mixing path `0.1.x` with crates.io `2.39.x` produces dual-`Value` link errors.

Optional sibling hosts (`platform-apple` / ms / lin / android) are **monorepo-only** path deps and are stripped for crates.io publish.

**Automated publish:** promote a GitHub prerelease to a full release → [`.github/workflows/crates-release.yml`](../../.github/workflows/crates-release.yml) sets versions from the tag, rewrites path deps to crates.io (`tishlang_core` train **2.39.x**, plus `tishlang_broker` / `tishlang_app`), then publishes in order. See [`docs/RELEASE.md`](../../docs/RELEASE.md).

**Manual / local publish:**

1. Align with the published `tish` / `tishlang_core` train (ecosystem **2.39.x** today). Temporarily switch deps to versioned lines **and** ensure the CLI you ship was built against the same crate.
2. Confirm crate metadata (`repository`, `homepage`, `readme`, `keywords`, `categories`) is present.
3. Publish `tishlang_broker`, then `tishlang_desktop`, then `tishlang_app` (`cargo publish -p … --dry-run` first).
4. Do **not** publish `tishlang_desktop_sample_ext` — `publish = false`.

Consumers after publish: versioned `tish.rustDependencies.tishlang_desktop` (or `tishlang_app`) in app `package.json` — no monorepo checkout required.

## Auth notes

`auth.login` supports PKCE + loopback or `redirectMode: "scheme"`. When scopes include `openid` (or `oidc: true`), a `nonce` is sent and checked against the `id_token` payload (claim check; JWKS signature verification is a follow-up). Optional `revocationEndpoint` is used on `auth.logout` (best-effort).
