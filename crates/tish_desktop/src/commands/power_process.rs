//! `power.preventSleep` / `power.allowSleep` (refcounted via `keepawake`),
//! and `process.pid` / `process.exit` / `process.restart`.

use serde_json::{json, Value};
use tauri::AppHandle;

use super::helpers::{catch_err, ensure_permission};
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "power.preventSleep" => prevent_sleep(state, args),
        "power.allowSleep" => allow_sleep(state),
        "process.pid" => Ok(json!({ "ok": true, "pid": std::process::id() })),
        "process.exit" => process_exit(app, state, args),
        "process.restart" => process_restart(app, state),
        _ => return None,
    };
    Some(result)
}

fn prevent_sleep(state: &AppState, args: &Value) -> Result<Value, String> {
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("User requested")
        .to_string();

    let mut count = state.sleep_blocks.lock();
    if *count == 0 {
        let guard = catch_err("power.preventSleep", || {
            keepawake::Builder::default()
                .display(true)
                .idle(true)
                .sleep(true)
                .reason(reason)
                .app_name("Tish Desktop")
                .create()
                .map_err(|e| e.to_string())
        })?;
        *state.keepawake_guard.lock() = Some(guard);
    }
    *count += 1;
    Ok(json!({ "ok": true, "blocks": *count }))
}

fn allow_sleep(state: &AppState) -> Result<Value, String> {
    let mut count = state.sleep_blocks.lock();
    if *count > 0 {
        *count -= 1;
    }
    if *count == 0 {
        *state.keepawake_guard.lock() = None;
    }
    Ok(json!({ "ok": true, "blocks": *count }))
}

fn process_exit(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "process")?;
    let code = args.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    app.exit(code);
    Ok(json!({ "ok": true }))
}

fn process_restart(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "process")?;
    app.restart();
}
