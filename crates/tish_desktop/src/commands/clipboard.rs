//! `clipboard.readText` / `writeText` / `clear` / `readImage` / `writeImage`.

use base64::Engine;
use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::helpers::{catch_err, ensure_clipboard};
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "clipboard.readText" => read_text(app, state),
        "clipboard.writeText" => write_text(app, state, args),
        "clipboard.clear" => clear(app, state),
        "clipboard.readImage" => read_image(app, state),
        "clipboard.writeImage" => write_image(app, state, args),
        _ => return None,
    };
    Some(result)
}

fn read_text(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_clipboard(app, state)?;
    let text = catch_err("clipboard.readText", || {
        app.clipboard().read_text().map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true, "text": text }))
}

fn write_text(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_clipboard(app, state)?;
    let text = args.get("text").and_then(|v| v.as_str()).ok_or("text required")?;
    catch_err("clipboard.writeText", || {
        app.clipboard().write_text(text).map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true }))
}

fn clear(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_clipboard(app, state)?;
    catch_err("clipboard.clear", || {
        app.clipboard().clear().map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true }))
}

fn read_image(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_clipboard(app, state)?;
    let (width, height, b64) = catch_err("clipboard.readImage", || {
        let image = app.clipboard().read_image().map_err(|e| e.to_string())?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(image.rgba());
        Ok((image.width(), image.height(), b64))
    })?;
    Ok(json!({ "ok": true, "width": width, "height": height, "rgba": b64 }))
}

fn write_image(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_clipboard(app, state)?;
    let width = args.get("width").and_then(|v| v.as_u64()).ok_or("width required")? as u32;
    let height = args.get("height").and_then(|v| v.as_u64()).ok_or("height required")? as u32;
    let b64 = args.get("rgba").and_then(|v| v.as_str()).ok_or("rgba (base64) required")?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| e.to_string())?;
    let image = tauri::image::Image::new_owned(bytes, width, height);
    catch_err("clipboard.writeImage", || {
        app.clipboard().write_image(&image).map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true }))
}
