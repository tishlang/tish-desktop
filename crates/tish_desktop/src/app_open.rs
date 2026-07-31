//! macOS file-open forwarding. Files/folders opened against the running app — dropped on the dock
//! icon, Finder "Open With", or a second `<binary> <path>` launch (via single-instance) — arrive as
//! `RunEvent::Opened { urls }` / the single-instance argv. We forward them to the webview as a
//! generic `open-paths` event carrying the raw absolute paths (each tagged with `isDir`); the hosting
//! app decides what to do with them. Nothing app-specific lives here.
//!
//! NOTE: Finder "Open With" / launch-by-file also needs the app packaged as a `.app` declaring its
//! document types in Info.plist (the hosting app's packaging). Cold-start opens additionally need the
//! host to queue the paths until its UI is listening.

use tauri::{AppHandle, Emitter};

/// Emit an `open-paths` event `{ paths: [{ path, isDir }] }` for the given absolute paths (no-op if
/// empty). `isDir` is a cheap fs probe so the host doesn't have to round-trip to classify each path.
pub fn emit_open_paths(app: &AppHandle, paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let entries: Vec<serde_json::Value> = paths
        .iter()
        .map(|p| {
            let is_dir = std::fs::metadata(p).map(|m| m.is_dir()).unwrap_or(false);
            serde_json::json!({ "path": p, "isDir": is_dir })
        })
        .collect();
    let _ = app.emit("open-paths", serde_json::json!({ "paths": entries }));
}
