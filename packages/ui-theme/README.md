# `@tishlang/tish-desktop-ui-theme`

Shared design tokens + small **lattish** primitives for Tish Desktop examples and apps.

## Contents

| Path | Role |
|------|------|
| `src/theme.css` | shadcn-style CSS variables (`--background`, `--primary`, …); dark default |
| `src/theme-utilities.css` | Maps `bg-background` / `text-muted-foreground` / … → `var(--*)` (required — tish-tailwind has no `@theme`) |
| `src/fonts.css` | Distinct UI font stack |
| `src/ui/*.tish` | `Button`, `Card`, `Section`, `Input`, `Badge`, `Separator`, `ScrollArea`, `Toolbar` |
| `src/index.tish` | Re-exports |

## Usage

1. Depend on the package (`file:` or published).
2. Alias `@tishlang/tish-desktop-ui-theme` in Vite to `src/index.tish`.
3. In `build-css.tish`, concatenate `fonts.css` + `theme.css` + `theme-utilities.css` with the tish-tailwind emit, and include theme `src/ui/*.tish` in `sourceFiles` so scanned utilities are emitted.
4. Import primitives in UI Tish:

```tish
import { Button, Section } from "@tishlang/tish-desktop-ui-theme"
```

`examples/basic` intentionally stays unthemed. Prefer this package for `file-browser`, `native-chrome`, and new apps.
