//! `webview.*` — embedded pane control (load / postMessage).

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
        "webview.list" => Ok(json!({
            "ok": true,
            "labels": app.webview_windows().keys().cloned().collect::<Vec<_>>(),
        })),
        _ => return None,
    };
    Some(result)
}

fn surface_label(args: &Value) -> Result<String, String> {
    args.get("surfaceId")
        .or_else(|| args.get("label"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "surfaceId required".into())
}

fn webview_load(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let label = surface_label(args)?;
    let url_str = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("url required")?;
    let Some(win) = app.get_webview_window(&label) else {
        return Ok(broker::unsupported("webview"));
    };
    let url = Url::parse(url_str).map_err(|e| format!("invalid url: {e}"))?;
    win.navigate(url).map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "surfaceId": label, "url": url_str }))
}

fn webview_post_message(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let label = surface_label(args)?;
    let channel = args
        .get("channel")
        .and_then(|v| v.as_str())
        .unwrap_or("message");
    let body = args.get("body").cloned().unwrap_or(Value::Null);
    if app.get_webview_window(&label).is_none() {
        return Ok(broker::unsupported("webview"));
    }
    // Tauri webviews: event channel. Apple WK panes use macos.webviewPostMessage (tish-apple bridge).
    app.emit(
        &format!("webview:{channel}"),
        json!({ "surfaceId": label, "channel": channel, "body": body }),
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "surfaceId": label, "channel": channel }))
}
