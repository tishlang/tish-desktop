//! `shell.reveal` / `shell.openPath` / `shell.exec`, `http.fetch`, `os.info`,
//! `os.theme`, `display.list`, `display.primary`.

use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::helpers::{ensure_opener, ensure_permission, window_for};
use crate::state::AppState;

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "shell.reveal" => shell_reveal(app, state, args),
        "shell.openPath" => shell_open_path(app, state, args),
        "shell.exec" => shell_exec(state, args),
        "http.fetch" => http_fetch(state, args),
        "os.info" => os_info(state),
        "os.theme" => os_theme(app, state, args),
        "display.list" => display_list(app, args),
        "display.primary" => display_primary(app, args),
        _ => return None,
    };
    Some(result)
}

fn shell_reveal(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_opener(app, state)?;
    let path = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn shell_open_path(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_opener(app, state)?;
    let path = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
    let with = args.get("with").and_then(|v| v.as_str());
    app.opener()
        .open_path(path, with)
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn shell_exec(state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "shell")?;
    let name = args.get("name").and_then(|v| v.as_str()).ok_or("name required")?;
    let extra_args: Vec<String> = args
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let entry = {
        let cfg = state.config.lock();
        cfg.shell_allow
            .iter()
            .find(|e| e.name == name)
            .cloned()
            .ok_or_else(|| format!("shell command not allow-listed: {name}"))?
    };

    let output = std::process::Command::new(&entry.cmd)
        .args(entry.args_prefix.iter())
        .args(extra_args.iter())
        .output()
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "ok": true,
        "code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
    }))
}

fn http_fetch(state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "http")?;
    let url = args.get("url").and_then(|v| v.as_str()).ok_or("url required")?;
    let host = url::Url::parse(url)
        .map_err(|e| e.to_string())?
        .host_str()
        .ok_or("url has no host")?
        .to_string();
    {
        let cfg = state.config.lock();
        if !cfg.http_allow.is_empty() && !cfg.http_allow.iter().any(|h| h == &host) {
            return Err(format!("http host not allow-listed: {host}"));
        }
    }

    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let client = reqwest::blocking::Client::new();
    let mut req = client.request(
        method.parse().map_err(|_| format!("invalid method: {method}"))?,
        url,
    );
    if let Some(headers) = args.get("headers").and_then(|v| v.as_object()) {
        for (k, v) in headers {
            if let Some(v) = v.as_str() {
                req = req.header(k.as_str(), v);
            }
        }
    }
    if let Some(body) = args.get("body").and_then(|v| v.as_str()) {
        req = req.body(body.to_string());
    }

    let resp = req.send().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let headers_json: serde_json::Map<String, Value> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), json!(v.to_str().unwrap_or(""))))
        .collect();
    let text = resp.text().map_err(|e| e.to_string())?;

    Ok(json!({ "ok": true, "status": status, "headers": headers_json, "body": text }))
}

fn os_info(state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "os")?;
    Ok(json!({
        "ok": true,
        "platform": tauri_plugin_os::platform(),
        "arch": tauri_plugin_os::arch(),
        "family": tauri_plugin_os::family(),
        "version": tauri_plugin_os::version().to_string(),
        "type": tauri_plugin_os::type_().to_string(),
        "hostname": tauri_plugin_os::hostname(),
        "locale": tauri_plugin_os::locale(),
    }))
}

fn os_theme(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "os")?;
    let theme = window_for(app, args)?.theme().map_err(|e| e.to_string())?;
    let s = format!("{theme:?}").to_lowercase();
    Ok(json!({ "ok": true, "theme": s }))
}

fn monitor_json(m: &tauri::window::Monitor) -> Value {
    json!({
        "name": m.name(),
        "width": m.size().width,
        "height": m.size().height,
        "x": m.position().x,
        "y": m.position().y,
        "scaleFactor": m.scale_factor(),
    })
}

fn display_list(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let monitors = window_for(app, args)?
        .available_monitors()
        .map_err(|e| e.to_string())?;
    let list: Vec<Value> = monitors.iter().map(monitor_json).collect();
    Ok(json!({ "ok": true, "displays": list }))
}

fn display_primary(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let monitor = window_for(app, args)?
        .primary_monitor()
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "display": monitor.as_ref().map(monitor_json) }))
}
