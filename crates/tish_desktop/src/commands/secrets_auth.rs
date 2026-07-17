//! `secrets.*` (OS keychain via `keyring`) and OAuth2 PKCE `auth.*`.
//!
//! Redirect modes:
//! - `loopback` (default): ephemeral `http://127.0.0.1:<port>/callback`
//! - `scheme`: `tish-desktop://oauth/callback` via deep-link
//!
//! Events: `auth:changed` `{ loggedIn }`, `auth:error` `{ message }`.

use base64::Engine;
use rand::RngCore;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use super::helpers::ensure_permission;
use crate::state::{AppState, PendingOAuth};

const SERVICE: &str = "com.tishlang.desktop";
const REFRESH_TOKEN_KEY: &str = "auth:refresh_token";
pub const OAUTH_SCHEME_REDIRECT: &str = "tish-desktop://oauth/callback";

pub fn try_dispatch(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    let result = match cmd {
        "secrets.set" => secrets_set(state, args),
        "secrets.get" => secrets_get(state, args),
        "secrets.delete" => secrets_delete(state, args),
        "auth.login" => auth_login(app, state, args),
        "auth.logout" => auth_logout(app, state),
        "auth.status" => auth_status(state),
        "auth.getAccessToken" => auth_get_access_token(app, state, args),
        _ => return None,
    };
    Some(result)
}

fn emit_auth_error(app: &AppHandle, message: impl AsRef<str>) {
    let message = message.as_ref();
    let _ = app.emit("auth:error", json!({ "message": message }));
    let _ = app.emit(
        "auth:changed",
        json!({ "loggedIn": false, "error": message }),
    );
}

// ── secrets.* ──────────────────────────────────────────────────────────────

fn secrets_set(state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "secrets")?;
    let key = args.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
    let value = args.get("value").and_then(|v| v.as_str()).ok_or("value required")?;
    keyring::Entry::new(SERVICE, key)
        .map_err(|e| e.to_string())?
        .set_password(value)
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true }))
}

fn secrets_get(state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "secrets")?;
    let key = args.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
    let entry = keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(v) => Ok(json!({ "ok": true, "value": v })),
        Err(keyring::Error::NoEntry) => Ok(json!({ "ok": true, "value": null })),
        Err(e) => Err(e.to_string()),
    }
}

fn secrets_delete(state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "secrets")?;
    let key = args.get("key").and_then(|v| v.as_str()).ok_or("key required")?;
    let entry = keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(json!({ "ok": true })),
        Err(e) => Err(e.to_string()),
    }
}

// ── auth.* ─────────────────────────────────────────────────────────────────

fn random_url_safe(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn check_auth_host(state: &AppState, url_str: &str) -> Result<(), String> {
    let cfg = state.config.lock();
    if let Some(auth) = &cfg.auth {
        if !auth.token_hosts.is_empty() {
            let host = url::Url::parse(url_str)
                .map_err(|e| e.to_string())?
                .host_str()
                .unwrap_or("")
                .to_string();
            if !auth.token_hosts.iter().any(|h| h == &host) {
                return Err(format!("token host not allowed: {host}"));
            }
        }
    }
    Ok(())
}

fn arg_endpoint(args: &Value, camel: &str, alt: &str) -> Result<String, String> {
    args.get(camel)
        .or_else(|| args.get(alt))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("{camel} (or {alt}) required"))
}

fn arg_scope(args: &Value) -> String {
    if let Some(s) = args.get("scope").and_then(|v| v.as_str()) {
        return s.to_string();
    }
    if let Some(arr) = args.get("scopes").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(" ");
    }
    String::new()
}

