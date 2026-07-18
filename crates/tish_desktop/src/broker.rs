//! BrokerCore — surface-agnostic dispatch helpers and shared microfrontend `state.*`.
//!
//! Persisted KV remains `store.*` (Tauri plugin). This module owns in-memory shared state
//! for coordinating native / webview / web surfaces.
//!
//! Contract (plan):
//! - `state.get|set|patch` use `{ path, value }` (not persisted `store` keys)
//! - `state:changed` payload: `{ path, value, revision, source }`

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

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

    pub fn list(&self) -> Vec<SurfaceInfo> {
        self.surfaces.lock().values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<SurfaceInfo> {
        self.surfaces.lock().get(id).cloned()
    }
}

/// Emit `state:changed` to all webviews (Tauri transport).
pub fn emit_state_changed(app: &AppHandle, path: &str, value: &Value, revision: u64, source: &str) {
    let _ = app.emit(
        "state:changed",
        json!({
            "path": path,
            "value": value,
            "revision": revision,
            "source": source,
        }),
    );
}

/// Capability / broker error shape (plan contract).
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

/// Native surfaces queued with a Tish root fn before `run()` (hybrid attach).
pub static PENDING_NATIVE_ROOTS: once_cell::sync::Lazy<
    Mutex<Vec<(String, tishlang_core::Value)>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));
