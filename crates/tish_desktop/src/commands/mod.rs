//! Broker dispatch: the webview calls `desktop_invoke(cmd, args)` which routes
//! to a Tish-registered handler or one of the command modules below.

pub(crate) mod chrome;
mod clipboard;
mod core;
mod dialog_extra;
mod helpers;
mod menu_set;
mod power_process;
pub(crate) mod secrets_auth;
mod shell_os;
mod shortcut;
pub(crate) mod state_shared;
mod store_auto;
mod webview_cap;
mod window_extra;

use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::state::AppState;

#[tauri::command]
pub fn desktop_protocol() -> serde_json::Value {
    json!({ "protocol": crate::state::PROTOCOL_VERSION })
}

#[tauri::command]
pub fn desktop_invoke(
    app: AppHandle,
    state: State<'_, AppState>,
    cmd: String,
    args: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let args = args.unwrap_or(json!({}));
    dispatch(&app, &state, &cmd, args)
}

/// Extra command modules, tried in order. Each returns `None` if it doesn't own `cmd`.
type ExtraDispatch =
    fn(&AppHandle, &AppState, &str, &serde_json::Value) -> Option<Result<serde_json::Value, String>>;

/// Legacy modules after CapProviders (migration period). `state.*` is handled earlier.
const LEGACY_MODULES: &[ExtraDispatch] = &[
    core::try_dispatch,
    chrome::try_dispatch,
    webview_cap::try_dispatch,
    window_extra::try_dispatch,
    dialog_extra::try_dispatch,
    clipboard::try_dispatch,
    shortcut::try_dispatch,
    shell_os::try_dispatch,
    store_auto::try_dispatch,
    power_process::try_dispatch,
    secrets_auth::try_dispatch,
    menu_set::try_dispatch,
];

fn dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Plan order: handlers → state.* → CapProvider → legacy try_dispatch → unknown
    if state.handlers.lock().contains_key(cmd) {
        return state.call_handler(cmd, args);
    }

    if let Some(result) = state_shared::try_dispatch(app, state, cmd, &args) {
        return result;
    }

    if let Some(result) = crate::caps::try_caps(app, state, cmd, &args) {
        return result;
    }

    for try_dispatch in LEGACY_MODULES {
        if let Some(result) = try_dispatch(app, state, cmd, &args) {
            return result;
        }
    }

    Err(format!("unknown command: {cmd}"))
}

#[tauri::command]
pub fn desktop_emit_tick(app: AppHandle, ts: u64) -> Result<(), String> {
    app.emit("tick", json!({ "ts": ts })).map_err(|e| e.to_string())
}

pub fn spawn_tick_loop(app: AppHandle, tick_ms: u64) {
    std::thread::spawn(move || {
        let interval = std::time::Duration::from_millis(tick_ms.max(1));
        loop {
            std::thread::sleep(interval);
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            if let Err(e) = desktop_emit_tick(app.clone(), ts) {
                eprintln!("tish_desktop: tick emit failed: {e}");
            }
        }
    });
}