fn parse_callback_query(url_str: &str) -> Result<(String, String), String> {
    let url = if url_str.starts_with("http://") || url_str.starts_with("https://") || url_str.contains("://")
    {
        url::Url::parse(url_str).map_err(|e| e.to_string())?
    } else {
        url::Url::parse(&format!("http://127.0.0.1{url_str}")).map_err(|e| e.to_string())?
    };
    let mut code = None;
    let mut state_val = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state_val = Some(v.into_owned()),
            _ => {}
        }
    }
    let code = code.ok_or("no code in callback")?;
    let state_val = state_val.unwrap_or_default();
    Ok((code, state_val))
}

fn exchange_code(app: &AppHandle, pending: &PendingOAuth, code: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&pending.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", pending.redirect_uri.as_str()),
            ("client_id", pending.client_id.as_str()),
            ("code_verifier", pending.verifier.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?;
    let body: Value = resp.json().map_err(|e| e.to_string())?;
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("no access_token in response")?
        .to_string();
    let expires_in = body.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);
    *app.state::<AppState>().auth_cache.lock() = Some((access_token, now_secs() + expires_in));

    if let Some(refresh_token) = body.get("refresh_token").and_then(|v| v.as_str()) {
        keyring::Entry::new(SERVICE, REFRESH_TOKEN_KEY)
            .map_err(|e| e.to_string())?
            .set_password(refresh_token)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Completes a pending OAuth session from a callback URL (deep-link or loopback path).
pub fn complete_oauth_callback(app: &AppHandle, callback_url: &str) -> bool {
    let Ok((code, recv_state)) = parse_callback_query(callback_url) else {
        return false;
    };
    let pending = app.state::<AppState>().pending_oauth.lock().take();
    let Some(pending) = pending else {
        return false;
    };
    if recv_state != pending.csrf_state {
        emit_auth_error(app, "state mismatch");
        return true;
    }
    match exchange_code(app, &pending, &code) {
        Ok(()) => {
            let _ = app.emit("auth:changed", json!({ "loggedIn": true }));
        }
        Err(e) => emit_auth_error(app, e),
    }
    true
}

/// True when `url` looks like the custom-scheme OAuth redirect.
pub fn is_oauth_scheme_callback(url: &str) -> bool {
    url.starts_with("tish-desktop://oauth/callback")
        || url.starts_with("tish-desktop://oauth/callback?")
}

fn auth_login(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "auth")?;
    let authorize_url = arg_endpoint(args, "authorizeUrl", "authorizationEndpoint")?;
    let token_url = arg_endpoint(args, "tokenUrl", "tokenEndpoint")?;
    let client_id = args
        .get("clientId")
        .and_then(|v| v.as_str())
        .ok_or("clientId required")?
        .to_string();
    let scope = arg_scope(args);
    let redirect_mode = args
        .get("redirectMode")
        .and_then(|v| v.as_str())
        .unwrap_or("loopback")
        .to_ascii_lowercase();

    check_auth_host(state, &token_url)?;

    let verifier = random_url_safe(48);
    let challenge = code_challenge_s256(&verifier);
    let csrf_state = random_url_safe(24);

    let (redirect_uri, loopback_server) = if redirect_mode == "scheme" {
        (OAUTH_SCHEME_REDIRECT.to_string(), None)
    } else {
        let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = server
            .server_addr()
            .to_ip()
            .ok_or("failed to bind loopback server")?
            .port();
        (format!("http://127.0.0.1:{port}/callback"), Some(server))
    };

    let mut authorize = url::Url::parse(&authorize_url).map_err(|e| e.to_string())?;
    {
        let mut q = authorize.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", &client_id);
        q.append_pair("redirect_uri", &redirect_uri);
        if !scope.is_empty() {
            q.append_pair("scope", &scope);
        }
        q.append_pair("state", &csrf_state);
        q.append_pair("code_challenge", &challenge);
        q.append_pair("code_challenge_method", "S256");
        if let Some(extra) = args.get("extraAuthParams").and_then(|v| v.as_object()) {
            for (k, v) in extra {
                if let Some(v) = v.as_str() {
                    q.append_pair(k, v);
                }
            }
        }
    }

    let pending = PendingOAuth {
        verifier,
        csrf_state,
        token_url,
        client_id,
        redirect_uri: redirect_uri.clone(),
    };
    *state.pending_oauth.lock() = Some(pending.clone());

    let _ = app.opener().open_url(authorize.to_string(), None::<&str>);

    if let Some(server) = loopback_server {
        let app2 = app.clone();
        std::thread::spawn(move || {
            let outcome: Result<(), String> = (|| {
                let request = server.recv().map_err(|e| e.to_string())?;
                let url = request.url().to_string();
                let _ = request.respond(tiny_http::Response::from_string(
                    "<html><body>Login complete — you can close this window.</body></html>",
                ));
                let (code, recv_state) = parse_callback_query(&url)?;
                let pending = app2
                    .state::<AppState>()
                    .pending_oauth
                    .lock()
                    .take()
                    .ok_or("no pending oauth session")?;
                if recv_state != pending.csrf_state {
                    return Err("state mismatch".into());
                }
                exchange_code(&app2, &pending, &code)
            })();

            match outcome {
                Ok(()) => {
                    let _ = app2.emit("auth:changed", json!({ "loggedIn": true }));
                }
                Err(e) => emit_auth_error(&app2, e),
            }
        });
    }

    Ok(json!({
        "ok": true,
        "pending": true,
        "redirectUri": redirect_uri,
        "redirectMode": redirect_mode,
    }))
}

fn auth_logout(app: &AppHandle, state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "auth")?;
    if let Ok(entry) = keyring::Entry::new(SERVICE, REFRESH_TOKEN_KEY) {
        let _ = entry.delete_credential();
    }
    *state.auth_cache.lock() = None;
    *state.pending_oauth.lock() = None;
    let _ = app.emit("auth:changed", json!({ "loggedIn": false }));
    Ok(json!({ "ok": true }))
}

