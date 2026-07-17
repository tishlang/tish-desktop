//! `shortcut.register` / `unregister` / `unregisterAll` / `isRegistered`.
//!
//! Registered shortcuts are tracked in `AppState.shortcuts` (id -> accelerator)
//! so `unregister(id)` and `unregisterAll` can find/clear the native registration.
//! Firing a shortcut emits `shortcut:{id}` to the frontend.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::helpers::ensure_global_shortcut;
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "shortcut.register" => register(app, state, args),
        "shortcut.unregister" => unregister(app, state, args),
        "shortcut.unregisterAll" => unregister_all(app, state),
        "shortcut.isRegistered" => is_registered(app, state, args),
        _ => return None,
    };
    Some(result)
}

fn register(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_global_shortcut(app, state)?;
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("id required")?.to_string();
    let accelerator = args
        .get("accelerator")
        .and_then(|v| v.as_str())
        .ok_or("accelerator required")?
        .to_string();

    let emit_id = id.clone();
    app.global_shortcut()
        .on_shortcut(accelerator.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit(&format!("shortcut:{emit_id}"), json!({ "id": emit_id }));
            }
        })
        .map_err(|e| e.to_string())?;

    state.shortcuts.lock().insert(id.clone(), accelerator.clone());
    Ok(json!({ "ok": true, "id": id, "accelerator": accelerator }))
}

fn unregister(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_global_shortcut(app, state)?;
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("id required")?;
    let accelerator = state
        .shortcuts
        .lock()
        .remove(id)
        .ok_or_else(|| format!("shortcut not registered: {id}"))?;
    app.global_shortcut()
        .unregister(accelerator.as_str())
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn unregister_all(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_global_shortcut(app, state)?;
    app.global_shortcut().unregister_all().map_err(|e| e.to_string())?;
    state.shortcuts.lock().clear();
    Ok(json!({ "ok": true }))
}

fn is_registered(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_global_shortcut(app, state)?;
    let accelerator = if let Some(a) = args.get("accelerator").and_then(|v| v.as_str()) {
        a.to_string()
    } else if let Some(id) = args.get("id").and_then(|v| v.as_str()) {
        state
            .shortcuts
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| format!("shortcut not registered: {id}"))?
    } else {
        return Err("id or accelerator required".into());
    };
    let registered = app.global_shortcut().is_registered(accelerator.as_str());
    Ok(json!({ "ok": true, "registered": registered }))
}
