# State cookbook (dual entrypoints)

## Rules

1. Lattish `useState` / stores live only in the webview heap.
2. Domain truth (FS, devices, window policy) lives in shell/host.
3. Crossing the broker always means `invoke` or `listen` — never shared memory.

## File browser pattern

| Action | Side | Mechanism |
|--------|------|-----------|
| List directory | UI → shell | `invoke("fs.list", { path })` |
| Read preview | UI → shell | `invoke("fs.readText", { path })` |
| External change | shell → UI | `listen("fs:changed")` → debounced refresh |
| Selection / scroll | UI only | local store fields |

Shell is the leader for listing contents. UI holds a cache and invalidates on events.