fn auth_status(state: &AppState) -> Result<Value, String> {
    ensure_permission(state, "auth")?;
    let logged_in = keyring::Entry::new(SERVICE, REFRESH_TOKEN_KEY)
        .ok()
        .and_then(|e| e.get_password().ok())
        .is_some();
    Ok(json!({ "ok": true, "loggedIn": logged_in }))
}

fn auth_get_access_token(app: &AppHandle, state: &AppState, args: &Value) -> Result<Value, String> {
    ensure_permission(state, "auth")?;

    if let Some((token, expires_at)) = state.auth_cache.lock().clone() {
        if now_secs() + 30 < expires_at {
            return Ok(json!({ "ok": true, "accessToken": token }));
        }
    }

    let token_url = arg_endpoint(args, "tokenUrl", "tokenEndpoint")?;
    let client_id = args
        .get("clientId")
        .and_then(|v| v.as_str())
        .ok_or("clientId required to refresh the access token")?;
    check_auth_host(state, &token_url)?;

    let refresh_token = keyring::Entry::new(SERVICE, REFRESH_TOKEN_KEY)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|_| "not logged in".to_string())?;

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&token_url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", client_id),
        ])
        .send()
        .map_err(|e| e.to_string())?;
    let body: Value = resp.json().map_err(|e| e.to_string())?;
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("no access_token in refresh response")?
        .to_string();
    let expires_in = body.get("expires_in").and_then(|v| v.as_u64()).unwrap_or(3600);

    if let Some(new_refresh) = body.get("refresh_token").and_then(|v| v.as_str()) {
        keyring::Entry::new(SERVICE, REFRESH_TOKEN_KEY)
            .map_err(|e| e.to_string())?
            .set_password(new_refresh)
            .map_err(|e| e.to_string())?;
    }

    let expires_at = now_secs() + expires_in;
    *state.auth_cache.lock() = Some((access_token.clone(), expires_at));
    let _ = app.emit("auth:changed", json!({ "loggedIn": true }));
    Ok(json!({ "ok": true, "accessToken": access_token }))
}
