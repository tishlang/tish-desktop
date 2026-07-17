use serde_json::json;
use tauri::menu::{Menu, MenuItem};
use tauri::plugin::PermissionState;
use tauri::window::{ProgressBarState, ProgressBarStatus};
use tauri::{AppHandle, Emitter, Manager, State, UserAttentionType};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::fs_sandbox;
use crate::state::AppState;
use crate::windows;

fn catch_err<T>(label: &str, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => r,
        Err(_) => Err(format!(
            "{label} panicked — on macOS, allow notifications for this app (System Settings → Notifications)"
        )),
    }
}

fn permission_state_str(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
        PermissionState::Prompt | PermissionState::PromptWithRationale => "prompt",
    }
}

fn ensure_notification_plugin(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if !state.has_permission("notification") {
        return Err("notification permission denied".into());
    }
    // Avoid NotificationExt::notification() → state() panic when plugin wasn't .init()'d.
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

fn window_for<'a>(app: &'a AppHandle, args: &serde_json::Value) -> Result<tauri::WebviewWindow, String> {
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("main");
    app.get_webview_window(label)
        .ok_or_else(|| format!("window not found: {label}"))
}

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

fn dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Custom Tish-registered handlers first
    if state.handlers.lock().contains_key(cmd) {
        return state.call_handler(cmd, args);
    }

    match cmd {
        "ping" => Ok(json!({
            "ok": true,
            "ts": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            "protocol": crate::state::PROTOCOL_VERSION,
        })),
        "window.list" => Ok(json!(windows::list_labels(app))),
        "window.focus" => {
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .ok_or("label required")?;
            windows::focus(app, label)?;
            Ok(json!({ "ok": true }))
        }
        "window.close" => {
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .ok_or("label required")?;
            windows::close(app, label)?;
            Ok(json!({ "ok": true }))
        }
        "window.create" => {
            let spec: crate::state::WindowSpec = serde_json::from_value(args)
                .map_err(|e| e.to_string())?;
            windows::create_from_spec(app, &spec)?;
            Ok(json!({ "ok": true, "label": spec.label }))
        }
        "fs.list" => {
            let root = state.fs_root.lock().clone().ok_or("fs root not set")?;
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let entries = fs_sandbox::list_dir(&root, path)?;
            Ok(json!({ "ok": true, "entries": entries }))
        }
        "fs.readText" => {
            let root = state.fs_root.lock().clone().ok_or("fs root not set")?;
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("path required")?;
            let text = fs_sandbox::read_text(&root, path)?;
            Ok(json!({ "ok": true, "text": text }))
        }
        "fs.stat" => {
            let root = state.fs_root.lock().clone().ok_or("fs root not set")?;
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("path required")?;
            let st = fs_sandbox::stat_path(&root, path)?;
            Ok(json!({ "ok": true, "stat": st }))
        }
        "fs.watch" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            fs_sandbox::start_watch(app.clone(), state, path)?;
            Ok(json!({ "ok": true }))
        }
        "fs.unwatch" => {
            fs_sandbox::stop_watch(state);
            Ok(json!({ "ok": true }))
        }
        "extensions.list" => Ok(crate::extensions::REGISTRY.list()),
        "dialog.message" => {
            if !state.has_permission("dialog") {
                return Err("dialog permission denied".into());
            }
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Message");
            let message = args
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            app.dialog()
                .message(message)
                .title(title)
                .kind(MessageDialogKind::Info)
                .buttons(MessageDialogButtons::Ok)
                .blocking_show();
            Ok(json!({ "ok": true }))
        }
        "dialog.open" => {
            if !state.has_permission("dialog") {
                return Err("dialog permission denied".into());
            }
            let directory = args
                .get("directory")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mut builder = app.dialog().file();
            if directory {
                builder = builder.set_directory("/");
            }
            let picked = builder.blocking_pick_file();
            Ok(json!({
                "ok": true,
                "path": picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string()),
            }))
        }

        // ── Dock / taskbar badge (macOS often requires notification authorization) ──
        "dock.badge" => {
            let win = window_for(app, &args)?;
            let count = args.get("count").and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_f64().map(|f| f as i64))
                    .or_else(|| v.as_u64().map(|u| u as i64))
            });
            // 0 / null clears the badge
            let value = match count {
                None | Some(0) => None,
                Some(n) => Some(n),
            };
            catch_err("dock.badge", || {
                win.set_badge_count(value).map_err(|e| e.to_string())
            })?;
            Ok(json!({ "ok": true, "count": value }))
        }
        "dock.badgeLabel" => {
            let win = window_for(app, &args)?;
            let label = args.get("label").and_then(|v| v.as_str()).map(|s| s.to_string());
            let empty = label.as_ref().map(|s| s.is_empty()).unwrap_or(true);
            let next = if empty { None } else { label.clone() };
            catch_err("dock.badgeLabel", || {
                win.set_badge_label(next.clone()).map_err(|e| e.to_string())
            })?;
            Ok(json!({ "ok": true, "label": next }))
        }

        // ── Window chrome ──
        "window.title" => {
            let win = window_for(app, &args)?;
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or("title required")?;
            win.set_title(title).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "window.titleBarStyle" => {
            let win = window_for(app, &args)?;
            let style = args
                .get("style")
                .and_then(|v| v.as_str())
                .ok_or("style required (visible|transparent|overlay)")?;
            #[cfg(target_os = "macos")]
            {
                win.set_title_bar_style(windows::parse_title_bar_style(style))
                    .map_err(|e| e.to_string())?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (win, style);
                return Err("window.titleBarStyle is macOS-only".into());
            }
            Ok(json!({ "ok": true, "style": style }))
        }
        "window.decorations" => {
            let win = window_for(app, &args)?;
            let decorations = args
                .get("decorations")
                .and_then(|v| v.as_bool())
                .ok_or("decorations bool required")?;
            win.set_decorations(decorations)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "decorations": decorations }))
        }
        "window.shadow" => {
            let win = window_for(app, &args)?;
            let enable = args
                .get("enable")
                .and_then(|v| v.as_bool())
                .ok_or("enable bool required")?;
            win.set_shadow(enable).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "enable": enable }))
        }
        "window.startDragging" => {
            let win = window_for(app, &args)?;
            win.start_dragging().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "window.progress" => {
            let win = window_for(app, &args)?;
            let progress = args
                .get("progress")
                .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)));
            let status_str = args
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or(if progress.is_some() { "normal" } else { "none" });
            let status = match status_str {
                "none" | "clear" => ProgressBarStatus::None,
                "indeterminate" => ProgressBarStatus::Indeterminate,
                "paused" => ProgressBarStatus::Paused,
                "error" => ProgressBarStatus::Error,
                _ => ProgressBarStatus::Normal,
            };
            win.set_progress_bar(ProgressBarState {
                status: Some(status),
                progress,
            })
            .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "status": status_str, "progress": progress }))
        }
        "window.attention" => {
            let win = window_for(app, &args)?;
            let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("informational");
            let attention = match kind {
                "critical" => Some(UserAttentionType::Critical),
                "none" | "clear" => None,
                _ => Some(UserAttentionType::Informational),
            };
            win.request_user_attention(attention)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "kind": kind }))
        }

        // ── Tray / menu-bar status item ──
        "tray.tooltip" => {
            if !state.has_permission("tray") {
                return Err("tray permission denied".into());
            }
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or("text required")?;
            let tray = state.tray.lock();
            let Some(tray) = tray.as_ref() else {
                return Err("tray not available".into());
            };
            tray.set_tooltip(Some(text)).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }
        "tray.title" => {
            if !state.has_permission("tray") {
                return Err("tray permission denied".into());
            }
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let tray = state.tray.lock();
            let Some(tray) = tray.as_ref() else {
                return Err("tray not available".into());
            };
            // macOS menu-bar status title next to the icon
            tray.set_title(Some(text)).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }

        // ── Context menu (native popup; selection arrives as menu:action) ──
        "menu.context" => {
            if !state.has_permission("menu") {
                return Err("menu permission denied".into());
            }
            let win = window_for(app, &args)?;
            let items = args
                .get("items")
                .and_then(|v| v.as_array())
                .ok_or("items array required")?;
            let menu = Menu::new(app).map_err(|e| e.to_string())?;
            for (i, item) in items.iter().enumerate() {
                if item.get("separator").and_then(|v| v.as_bool()).unwrap_or(false) {
                    let sep = tauri::menu::PredefinedMenuItem::separator(app)
                        .map_err(|e| e.to_string())?;
                    menu.append(&sep).map_err(|e| e.to_string())?;
                    continue;
                }
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("ctx:{i}"));
                let label = item
                    .get("label")
                    .and_then(|v| v.as_str())
                    .ok_or("item.label required")?;
                let enabled = item
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let mi = MenuItem::with_id(app, id, label, enabled, None::<&str>)
                    .map_err(|e| e.to_string())?;
                menu.append(&mi).map_err(|e| e.to_string())?;
            }
            win.popup_menu(&menu).map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }

        // ── Native notifications ──
        "notification.permissionState" => {
            ensure_notification_plugin(app, state)?;
            let state = catch_err("notification.permissionState", || {
                app.notification()
                    .permission_state()
                    .map_err(|e| e.to_string())
            })?;
            Ok(json!({ "state": permission_state_str(state) }))
        }
        "notification.requestPermission" => {
            ensure_notification_plugin(app, state)?;
            let state = catch_err("notification.requestPermission", || {
                app.notification()
                    .request_permission()
                    .map_err(|e| e.to_string())
            })?;
            Ok(json!({ "state": permission_state_str(state) }))
        }
        "notification.show" => {
            ensure_notification_plugin(app, state)?;
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish Desktop")
                .to_string();
            let body = args
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Request OS permission first (no-op/granted stub on some desktop builds).
            let perm = catch_err("notification.requestPermission", || {
                app.notification()
                    .request_permission()
                    .map_err(|e| e.to_string())
            })?;
            if perm == PermissionState::Denied {
                return Err(
                    "notifications denied — enable them in System Settings → Notifications"
                        .into(),
                );
            }

            // Prefer sync notify-rust on the calling thread. The plugin's show()
            // spawns an async task that has panicked/crashed some macOS builds.
            let title2 = title.clone();
            let body2 = body.clone();
            catch_err("notification.show", move || {
                #[cfg(target_os = "macos")]
                {
                    // Dev shells aren't a proper .app bundle; Terminal's id is the usual workaround.
                    let _ = notify_rust::set_application(if tauri::is_dev() {
                        "com.apple.Terminal"
                    } else {
                        "com.tishlang.desktop"
                    });
                }
                let mut n = notify_rust::Notification::new();
                n.summary(&title2);
                if !body2.is_empty() {
                    n.body(&body2);
                }
                n.show().map(|_| ()).map_err(|e| e.to_string())
            })?;
            Ok(json!({
                "ok": true,
                "permission": permission_state_str(perm)
            }))
        }

        // ── Opener ──
        "opener.open" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("url required")?;
            app.opener()
                .open_url(url, None::<&str>)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        }

        _ => Err(format!("unknown command: {cmd}")),
    }
}

#[tauri::command]
pub fn desktop_emit_tick(app: AppHandle) -> Result<(), String> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    app.emit("tick", json!({ "ts": ts })).map_err(|e| e.to_string())
}

pub fn spawn_tick_loop(app: AppHandle, interval_ms: u64) {
    // WKWebView eval (used by emit → JS listeners) must run on the main thread.
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(interval_ms));
        let app2 = app.clone();
        if app
            .run_on_main_thread(move || {
                if let Err(e) = desktop_emit_tick(app2) {
                    eprintln!("tish_desktop: tick emit failed: {e}");
                }
            })
            .is_err()
        {
            break;
        }
        if app.webview_windows().is_empty() {
            break;
        }
    });
}
