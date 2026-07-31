//! `webview.*` — embedded pane control (load / postMessage).
//!
//! Desktop Tauri windows first; host WK panes (`<webview bridge>`) via tish-macos when present.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

use crate::broker;
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    _state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "webview.load" => webview_load(app, args),
        "webview.postMessage" => webview_post_message(app, args),
        "webview.eval" => webview_eval(app, args),
        "webview.list" => webview_list(app),
        _ => return None,
    };
    Some(result)
}

fn surface_label(args: &Value) -> Result<String, String> {
    args.get("surfaceId")
        .or_else(|| args.get("label"))
        .or_else(|| args.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "surfaceId required".into())
}

fn webview_list(app: &AppHandle) -> Result<Value, String> {
    let mut labels: Vec<String> = app.webview_windows().keys().cloned().collect();
    #[cfg(all(feature = "platform-apple", target_os = "macos"))]
    {
        if let Some(Ok(v)) = tishlang_macos::webview_broker_try_invoke("webview.list", &json!({})) {
            if let Some(arr) = v.get("labels").and_then(|x| x.as_array()) {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if !labels.iter().any(|l| l == s) {
                            labels.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(json!({ "ok": true, "labels": labels, "surfaceIds": labels }))
}

fn webview_load(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let label = surface_label(args)?;
    if let Some(win) = app.get_webview_window(&label) {
        let url_str = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("url required")?;
        let url = Url::parse(url_str).map_err(|e| format!("invalid url: {e}"))?;
        win.navigate(url).map_err(|e| e.to_string())?;
        return Ok(json!({ "ok": true, "surfaceId": label, "url": url_str }));
    }
    if let Some(r) = wk_try(args, "webview.load") {
        return r;
    }
    Ok(broker::unsupported("webview"))
}

fn webview_post_message(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let label = surface_label(args)?;
    let channel = args
        .get("channel")
        .or_else(|| args.get("event"))
        .and_then(|v| v.as_str())
        .unwrap_or("message");
    let body = args
        .get("body")
        .or_else(|| args.get("payload"))
        .cloned()
        .unwrap_or(Value::Null);
    if app.get_webview_window(&label).is_some() {
        app.emit(
            &format!("webview:{channel}"),
            json!({ "surfaceId": label, "channel": channel, "body": body }),
        )
        .map_err(|e| e.to_string())?;
        return Ok(json!({ "ok": true, "surfaceId": label, "channel": channel }));
    }
    if let Some(r) = wk_try(args, "webview.postMessage") {
        return r;
    }
    Ok(broker::unsupported("webview"))
}

fn webview_eval(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let label = surface_label(args)?;
    let js = args
        .get("js")
        .or_else(|| args.get("script"))
        .and_then(|v| v.as_str())
        .ok_or("js required")?;
    if let Some(win) = app.get_webview_window(&label) {
        win.eval(js).map_err(|e| e.to_string())?;
        return Ok(json!({ "ok": true, "surfaceId": label }));
    }
    if let Some(r) = wk_try(args, "webview.eval") {
        return r;
    }
    Ok(broker::unsupported("webview"))
}

#[cfg(all(feature = "platform-apple", target_os = "macos"))]
fn wk_try(args: &Value, cmd: &str) -> Option<Result<Value, String>> {
    match tishlang_macos::webview_broker_try_invoke(cmd, args) {
        Some(Ok(v)) => Some(Ok(v)),
        Some(Err(e)) if e.contains("no bridged webview") => None,
        Some(Err(e)) => Some(Err(e)),
        None => None,
    }
}

#[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
fn wk_try(_args: &Value, _cmd: &str) -> Option<Result<Value, String>> {
    None
}
