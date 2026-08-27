//! Broker dispatch: the webview calls `desktop_invoke(cmd, args)` which routes
//! to a Tish-registered handler or one of the command modules below.

pub(crate) mod chrome;
mod clipboard;
mod core;
pub(crate) mod dialog_extra;
mod helpers;
mod menu_set;
mod power_process;
pub(crate) mod secrets_auth;
mod shell_os;
mod shortcut;
pub(crate) mod state_shared;
pub(crate) mod store_auto;
pub(crate) mod webview_cap;
mod window_extra;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::state::AppState;
use crate::windows;

#[tauri::command]
pub fn desktop_protocol() -> serde_json::Value {
    json!({ "protocol": crate::state::PROTOCOL_VERSION })
}

#[tauri::command]
pub async fn desktop_invoke(
    app: AppHandle,
    window: WebviewWindow,
    cmd: String,
    args: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let args = args.unwrap_or(json!({}));
    let caller = window.label().to_string();
    // A Tish `handle()` command is app logic (fs / process / http / index) that can block for a
    // long time; run it OFF the main thread so the webview never beachballs. The rest — state.*,
    // CapProviders, and the window/menu/dialog glue — is fast and (on macOS) main-thread-only, so
    // it stays inline. NativeFn + AppState are Send + Sync, so the handler moves safely. State is
    // fetched from the AppHandle (not taken as a command param) so no non-Send borrow crosses the
    // await and the command future stays Send.
    let is_handler = app.state::<AppState>().handlers.lock().contains_key(&cmd);
    // Diagnostic trace: a command that logs `→` but never `←` is the one hanging the boot (a `←`
    // with a large ms is merely slow). Gated on TISH_DESKTOP_TRACE_INVOKE=1 so it's silent by default.
    let trace = std::env::var("TISH_DESKTOP_TRACE_INVOKE").as_deref() == Ok("1");
    if trace {
        eprintln!("[desktop_invoke] → {cmd} (handler={is_handler}, caller={caller})");
    }
    let started = std::time::Instant::now();
    let out = if is_handler {
        let app2 = app.clone();
        let cmd2 = cmd.clone();
        let caller2 = caller.clone();
        match tauri::async_runtime::spawn_blocking(move || {
            let state = app2.state::<AppState>();
            dispatch(&app2, state.inner(), &cmd2, args, Some(&caller2))
        })
        .await
        {
            Ok(r) => r,
            Err(e) => Err(format!("desktop_invoke worker failed: {e}")),
        }
    } else {
        let state = app.state::<AppState>();
        dispatch(&app, state.inner(), &cmd, args, Some(&caller))
    };
    if trace {
        eprintln!(
            "[desktop_invoke] ← {cmd} ({}ms)",
            started.elapsed().as_millis()
        );
    }
    out
}

/// Extra command modules, tried in order. Each returns `None` if it doesn't own `cmd`.
type ExtraDispatch =
    fn(&AppHandle, &AppState, &str, &serde_json::Value) -> Option<Result<serde_json::Value, String>>;

/// In-process entry for shell / nested WK `brokerInvoke` when Tauri `AppHandle` is live.
/// Nested calls inherit the invoking webview from `desktop_invoke` (thread-local).
pub fn invoke_for_handle(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let caller = windows::caller_label();
    dispatch(app, state, cmd, args, caller.as_deref())
}

/// Legacy modules after CapProviders. `dialog.*` / `notification.*` / `store.*` /
/// `webview.*` are claimed by CapProviders first; chrome still owns tray/window bits
/// and `dialog.message` fallback is routed via DialogProvider.
const LEGACY_MODULES: &[ExtraDispatch] = &[
    core::try_dispatch,
    chrome::try_dispatch,
    window_extra::try_dispatch,
    clipboard::try_dispatch,
    shortcut::try_dispatch,
    shell_os::try_dispatch,
    store_auto::try_dispatch, // autostart.* / updater.* (store.* already CapProvider)
    power_process::try_dispatch,
    secrets_auth::try_dispatch,
    menu_set::try_dispatch,
];

fn dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: serde_json::Value,
    caller: Option<&str>,
) -> Result<serde_json::Value, String> {
    let prev = windows::caller_label();
    if caller.is_some() {
        windows::set_caller_label(caller);
    }
    let result = dispatch_inner(app, state, cmd, args);
    windows::set_caller_label(prev.as_deref());
    result
}

fn dispatch_inner(
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
                eprintln!("tishlang_desktop: tick emit failed: {e}");
            }
        }
    });
}
