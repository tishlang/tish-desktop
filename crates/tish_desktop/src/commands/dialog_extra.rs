//! Dialog commands beyond the basic `dialog.message`: file pickers, save,
//! confirm/ask. All gated on the `dialog` permission + plugin presence.

use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use super::helpers::ensure_dialog;
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "dialog.open" => dialog_open(app, state, args),
        "dialog.save" => dialog_save(app, state, args),
        "dialog.confirm" => dialog_confirm(app, state, args),
        "dialog.ask" => dialog_ask(app, state, args),
        _ => return None,
    };
    Some(result)
}

fn apply_filters(
    mut builder: tauri_plugin_dialog::FileDialogBuilder<tauri::Wry>,
    args: &Value,
) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    if let Some(filters) = args.get("filters").and_then(|v| v.as_array()) {
        for f in filters {
            let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("Files");
            let exts: Vec<&str> = f
                .get("extensions")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if !exts.is_empty() {
                builder = builder.add_filter(name, &exts);
            }
        }
    }
    if let Some(dir) = args.get("defaultPath").and_then(|v| v.as_str()) {
        builder = builder.set_directory(dir);
    }
    builder
}

fn dialog_open(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_dialog(app, state)?;
    let directory = args
        .get("directory")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let multiple = args
        .get("multiple")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let builder = apply_filters(app.dialog().file(), args);

    if directory {
        if multiple {
            let picked = builder.blocking_pick_folders();
            let paths: Vec<String> = picked
                .into_iter()
                .flatten()
                .filter_map(|p| p.into_path().ok())
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            Ok(json!({ "ok": true, "paths": paths }))
        } else {
            let picked = builder.blocking_pick_folder();
            let path = picked
                .and_then(|p| p.into_path().ok())
                .map(|p| p.to_string_lossy().to_string());
            Ok(json!({ "ok": true, "path": path }))
        }
    } else if multiple {
        let picked = builder.blocking_pick_files();
        let paths: Vec<String> = picked
            .into_iter()
            .flatten()
            .filter_map(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        Ok(json!({ "ok": true, "paths": paths }))
    } else {
        let picked = builder.blocking_pick_file();
        let path = picked
            .and_then(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().to_string());
        Ok(json!({ "ok": true, "path": path }))
    }
}

fn dialog_save(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_dialog(app, state)?;
    let mut builder = apply_filters(app.dialog().file(), args);
    if let Some(name) = args.get("defaultName").and_then(|v| v.as_str()) {
        builder = builder.set_file_name(name);
    }
    let picked = builder.blocking_save_file();
    let path = picked
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string());
    Ok(json!({ "ok": true, "path": path }))
}

fn dialog_confirm(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_dialog(app, state)?;
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Confirm");
    let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let confirmed = app
        .dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancel)
        .blocking_show();
    Ok(json!({ "ok": true, "confirmed": confirmed }))
}

fn dialog_ask(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_dialog(app, state)?;
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Question");
    let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let answer = app
        .dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();
    Ok(json!({ "ok": true, "answer": answer }))
}
