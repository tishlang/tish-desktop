//! In-process broker invoke for shell / native WK bridge (no Tauri AppHandle required).
//!
//! Plan dispatch order (subset): handlers → state.* → unsupported for desktop-only caps.

use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::{json, Value as Json};
use tauri::AppHandle;
use tishlang_core::{value_call, Value};

use crate::broker::{self, GLOBAL_SHARED_STATE};
use crate::state::PENDING_HANDLERS;
use crate::value_util::{arg_object_json, arg_str, err_str, json_to_value, ok_json, value_to_json};

/// Optional Tauri handle for multi-transport emit after `run()` setup.
pub static APP_HANDLE: once_cell::sync::Lazy<Mutex<Option<AppHandle>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

pub fn set_app_handle(app: AppHandle) {
    *APP_HANDLE.lock() = Some(app);
}

/// `brokerInvoke(cmd, args?)` — shell / WK `onBridgeInvoke` entry.
pub fn broker_invoke(args: &[Value]) -> Value {
    let Some(cmd) = arg_str(args, 0) else {
        return err_str("brokerInvoke(cmd, args?): cmd required");
    };
    let args_json = arg_object_json(args, 1).unwrap_or(json!({}));
    match invoke_local(&cmd, args_json) {
        Ok(v) => ok_json(v),
        Err(e) => err_str(e),
    }
}

pub fn invoke_local(cmd: &str, args: Json) -> Result<Json, String> {
    // 1. Shell handlers (clone Arc; do not take)
    if let Some(f) = PENDING_HANDLERS.lock().get(cmd).cloned() {
        let arg = json_to_value(&args);
        let result = value_call(&Value::Function(f), &[arg]);
        return value_to_json(&result).ok_or_else(|| "handler returned non-JSON value".into());
    }

    // 2. state.*
    if let Some(r) = dispatch_state(cmd, &args) {
        return r;
    }

    // 3. notification.* — platform native backends when no Tauri AppHandle path
    if cmd.starts_with("notification.") {
        return platform_notification(cmd, &args);
    }

    // 4. Other desktop-only caps without AppHandle → unsupported
    if cmd.starts_with("tray.")
        || cmd.starts_with("dialog.")
        || cmd.starts_with("store.")
        || cmd.starts_with("webview.")
        || cmd.starts_with("window.")
    {
        return Ok(broker::unsupported(cmd.split('.').next().unwrap_or(cmd)));
    }

    Err(format!("unknown command: {cmd}"))
}

fn dispatch_state(cmd: &str, args: &Json) -> Option<Result<Json, String>> {
    let path = || {
        args.get("path")
            .or_else(|| args.get("key"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "path required".into())
    };
    let source = args
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("native")
        .to_string();

    match cmd {
        "state.get" => {
            let path = match path() {
                Ok(p) => p,
                Err(e) => return Some(Err(e)),
            };
            let (value, revision) = GLOBAL_SHARED_STATE.get(&path);
            Some(Ok(json!({
                "ok": true,
                "path": path,
                "value": value,
                "revision": revision,
            })))
        }
        "state.set" => {
            let path = match path() {
                Ok(p) => p,
                Err(e) => return Some(Err(e)),
            };
            let value = args.get("value").cloned().unwrap_or(Json::Null);
            let out = GLOBAL_SHARED_STATE.set(path.clone(), value.clone());
            let revision = out.get("revision").and_then(|v| v.as_u64()).unwrap_or(0);
            emit_state_multi(&path, &value, revision, &source);
            Some(Ok(out))
        }
        "state.patch" => {
            let path = match path() {
                Ok(p) => p,
                Err(e) => return Some(Err(e)),
            };
            let patch = args
                .get("value")
                .or_else(|| args.get("patch"))
                .cloned()
                .unwrap_or(json!({}));
            match GLOBAL_SHARED_STATE.patch(path.clone(), patch) {
                Ok(out) => {
                    if let Some(v) = out.get("value") {
                        let revision = out.get("revision").and_then(|r| r.as_u64()).unwrap_or(0);
                        emit_state_multi(&path, v, revision, &source);
                    }
                    Some(Ok(out))
                }
                Err(e) => Some(Err(e)),
            }
        }
        "state.keys" => Some(Ok(json!({
            "ok": true,
            "keys": GLOBAL_SHARED_STATE.keys(),
        }))),
        "state.delete" => {
            let path = match path() {
                Ok(p) => p,
                Err(e) => return Some(Err(e)),
            };
            let (deleted, revision) = GLOBAL_SHARED_STATE.delete(&path);
            if deleted {
                emit_state_multi(&path, &Json::Null, revision, &source);
            }
            Some(Ok(json!({ "ok": true, "deleted": deleted, "revision": revision })))
        }
        "state.surfaces" => Some(Ok(json!({
            "ok": true,
            "surfaces": broker::GLOBAL_SURFACES.list(),
        }))),
        _ => None,
    }
}

fn platform_notification(cmd: &str, args: &Json) -> Result<Json, String> {
    #[cfg(all(feature = "platform-apple", target_os = "macos"))]
    {
        return apple_notification(cmd, args);
    }
    #[cfg(all(
        feature = "platform-ms",
        not(all(feature = "platform-apple", target_os = "macos"))
    ))]
    {
        return ms_notification(cmd, args);
    }
    #[cfg(all(
        feature = "platform-lin",
        not(feature = "platform-ms"),
        not(all(feature = "platform-apple", target_os = "macos"))
    ))]
    {
        return lin_notification(cmd, args);
    }
    #[cfg(all(
        feature = "platform-android",
        not(feature = "platform-ms"),
        not(feature = "platform-lin"),
        not(all(feature = "platform-apple", target_os = "macos"))
    ))]
    {
        return android_notification(cmd, args);
    }
    #[cfg(not(any(
        all(feature = "platform-apple", target_os = "macos"),
        feature = "platform-ms",
        feature = "platform-lin",
        feature = "platform-android"
    )))]
    {
        let _ = (cmd, args);
        return Ok(broker::unsupported("notification"));
    }
    #[allow(unreachable_code)]
    {
        let _ = (cmd, args);
        Ok(broker::unsupported("notification"))
    }
}

