//! Dock badge, window chrome, tray, context menu, notifications, opener.

use serde_json::{json, Value};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::plugin::PermissionState;
use tauri::window::{ProgressBarState, ProgressBarStatus};
use tauri::{AppHandle, UserAttentionType};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use super::helpers::{
    catch_err, ensure_notification, ensure_opener, permission_state_str, window_for,
};
use crate::state::AppState;
use crate::windows;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "dialog.message" => dialog_message(app, state, args),
        "dock.badge" => dock_badge(app, args),
        "dock.badgeLabel" => dock_badge_label(app, args),
        "dock.setIcon" => dock_set_icon(app, args),
        "window.title" => window_title(app, args),
        "window.titleBarStyle" => window_title_bar_style(app, args),
        "window.decorations" => window_decorations(app, args),
        "window.shadow" => window_shadow(app, args),
        "window.startDragging" => window_start_dragging(app, args),
        "window.progress" => window_progress(app, args),
        "window.attention" => window_attention(app, args),
        "tray.tooltip" => tray_tooltip(state, args),
        "tray.title" => tray_title(state, args),
        "tray.setMenu" => tray_set_menu(app, state, args),
        "menu.context" => menu_context(app, state, args),
        "notification.permissionState" => notification_permission_state(app, state),
        "notification.requestPermission" => notification_request_permission(app, state),
        "notification.show" => notification_show(app, state, args),
        "opener.open" => opener_open(app, state, args),
        // macOS traffic-light chrome (theme-driven inset + color tint); no-op off apple/macos.
        "window.setTransparent" => window_set_transparent(app, args),
        "window.trafficLightInset" => traffic_light_inset(app, args),
        "window.reapplyTrafficLightInset" => reapply_traffic_light_inset(app),
        "window.trafficLightTint" => traffic_light_tint_cmd(app, args),
        "window.reapplyTrafficLightTint" => reapply_traffic_light_tint_cmd(app),
        _ => return None,
    };
    Some(result)
}

// ---- traffic-light chrome (apple/macos does the work; everything else is a successful no-op) ----

/// `window.setTransparent({ transparent })` — flip window transparency at runtime on every managed
/// window: NSWindow.isOpaque + background color + the WKWebView's drawsBackground, then refresh the
/// shadow. Lets hosts default to the CHEAP opaque window and opt into see-through chrome only while
/// a theme wants it — a permanently transparent window recomposites every display frame on macOS
/// and feeds the macOS 26 WebKit IOAccelerator slab leak. No-op off apple/macos.
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn window_set_transparent(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let transparent = args
        .get("transparent")
        .and_then(|v| v.as_bool())
        .ok_or("transparent bool required")?;
    crate::window_transparent::set_transparent(app, transparent);
    Ok(json!({ "ok": true, "transparent": transparent }))
}
#[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
fn window_set_transparent(_app: &AppHandle, _args: &Value) -> Result<Value, String> {
    Ok(json!({ "ok": true, "unsupported": true }))
}

#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn traffic_light_inset(app: &AppHandle, args: &Value) -> Result<Value, String> {
    crate::traffic_lights::set_inset(
        app,
        args.get("padX").and_then(|v| v.as_f64()),
        args.get("padY").and_then(|v| v.as_f64()),
        args.get("spacing").and_then(|v| v.as_f64()),
    );
    Ok(json!({ "ok": true }))
}
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn reapply_traffic_light_inset(app: &AppHandle) -> Result<Value, String> {
    crate::traffic_lights::reapply(app);
    Ok(json!({ "ok": true }))
}
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn traffic_light_tint_cmd(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
    crate::traffic_light_tint::set_tint(
        app,
        s("close"),
        s("minimize"),
        s("zoom"),
        args.get("diameter").and_then(|v| v.as_f64()),
        args.get("opacity").and_then(|v| v.as_f64()),
    );
    Ok(json!({ "ok": true }))
}
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn reapply_traffic_light_tint_cmd(app: &AppHandle) -> Result<Value, String> {
    crate::traffic_light_tint::reapply(app);
    Ok(json!({ "ok": true }))
}

#[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
fn traffic_light_inset(_app: &AppHandle, _args: &Value) -> Result<Value, String> {
    Ok(json!({ "ok": true }))
}
#[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
fn reapply_traffic_light_inset(_app: &AppHandle) -> Result<Value, String> {
    Ok(json!({ "ok": true }))
}
#[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
fn traffic_light_tint_cmd(_app: &AppHandle, _args: &Value) -> Result<Value, String> {
    Ok(json!({ "ok": true }))
}
#[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
fn reapply_traffic_light_tint_cmd(_app: &AppHandle) -> Result<Value, String> {
    Ok(json!({ "ok": true }))
}

fn dialog_message(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    if !state.has_permission("dialog") {
        return Err("dialog permission denied".into());
    }
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Message");
    let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::Ok)
        .blocking_show();
    Ok(json!({ "ok": true }))
}

fn dock_badge(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let count = args.get("count").and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_u64().map(|u| u as i64))
    });
    let value = match count {
        None | Some(0) => None,
        Some(n) => Some(n),
    };
    catch_err("dock.badge", || {
        win.set_badge_count(value).map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true, "count": value }))
}

