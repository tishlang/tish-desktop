//! Tauri-free BrokerCore — shared microfrontend `state.*` and stable unsupported errors.
//!
//! Desktop adds Tauri IPC transport on top. iOS / pure-native hosts call [`invoke`] directly.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Shared microfrontend state (not persisted `store.*`).
#[derive(Default)]
pub struct SharedState {
    data: Mutex<HashMap<String, Value>>,
    revision: AtomicU64,
}

impl SharedState {
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    pub fn get(&self, path: &str) -> (Value, u64) {
        let rev = self.revision();
        let value = self
            .data
            .lock()
            .get(path)
            .cloned()
            .unwrap_or(Value::Null);
        (value, rev)
    }

    pub fn set(&self, path: String, value: Value) -> Value {
        self.data.lock().insert(path.clone(), value.clone());
        let rev = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        json!({
            "ok": true,
            "path": path,
            "value": value,
            "revision": rev,
        })
    }

    pub fn patch(&self, path: String, patch: Value) -> Result<Value, String> {
        let mut map = self.data.lock();
        let current = map.get(&path).cloned().unwrap_or(json!({}));
        let merged = merge_json(current, patch)?;
        map.insert(path.clone(), merged.clone());
        drop(map);
        let rev = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(json!({
            "ok": true,
            "path": path,
            "value": merged,
            "revision": rev,
        }))
    }

    pub fn keys(&self) -> Vec<String> {
        self.data.lock().keys().cloned().collect()
    }

    pub fn delete(&self, path: &str) -> (bool, u64) {
        let deleted = self.data.lock().remove(path).is_some();
        let rev = if deleted {
            self.revision.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.revision()
        };
        (deleted, rev)
    }
}

fn merge_json(base: Value, patch: Value) -> Result<Value, String> {
    match (base, patch) {
        (Value::Object(mut a), Value::Object(b)) => {
            for (k, v) in b {
                let next = match a.remove(&k) {
                    Some(prev) => merge_json(prev, v)?,
                    None => v,
                };
                a.insert(k, next);
            }
            Ok(Value::Object(a))
        }
        (_, p) => Ok(p),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceKind {
    Native,
    Webview,
    Web,
}

impl Default for SurfaceKind {
    fn default() -> Self {
        Self::Webview
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceInfo {
    pub id: String,
    pub kind: SurfaceKind,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Default)]
pub struct SurfaceRegistry {
    surfaces: Mutex<HashMap<String, SurfaceInfo>>,
}

impl SurfaceRegistry {
    pub fn register(&self, info: SurfaceInfo) {
        self.surfaces.lock().insert(info.id.clone(), info);
    }

    pub fn unregister(&self, id: &str) -> bool {
        self.surfaces.lock().remove(id).is_some()
    }

    pub fn list(&self) -> Vec<SurfaceInfo> {
        self.surfaces.lock().values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<SurfaceInfo> {
        self.surfaces.lock().get(id).cloned()
    }
}

pub fn unsupported(capability: &str) -> Value {
    unsupported_on(capability, "current")
}

pub fn unsupported_on(capability: &str, platform: &str) -> Value {
    json!({
        "ok": false,
        "code": "unsupported",
        "capability": capability,
        "platform": platform,
        "message": format!("{capability} is not supported on {platform}"),
    })
}

pub static GLOBAL_SHARED_STATE: once_cell::sync::Lazy<Arc<SharedState>> =
    once_cell::sync::Lazy::new(|| Arc::new(SharedState::default()));

pub static GLOBAL_SURFACES: once_cell::sync::Lazy<Arc<SurfaceRegistry>> =
    once_cell::sync::Lazy::new(|| Arc::new(SurfaceRegistry::default()));

/// Optional host hooks for caps that are not pure state (notifications, etc.).
pub trait CapBackend: Send + Sync {
    fn try_invoke(&self, cmd: &str, args: &Value) -> Option<Result<Value, String>>;
}

/// Dispatch `state.*` against the global shared state.
pub fn dispatch_state(cmd: &str, args: &Value) -> Option<Result<Value, String>> {
    let path = || {
        args.get("path")
            .or_else(|| args.get("key"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "path required".into())
    };

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
            let value = args.get("value").cloned().unwrap_or(Value::Null);
            Some(Ok(GLOBAL_SHARED_STATE.set(path, value)))
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
            Some(GLOBAL_SHARED_STATE.patch(path, patch))
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
            Some(Ok(json!({ "ok": true, "deleted": deleted, "revision": revision })))
        }
        "state.surfaces" => Some(Ok(json!({
            "ok": true,
            "surfaces": GLOBAL_SURFACES.list(),
        }))),
        _ => None,
    }
}

/// Core invoke: state.* → CapBackend → desktop-only unsupported → unknown error.
pub fn invoke(
    cmd: &str,
    args: Value,
    platform: &str,
    caps: Option<&dyn CapBackend>,
) -> Result<Value, String> {
    if let Some(r) = dispatch_state(cmd, &args) {
        return r;
    }
    if let Some(backend) = caps {
        if let Some(r) = backend.try_invoke(cmd, &args) {
            return r;
        }
    }
    // Desktop-only / host-only caps — stable shape for App branching.
    if cmd.starts_with("tray.")
        || cmd.starts_with("dialog.")
        || cmd.starts_with("store.")
        || cmd.starts_with("webview.")
        || cmd.starts_with("window.")
        || cmd.starts_with("notification.")
    {
        return Ok(unsupported_on(
            cmd.split('.').next().unwrap_or(cmd),
            platform,
        ));
    }
    Err(format!("unknown command: {cmd}"))
}

/// Build `state:changed` payload (transports emit this).
pub fn state_changed_payload(path: &str, value: &Value, revision: u64, source: &str) -> Value {
    json!({
        "path": path,
        "value": value,
        "revision": revision,
        "source": source,
    })
}
