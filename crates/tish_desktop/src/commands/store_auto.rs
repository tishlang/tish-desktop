//! `store.*` (tauri-plugin-store), `autostart.*`, `updater.*` (best-effort,
//! graceful error if the updater isn't configured in `tauri.conf.json`).

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_store::StoreExt;
use tauri_plugin_updater::UpdaterExt;

use super::helpers::{catch_err, ensure_permission};
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "store.get" => store_get(app, state, args),
        "store.set" => store_set(app, state, args),
        "store.delete" => store_delete(app, state, args),
        "store.keys" => store_keys(app, state, args),
        "store.clear" => store_clear(app, state, args),
        "autostart.enable" => autostart_enable(app, state),
        "autostart.disable" => autostart_disable(app, state),
        "autostart.isEnabled" => autostart_is_enabled(app, state),
        "updater.check" => updater_check(app, state),
        "updater.downloadAndInstall" => updater_download_and_install(app, state),
        "updater.currentVersion" => updater_current_version(app, state),
        _ => return None,
    };
    Some(result)
}

fn store_path(args: &Value) -> String {
    args.get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("store.json")
        .to_string()
}

fn store_get(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "store")?;
    let path = store_path(args);
    let key = args.get("key").and_then(|v| v.as_str()).ok_or("key required")?.to_string();
    catch_err("store.get", move || {
        let store = app.store(&path).map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true, "value": store.get(&key).unwrap_or(Value::Null) }))
    })
}

fn store_set(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "store")?;
    let path = store_path(args);
    let key = args.get("key").and_then(|v| v.as_str()).ok_or("key required")?.to_string();
    let value = args.get("value").cloned().unwrap_or(Value::Null);
    catch_err("store.set", move || {
        let store = app.store(&path).map_err(|e| e.to_string())?;
        store.set(key, value);
        store.save().map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true }))
    })
}

fn store_delete(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "store")?;
    let path = store_path(args);
    let key = args.get("key").and_then(|v| v.as_str()).ok_or("key required")?.to_string();
    catch_err("store.delete", move || {
        let store = app.store(&path).map_err(|e| e.to_string())?;
        let deleted = store.delete(&key);
        store.save().map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true, "deleted": deleted }))
    })
}

fn store_keys(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "store")?;
    let path = store_path(args);
    catch_err("store.keys", move || {
        let store = app.store(&path).map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true, "keys": store.keys() }))
    })
}

fn store_clear(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "store")?;
    let path = store_path(args);
    catch_err("store.clear", move || {
        let store = app.store(&path).map_err(|e| e.to_string())?;
        store.clear();
        store.save().map_err(|e| e.to_string())?;
        Ok(json!({ "ok": true }))
    })
}

fn autostart_enable(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "autostart")?;
    catch_err("autostart.enable", || {
        app.autolaunch().enable().map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true }))
}

fn autostart_disable(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "autostart")?;
    catch_err("autostart.disable", || {
        app.autolaunch().disable().map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true }))
}

fn autostart_is_enabled(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "autostart")?;
    let enabled = catch_err("autostart.isEnabled", || {
        app.autolaunch().is_enabled().map_err(|e| e.to_string())
    })?;
    Ok(json!({ "ok": true, "enabled": enabled }))
}

fn updater_current_version(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "updater")?;
    Ok(json!({ "ok": true, "version": app.package_info().version.to_string() }))
}

fn updater_check(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "updater")?;
    let updater = catch_err("updater.check", || {
        app.updater().map_err(|e| e.to_string())
    })
    .map_err(|_| "updater not configured — set plugins.updater: true and configure tauri.conf.json".to_string())?;
    let update = tauri::async_runtime::block_on(updater.check()).map_err(|e| e.to_string())?;
    match update {
        Some(u) => Ok(json!({
            "ok": true,
            "available": true,
            "currentVersion": u.current_version,
            "version": u.version,
            "body": u.body,
        })),
        None => Ok(json!({ "ok": true, "available": false })),
    }
}

fn updater_download_and_install(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "updater")?;
    let updater = catch_err("updater.downloadAndInstall", || {
        app.updater().map_err(|e| e.to_string())
    })
    .map_err(|_| "updater not configured — set plugins.updater: true and configure tauri.conf.json".to_string())?;
    let update = tauri::async_runtime::block_on(updater.check()).map_err(|e| e.to_string())?;
    let Some(update) = update else {
        return Err("no update available".into());
    };
    let app2 = app.clone();
    tauri::async_runtime::block_on(update.download_and_install(
        move |chunk_len, content_len| {
            let _ = app2.emit(
                "updater:progress",
                json!({
                    "chunkLength": chunk_len,
                    "contentLength": content_len,
                }),
            );
        },
        || {},
    ))
    .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "installed": true }))
}
