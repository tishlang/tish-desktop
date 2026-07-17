//! Extra window chrome/geometry commands: `window.minimize`, `window.maximize`, …

use serde_json::{json, Value};
use tauri::{AppHandle, PhysicalPosition, PhysicalSize};
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

use super::helpers::{catch_err, ensure_permission, window_for};
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "window.minimize" => (|| {
            window_for(app, args)?.minimize().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.maximize" => (|| {
            window_for(app, args)?.maximize().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.unmaximize" => (|| {
            window_for(app, args)?
                .unmaximize()
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.fullscreen" => (|| {
            let fullscreen = args
                .get("fullscreen")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            window_for(app, args)?
                .set_fullscreen(fullscreen)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "fullscreen": fullscreen }))
        })(),
        "window.isFullscreen" => (|| {
            let v = window_for(app, args)?
                .is_fullscreen()
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "fullscreen": v }))
        })(),
        "window.setPosition" => (|| {
            let x = args.get("x").and_then(|v| v.as_f64()).ok_or("x required")?;
            let y = args.get("y").and_then(|v| v.as_f64()).ok_or("y required")?;
            window_for(app, args)?
                .set_position(PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "x": x, "y": y }))
        })(),
        "window.getPosition" => (|| {
            let pos = window_for(app, args)?
                .outer_position()
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "x": pos.x, "y": pos.y }))
        })(),
        "window.setSize" => (|| {
            let width = args
                .get("width")
                .and_then(|v| v.as_f64())
                .ok_or("width required")?;
            let height = args
                .get("height")
                .and_then(|v| v.as_f64())
                .ok_or("height required")?;
            window_for(app, args)?
                .set_size(PhysicalSize::new(width as u32, height as u32))
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "width": width, "height": height }))
        })(),
        "window.getSize" => (|| {
            let size = window_for(app, args)?
                .outer_size()
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "width": size.width, "height": size.height }))
        })(),
        "window.center" => (|| {
            window_for(app, args)?.center().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.setAlwaysOnTop" => (|| {
            let v = args
                .get("alwaysOnTop")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            window_for(app, args)?
                .set_always_on_top(v)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "alwaysOnTop": v }))
        })(),
        "window.setResizable" => (|| {
            let v = args
                .get("resizable")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            window_for(app, args)?
                .set_resizable(v)
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true, "resizable": v }))
        })(),
        "window.setFocus" => (|| {
            window_for(app, args)?
                .set_focus()
                .map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.print" => (|| {
            window_for(app, args)?.print().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.reload" => (|| {
            window_for(app, args)?.reload().map_err(|e| e.to_string())?;
            Ok(json!({ "ok": true }))
        })(),
        "window.openDevtools" => (|| {
            window_for(app, args)?.open_devtools();
            Ok(json!({ "ok": true }))
        })(),
        "windowState.save" => window_state_save(app, state),
        "windowState.restore" => window_state_restore(app, state, args),
        "windowState.enabled" => window_state_enabled(state),
        _ => return None,
    };
    Some(result)
}

fn window_state_enabled(state: &AppState) -> Result<Value, String> {
    Ok(json!({
        "ok": true,
        "enabled": state.has_permission("window-state"),
    }))
}

fn window_state_save(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "window-state")?;
    catch_err("windowState.save", || {
        app.save_window_state(StateFlags::all())
            .map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true }))
    })
}

fn window_state_restore(
    app: &AppHandle,
    state: &AppState,
    args: &Value,
) -> Result<Value, String> {
    ensure_permission(state, "window-state")?;
    catch_err("windowState.restore", || {
        let win = window_for(app, args)?;
        win.restore_state(StateFlags::all())
            .map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true }))
    })
}
