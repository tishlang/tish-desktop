//! Core broker commands: ping, multi-window, FS sandbox, extensions.

use serde_json::{json, Value};
use tauri::AppHandle;

use crate::fs_sandbox;
use crate::state::AppState;
use crate::windows;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "ping" => Ok(json!({
            "ok": true,
            "ts": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            "protocol": crate::state::PROTOCOL_VERSION,
        })),
        "window.list" => Ok(json!(windows::list_labels(app))),
        "window.focus" => (|| {
            let caller = windows::caller_label();
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .or(caller.as_deref())
                .ok_or("label required")?;
            windows::focus(app, label)?;
            Ok(json!({ "ok": true }))
        })(),
        "window.close" => (|| {
            let caller = windows::caller_label();
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .or(caller.as_deref())
                .ok_or("label required")?;
            windows::close(app, label)?;
            Ok(json!({ "ok": true }))
        })(),
        "window.create" => (|| {
            let mut spec: crate::state::WindowSpec =
                serde_json::from_value(args.clone()).map_err(|e| e.to_string())?;
            if spec.label.is_empty() {
                spec.label = windows::next_window_label(app);
            }
            windows::create_from_spec(app, &spec)?;
            Ok(json!({ "ok": true, "label": spec.label }))
        })(),
        "fs.list" => (|| {
            let root = state.fs_root.lock().clone().ok_or("fs root not set")?;
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let entries = fs_sandbox::list_dir(&root, path)?;
            Ok(json!({ "ok": true, "entries": entries }))
        })(),
        "fs.readText" => (|| {
            let root = state.fs_root.lock().clone().ok_or("fs root not set")?;
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("path required")?;
            let text = fs_sandbox::read_text(&root, path)?;
            Ok(json!({ "ok": true, "text": text }))
        })(),
        "fs.stat" => (|| {
            let root = state.fs_root.lock().clone().ok_or("fs root not set")?;
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("path required")?;
            let st = fs_sandbox::stat_path(&root, path)?;
            Ok(json!({ "ok": true, "stat": st }))
        })(),
        "fs.watch" => (|| {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            fs_sandbox::start_watch(app.clone(), state, path)?;
            Ok(json!({ "ok": true }))
        })(),
        "fs.unwatch" => {
            fs_sandbox::stop_watch(state);
            Ok(json!({ "ok": true }))
        }
        "extensions.list" => Ok(crate::extensions::REGISTRY.list()),
        _ => return None,
    };
    Some(result)
}
