# `cargo:tishlang_app`

Alias crate for [`tishlang_desktop`](../tish_desktop). Same API:

```tish
import { run, handle, createSurface, stateSet } from "cargo:tishlang_app"
```

Add to `package.json`:

```json
"tish": {
  "rustDependencies": {
    "tishlang_app": { "path": "../../crates/tish_app" }
  }
}
```
