//! Broker `state.*` — in-memory shared microfrontend state (not persisted `store.*`).

use serde_json::{json, Value};
use tauri::AppHandle;

use crate::broker::emit_state_changed;
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "state.get" => state_get(state, args),
        "state.set" => state_set(app, state, args),
        "state.patch" => state_patch(app, state, args),
        "state.keys" => Ok(json!({ "ok": true, "keys": state.shared_state.keys() })),
        "state.delete" => state_delete(app, state, args),
        "state.surfaces" => Ok(json!({
            "ok": true,
            "surfaces": state.surfaces.list(),
        })),
        _ => return None,
    };
    Some(result)
}

/// Accept plan `path` or legacy `key`.
fn path_arg(args: &Value) -> Result<String, String> {
    args.get("path")
        .or_else(|| args.get("key"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "path required".into())
}

fn source_arg(args: &Value) -> String {
    args.get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("webview")
        .to_string()
}

fn state_get(state: &AppState, args: &Value) -> Result<Value, String> {
    let path = path_arg(args)?;
    let (value, revision) = state.shared_state.get(&path);
    Ok(json!({ "ok": true, "path": path, "value": value, "revision": revision }))
}

fn state_set(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    let path = path_arg(args)?;
    let value = args.get("value").cloned().unwrap_or(Value::Null);
    let source = source_arg(args);
    let out = state.shared_state.set(path.clone(), value.clone());
    let revision = out.get("revision").and_then(|v| v.as_u64()).unwrap_or(0);
    emit_state_changed(app, &path, &value, revision, &source);
    Ok(out)
}

fn state_patch(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    let path = path_arg(args)?;
    let patch = args
        .get("value")
        .or_else(|| args.get("patch"))
        .cloned()
        .unwrap_or(json!({}));
    let source = source_arg(args);
    let out = state.shared_state.patch(path.clone(), patch)?;
    if let Some(v) = out.get("value") {
        let revision = out.get("revision").and_then(|r| r.as_u64()).unwrap_or(0);
        emit_state_changed(app, &path, v, revision, &source);
    }
    Ok(out)
}

fn state_delete(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    let path = path_arg(args)?;
    let source = source_arg(args);
    let (deleted, revision) = state.shared_state.delete(&path);
    if deleted {
        emit_state_changed(app, &path, &Value::Null, revision, &source);
    }
    Ok(json!({ "ok": true, "deleted": deleted, "revision": revision }))
}
