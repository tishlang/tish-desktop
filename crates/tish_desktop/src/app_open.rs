//! macOS "open files" handling — files/folders dropped onto the running app's dock icon (or passed
//! by Finder's "Open With") arrive as `RunEvent::Opened { urls }`. We turn them into the `cli-open`
//! invocation the Dune webview already consumes (folders → workspace opens, files → editor tabs).
//!
//! NOTE: this covers dragging onto the RUNNING dock icon. Finder "Open With" / launching-by-file
//! also needs the app to be a packaged `.app` declaring the document types in Info.plist — the raw
//! dev binary has no bundle, so Finder won't offer Dune there. Cold-start opens additionally need a
//! pending-queue (cli_take_pending) since the webview isn't listening yet at launch.

use tauri::{AppHandle, Emitter};

/// Emit a `cli-open` event carrying an invocation built from the opened paths: existing directories
/// become `folders`, everything else a `files` entry (the webview routes `.code-workspace` itself).
pub fn emit_open_paths(app: &AppHandle, paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let mut folders: Vec<String> = Vec::new();
    let mut files: Vec<serde_json::Value> = Vec::new();
    for p in paths {
        match std::fs::metadata(p) {
            Ok(m) if m.is_dir() => folders.push(p.clone()),
            _ => files.push(serde_json::json!({ "path": p })),
        }
    }
    let inv = serde_json::json!({
        "folders": folders,
        "files": files,
        "addFolders": [],
        "newWindow": false,
    });
    let _ = app.emit("cli-open", inv);
}
