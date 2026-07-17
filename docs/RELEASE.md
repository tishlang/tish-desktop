# How to Release Tish Desktop

Mirrors the [tish release flow](https://github.com/tishlang/tish/blob/main/docs/RELEASE.md): cut a GitHub **prerelease**, then promote it to a full release to publish crates.io + npm.

---

## One-time setup

### GitHub secrets (Settings → Secrets and variables → Actions)

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token for `tish_desktop` |

npm uses **OIDC trusted publishing** (no `NPM_TOKEN`).

### npm Trusted Publishers

For each package, on npmjs.com → package → Settings → **Trusted Publisher** → GitHub Actions:

| Field | Value |
|-------|-------|
| Organization or user | `tishlang` |
| Repository | `tish-desktop` |
| Workflow filename | `npm-release.yml` |
| Environment | *(blank)* |

Packages:

- `@tish-desktop/cli`
- `@tish-desktop/desktop-api`
- `@tish-desktop/shared`
- `@tish-desktop/ui-theme`

Create the packages on npm (empty publish or “Create package”) before the first OIDC publish if npm requires them to exist.

---

## Every release

1. Tag / create a **prerelease** (e.g. `v0.1.0`) on the commit you want to ship. App distribute artifacts can use the draft distribute workflows separately.
2. When ready, **Edit** the release → uncheck “Set as a pre-release” → **Update release**.
3. That runs in parallel:
   - **Crates.io release** → `tish_desktop` (rewrites path `tishlang_core` → crates.io `2.39` for publish)
   - **NPM release** → `@tish-desktop/*` packages (version = tag without `v`)

### Manual re-run

- **Actions → Crates.io release → Run workflow** with the tag (skips if that version is already on crates.io). Optional `tishlang_core_version` input if the ecosystem train moved.
- **Actions → NPM release → Run workflow** with the tag (skips packages already at that version).

---

## Verify

```bash
npm view @tish-desktop/cli version
cargo search tish_desktop
# or: https://crates.io/crates/tish_desktop
```

---

## Notes

- **Path vs crates.io `tishlang_core`:** monorepo `cargo check` / `tish build` keep the path dep. Only the crates release workflow switches to a versioned dep for `cargo publish`. See [`crates/tish_desktop/README.md`](../crates/tish_desktop/README.md).
- **Do not publish** `tish_desktop_sample_ext` (`publish = false`).
- App signing / store / updater workflows under `.github/workflows/release-*` are separate from crate/npm package publish.