#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn apple_notification(cmd: &str, args: &Json) -> Result<Json, String> {
    match cmd {
        "notification.permissionState" => Ok(json!({
            "state": tish_macos::notification_permission_state(),
        })),
        "notification.requestPermission" => Ok(json!({
            "state": tish_macos::notification_request_permission(),
        })),
        "notification.show" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish Desktop");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            tish_macos::notification_show(title, body)?;
            Ok(json!({ "ok": true, "title": title, "body": body }))
        }
        _ => Ok(broker::unsupported("notification")),
    }
}

#[cfg(feature = "platform-ms")]
fn ms_notification(cmd: &str, args: &Json) -> Result<Json, String> {
    match cmd {
        "notification.permissionState" => Ok(tish_ms::notification_permission_state()),
        "notification.requestPermission" => Ok(tish_ms::notification_request_permission()),
        "notification.show" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            tish_ms::notification_show(title, body)
        }
        _ => Ok(broker::unsupported_on("notification", "ms")),
    }
}

#[cfg(feature = "platform-lin")]
fn lin_notification(cmd: &str, args: &Json) -> Result<Json, String> {
    match cmd {
        "notification.permissionState" => Ok(tish_lin::notification_permission_state()),
        "notification.requestPermission" => Ok(tish_lin::notification_request_permission()),
        "notification.show" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            tish_lin::notification_show(title, body)
        }
        _ => Ok(broker::unsupported_on("notification", "lin")),
    }
}

#[cfg(feature = "platform-android")]
fn android_notification(cmd: &str, args: &Json) -> Result<Json, String> {
    match cmd {
        "notification.permissionState" => Ok(tish_android::notification_permission_state()),
        "notification.requestPermission" => Ok(tish_android::notification_request_permission()),
        "notification.show" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish");
            let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
            tish_android::notification_show(title, body)
        }
        _ => Ok(broker::unsupported_on("notification", "android")),
    }
}

fn emit_state_multi(path: &str, value: &Json, revision: u64, source: &str) {
    if let Some(app) = APP_HANDLE.lock().clone() {
        broker::emit_state_changed(&app, path, value, revision, source);
    }
    #[cfg(feature = "platform-apple")]
    {
        let mut o = tishlang_core::ObjectMap::default();
        o.insert(Arc::from("path"), Value::String(path.into()));
        o.insert(Arc::from("value"), json_to_value(value));
        o.insert(Arc::from("revision"), Value::Number(revision as f64));
        o.insert(Arc::from("source"), Value::String(source.into()));
        tish_macos::broadcast_event("state:changed", &Value::object(o));
    }
}
