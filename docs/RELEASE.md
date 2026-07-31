# How to Release Tish Desktop / Tish Mode

Mirrors the [tish release flow](https://github.com/tishlang/tish/blob/main/docs/RELEASE.md): conventional commits → push `main` → GitHub **prerelease** → promote to a full release → crates.io + npm publish.

This repo is the **tish-mode** monorepo. Published artifacts stay under the **tish-desktop** product names so consumers never need a monorepo checkout:

| Surface | Package |
|---------|---------|
| npm CLI | `@tishlang/tish-desktop` (`npx mode`) |
| npm libs | `@tishlang/tish-desktop-shared`, `@tishlang/tish-desktop-api`, `@tishlang/tish-app-api`, `@tishlang/tish-desktop-ui-theme` |
| crates.io | `tish_broker`, `tish_desktop`, `tish_app` |

---

## Before You Start: One-Time Setup

### 1. GitHub Secrets (Settings → Secrets and variables → Actions)

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token for `tish_broker` / `tish_desktop` / `tish_app` |

npm uses **OIDC trusted publishing** (no `NPM_TOKEN`).

### 2. npm Trusted Publishers

For each package, on npmjs.com → package → Settings → **Trusted Publisher** → GitHub Actions:

| Field | Value |
|-------|-------|
| Organization or user | `tishlang` |
| Repository | `mode` |
| Workflow filename | `npm-release.yml` |
| Environment | *(blank)* |

Packages:

- `@tishlang/tish-desktop`
- `@tishlang/tish-desktop-api`
- `@tishlang/tish-app-api`
- `@tishlang/tish-desktop-shared`
- `@tishlang/tish-desktop-ui-theme`

Create the packages on npm (empty publish or “Create package”) before the first OIDC publish if npm requires them to exist.

---

## Every Release

### Step 1: Commit with a release-triggering message

You need at least one commit that triggers a version bump. Use conventional commits:

```
feat: add something new        → minor (0.1.0 → 0.2.0)
fix: fix a bug                 → patch (0.1.0 → 0.1.1)
perf: make it faster           → patch
feat!: breaking change         → major (0.1.0 → 1.0.0)
```

`docs:` and `chore:` do **not** trigger a release. If CI fails with “No incremental release would be triggered”, you need a `feat`, `fix`, `perf`, or `BREAKING CHANGE` commit.

### Step 2: Push to `main`

```bash
git push origin main
```

### Step 3: Let CI run

- Open **Actions** in the tish-desktop repo
- Wait for **CI** and **Release (prerelease)** to finish
- A GitHub **prerelease** (e.g. `v0.1.0`) should appear under Releases

If it fails:

- **“No incremental release would be triggered”** → Add a `feat:`, `fix:`, or `perf:` commit and push again
- **Build/test failures** → Fix them and push again

### Step 4: Promote the prerelease to a full release

1. Go to **Releases**
2. Find the **latest prerelease**
3. Click **Edit**
4. **Uncheck** “Set as a pre-release”
5. Click **Update release**

This runs the NPM and Crates.io release workflows. They run automatically; no further action needed.

### Manual re-run

- **Actions → Crates.io release → Run workflow** with the tag (skips versions already on crates.io). Optional `tishlang_core_version` if the ecosystem train moved.
- **Actions → NPM release → Run workflow** with the tag (skips packages already at that version).

---

## Verify

```bash
npm view @tishlang/tish-desktop version
npm view @tishlang/tish-desktop-api version
cargo search tish_desktop
# or: https://crates.io/crates/tish_desktop
```

Standalone smoke (no monorepo):

```bash
npx @tishlang/tish-desktop init /tmp/td-smoke --ui none
cd /tmp/td-smoke && npm install && npx mode doctor --platform macos --surface webview
```

---

## Notes

- **Monorepo vs registry:** local `cargo check` / `tish build` keep path deps on `tishlang_core` and sibling hosts. Release workflows rewrite to crates.io / npm versions. See [`crates/tish_desktop/README.md`](../crates/tish_desktop/README.md).
- **`platform-apple` / ms / lin / android:** monorepo-only (sibling path deps). Published `tish_desktop` is the default Tauri webview host.
- **Do not publish** `tish_desktop_sample_ext` (`publish = false`).
- Root package name is **`tish-mode`** (`private: true`) — it is never published. Consumers use `@tishlang/tish-desktop*` packages.
- App signing / store / updater workflows under `.github/workflows/release-*` are separate from crate/npm package publish.
