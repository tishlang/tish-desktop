//! `menu.set` — replace the app-wide menu from a JSON `menus` array.
//!
//! Shape: `{ menus: [ { label, items: [ { id, label, accelerator?, enabled?,
//! checked?, separator?, predefined?, items? (nested submenu) }, ... ] }, ... ] }`.
//!
//! `predefined` builds a native role item instead of a custom one: `undo`, `redo`, `cut`,
//! `copy`, `paste`, `selectAll` (plus `minimize`, `hide`, `hideOthers`, `showAll`,
//! `fullscreen`, `closeWindow`, `quit`). On macOS these are NSMenuItems with the standard
//! selectors (`paste:` …) sent down the responder chain, so the focused WKWebView handles
//! them natively — no `execCommand("paste")` permission bubble. They emit no `menu:action`;
//! `label` optionally overrides the default title.
//! Selection arrives as the existing `menu:action` event (wired in `lib.rs`).

use serde_json::Value;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::AppHandle;

use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    if cmd != "menu.set" {
        return None;
    }
    Some(menu_set(app, state, args))
}

fn menu_set(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    if !state.has_permission("menu") {
        return Err("menu permission denied".into());
    }
    let menus = args.get("menus").and_then(|v| v.as_array()).ok_or("menus array required")?;

    let menu = Menu::new(app).map_err(|e| e.to_string())?;
    for spec in menus {
        let label = spec.get("label").and_then(|v| v.as_str()).unwrap_or("Menu");
        let submenu = build_submenu(app, label, spec)?;
        menu.append(&submenu).map_err(|e| e.to_string())?;
    }
    app.set_menu(menu).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "ok": true }))
}

fn build_submenu(app: &AppHandle, label: &str, spec: &Value) -> Result<Submenu<tauri::Wry>, String> {
    let submenu = Submenu::new(app, label, true).map_err(|e| e.to_string())?;
    if let Some(items) = spec.get("items").and_then(|v| v.as_array()) {
        for item in items {
            append_item(app, &submenu, item)?;
        }
    }
    Ok(submenu)
}

fn append_item(app: &AppHandle, parent: &Submenu<tauri::Wry>, item: &Value) -> Result<(), String> {
    if item.get("separator").and_then(|v| v.as_bool()).unwrap_or(false) {
        let sep = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
        return parent.append(&sep).map_err(|e| e.to_string());
    }

    if let Some(role) = item.get("predefined").and_then(|v| v.as_str()) {
        let text = item.get("label").and_then(|v| v.as_str());
        let mi = predefined_item(app, role, text)?;
        return parent.append(&mi).map_err(|e| e.to_string());
    }

    let label = item.get("label").and_then(|v| v.as_str()).unwrap_or("Item").to_string();

    if item.get("items").and_then(|v| v.as_array()).is_some() {
        let inner = build_submenu(app, &label, item)?;
        return parent.append(&inner).map_err(|e| e.to_string());
    }

    let id = item
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("menu:{label}"));
    let enabled = item.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
    let accelerator = item.get("accelerator").and_then(|v| v.as_str());

    if let Some(checked) = item.get("checked").and_then(|v| v.as_bool()) {
        let mi = CheckMenuItem::with_id(app, id, label, enabled, checked, accelerator)
            .map_err(|e| e.to_string())?;
        parent.append(&mi).map_err(|e| e.to_string())
    } else {
        let mi = MenuItem::with_id(app, id, label, enabled, accelerator).map_err(|e| e.to_string())?;
        parent.append(&mi).map_err(|e| e.to_string())
    }
}

pub(super) fn predefined_item(
    app: &AppHandle,
    role: &str,
    text: Option<&str>,
) -> Result<PredefinedMenuItem<tauri::Wry>, String> {
    let r = match role {
        "undo" => PredefinedMenuItem::undo(app, text),
        "redo" => PredefinedMenuItem::redo(app, text),
        "cut" => PredefinedMenuItem::cut(app, text),
        "copy" => PredefinedMenuItem::copy(app, text),
        "paste" => PredefinedMenuItem::paste(app, text),
        "selectAll" | "select_all" => PredefinedMenuItem::select_all(app, text),
        "minimize" => PredefinedMenuItem::minimize(app, text),
        "hide" => PredefinedMenuItem::hide(app, text),
        "hideOthers" | "hide_others" => PredefinedMenuItem::hide_others(app, text),
        "showAll" | "show_all" => PredefinedMenuItem::show_all(app, text),
        "fullscreen" => PredefinedMenuItem::fullscreen(app, text),
        "closeWindow" | "close_window" => PredefinedMenuItem::close_window(app, text),
        "quit" => PredefinedMenuItem::quit(app, text),
        "separator" => PredefinedMenuItem::separator(app),
        other => return Err(format!("unknown predefined menu item: {other}")),
    };
    r.map_err(|e| e.to_string())
}