fn dock_badge_label(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let empty = label.as_ref().map(|s| s.is_empty()).unwrap_or(true);
    let next = if empty { None } else { label.clone() };
    // `set_badge_label` is macOS-only in Tauri (unlike `set_badge_count`, which is cross-platform);
    // no-op elsewhere so tishlang_desktop still compiles for linux/windows.
    #[cfg(target_os = "macos")]
    catch_err("dock.badgeLabel", || {
        win.set_badge_label(next.clone()).map_err(|e| e.to_string())
    })?;
    #[cfg(not(target_os = "macos"))]
    let _ = &win;
    Ok(json!({ "ok": true, "label": next }))
}

/// `dock.setIcon({ path })` — change the macOS dock icon at runtime from a PNG (or any
/// NSImage-readable) file. Dispatches to the main thread (AppKit requires it; broker handlers run
/// off-main). A successful no-op on non-macOS. Generic: any app on tish-desktop can rebrand its dock
/// icon on the fly (state/notification variants), complementing the startup `RunConfig.icon`.
#[allow(unused_variables)]
fn dock_set_icon(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("path required")?
        .to_string();
    #[cfg(all(feature = "platform-apple", target_os = "macos"))]
    {
        let handle = app.clone();
        handle
            .run_on_main_thread(move || {
                let _ = crate::app_icon::set_dock_icon(&path);
            })
            .map_err(|e| e.to_string())?;
        return Ok(json!({ "ok": true }));
    }
    #[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
    Ok(json!({ "ok": true, "unsupported": true }))
}

fn window_title(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("title required")?;
    win.set_title(title).map_err(|e| e.to_string())?;
    crate::windows::note_title(win.label(), title);
    Ok(json!({ "ok": true }))
}

fn window_title_bar_style(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
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

fn window_decorations(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let decorations = args
        .get("decorations")
        .and_then(|v| v.as_bool())
        .ok_or("decorations bool required")?;
    win.set_decorations(decorations)
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "decorations": decorations }))
}

fn window_shadow(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let enable = args
        .get("enable")
        .and_then(|v| v.as_bool())
        .ok_or("enable bool required")?;
    win.set_shadow(enable).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "enable": enable }))
}

fn window_start_dragging(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    win.start_dragging().map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn window_progress(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let progress = args
        .get("progress")
        .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)));
    let status_str = args
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or(if progress.is_some() {
            "normal"
        } else {
            "none"
        });
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

fn window_attention(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let win = window_for(app, args)?;
    let kind = args
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("informational");
    let attention = match kind {
        "critical" => Some(UserAttentionType::Critical),
        "none" | "clear" => None,
        _ => Some(UserAttentionType::Informational),
    };
    win.request_user_attention(attention)
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "kind": kind }))
}

/// Rebuild the tray's context menu from a host-supplied item list. Each item is
/// `{ id, label, enabled? }` or `{ separator: true }`. Lets a hosted app own a dynamic tray menu
/// instead of the runtime's default. Item clicks fire through the tray's on_menu_event (set at
/// build time), which emits a generic `tray:action` { id } for the host to interpret.
fn tray_set_menu(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    if !state.has_permission("tray") {
        return Err("tray permission denied".into());
    }
    let items = args
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("items required")?;
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    for it in items {
        if it.get("separator").and_then(|v| v.as_bool()).unwrap_or(false) {
            let sep = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
            menu.append(&sep).map_err(|e| e.to_string())?;
            continue;
        }
        let id = it.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let label = it.get("label").and_then(|v| v.as_str()).unwrap_or("");
        let enabled = it.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
        let mi =
            MenuItem::with_id(app, id, label, enabled, None::<&str>).map_err(|e| e.to_string())?;
        menu.append(&mi).map_err(|e| e.to_string())?;
    }
    let tray = state.tray.lock();
    let Some(tray) = tray.as_ref() else {
        return Err("tray not available".into());
    };
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn tray_tooltip(state: &AppState, args: &Value) -> Result<Value, String> {
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

fn tray_title(state: &AppState, args: &Value) -> Result<Value, String> {
    if !state.has_permission("tray") {
        return Err("tray permission denied".into());
    }
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let tray = state.tray.lock();
    let Some(tray) = tray.as_ref() else {
        return Err("tray not available".into());
    };
    tray.set_title(Some(text)).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn menu_context(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    if !state.has_permission("menu") {
        return Err("menu permission denied".into());
    }
    let win = window_for(app, args)?;
    let items = args
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or("items array required")?;
    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    for (i, item) in items.iter().enumerate() {
        if item
            .get("separator")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let sep =
                tauri::menu::PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
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

fn notification_permission_state(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_notification(app, state)?;
    let state = catch_err("notification.permissionState", || {
        app.notification()
            .permission_state()
            .map_err(|e| e.to_string())
    })?;
    Ok(json!({ "state": permission_state_str(state) }))
}

fn notification_request_permission(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_notification(app, state)?;
    let state = catch_err("notification.requestPermission", || {
        app.notification()
            .request_permission()
            .map_err(|e| e.to_string())
    })?;
    Ok(json!({ "state": permission_state_str(state) }))
}

fn notification_show(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_notification(app, state)?;
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

    let perm = catch_err("notification.requestPermission", || {
        app.notification()
            .request_permission()
            .map_err(|e| e.to_string())
    })?;
    if perm == PermissionState::Denied {
        return Err(
            "notifications denied — enable them in System Settings → Notifications".into(),
        );
    }

    let title2 = title.clone();
    let body2 = body.clone();
    catch_err("notification.show", move || {
        #[cfg(target_os = "macos")]
        {
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

fn opener_open(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_opener(app, state)?;
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("url required")?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}
