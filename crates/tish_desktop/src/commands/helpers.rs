//! Shared helpers for broker command modules: permission/plugin gating,
//! window resolution, and panic-safe execution for flaky OS/plugin APIs.

use tauri::plugin::PermissionState;
use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Runs `f`, converting a panic (e.g. an unconfigured OS API) into an `Err`
/// instead of crashing the whole app.
pub fn catch_err<T>(label: &str, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(_) => Err(format!(
            "{label} panicked — the underlying plugin/OS API is likely unavailable or not enabled"
        )),
    }
}

pub fn permission_state_str(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
        PermissionState::Prompt | PermissionState::PromptWithRationale => "prompt",
    }
}

/// Resolves the target window: `args.label`, else the invoking webview's label.
/// Never a silent `"main"` fallback unless the caller really is `main`.
pub fn window_for(
    app: &AppHandle,
    args: &serde_json::Value,
) -> Result<tauri::WebviewWindow, String> {
    crate::windows::resolve(app, args, crate::windows::caller_label().as_deref())
}

/// Checks that `name` is present in the app's granted permission list.
pub fn ensure_permission(state: &AppState, name: &str) -> Result<(), String> {
    if !state.has_permission(name) {
        return Err(format!("{name} permission denied"));
    }
    Ok(())
}

/// Checks the `dialog` permission and that the dialog plugin was `.init()`'d.
pub fn ensure_dialog(app: &AppHandle, state: &AppState) -> Result<(), String> {
    ensure_permission(state, "dialog")?;
    if app
        .try_state::<tauri_plugin_dialog::Dialog<tauri::Wry>>()
        .is_none()
    {
        return Err("dialog plugin not enabled — set plugins.dialog: true in run()".into());
    }
    Ok(())
}

/// Checks the `opener` permission and that the opener plugin was `.init()`'d.
pub fn ensure_opener(app: &AppHandle, state: &AppState) -> Result<(), String> {
    ensure_permission(state, "opener")?;
    if app
        .try_state::<tauri_plugin_opener::Opener<tauri::Wry>>()
        .is_none()
    {
        return Err("opener plugin not enabled — set plugins.opener: true in run()".into());
    }
    Ok(())
}

/// Checks the `notification` permission and that the plugin was `.init()`'d.
pub fn ensure_notification(app: &AppHandle, state: &AppState) -> Result<(), String> {
    ensure_permission(state, "notification")?;
    if app
        .try_state::<tauri_plugin_notification::Notification<tauri::Wry>>()
        .is_none()
    {
        return Err(
            "notification plugin not enabled — set plugins.notification: true in run()".into(),
        );
    }
    Ok(())
}

/// Checks the `clipboard` permission and that the plugin was `.init()`'d.
pub fn ensure_clipboard(app: &AppHandle, state: &AppState) -> Result<(), String> {
    ensure_permission(state, "clipboard")?;
    if app
        .try_state::<tauri_plugin_clipboard_manager::Clipboard<tauri::Wry>>()
        .is_none()
    {
        return Err("clipboard plugin not enabled — set plugins.clipboard: true in run()".into());
    }
    Ok(())
}

/// Checks the `global-shortcut` permission and that the plugin was `.init()`'d.
pub fn ensure_global_shortcut(app: &AppHandle, state: &AppState) -> Result<(), String> {
    ensure_permission(state, "global-shortcut")?;
    if app
        .try_state::<tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>>()
        .is_none()
    {
        return Err(
            "global-shortcut plugin not enabled — set plugins.globalShortcut: true in run()"
                .into(),
        );
    }
    Ok(())
}
