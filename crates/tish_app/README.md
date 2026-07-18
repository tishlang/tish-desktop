# `cargo:tish_app`

Alias crate for [`tish_desktop`](../tish_desktop). Same API:

```tish
import { run, handle, createSurface, stateSet } from "cargo:tish_app"
```

Add to `package.json`:

```json
"tish": {
  "rustDependencies": {
    "tish_app": { "path": "../../crates/tish_app" }
  }
}
```
