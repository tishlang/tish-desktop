//! BrokerCore — re-exports Tauri-free core from `tishlang_broker` plus desktop transports.

use parking_lot::Mutex;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

pub use tishlang_broker::{
    unsupported, unsupported_on, SharedState, SurfaceInfo, SurfaceKind, SurfaceRegistry,
    GLOBAL_SHARED_STATE, GLOBAL_SURFACES,
};

/// Emit `state:changed` to all webviews (Tauri transport).
pub fn emit_state_changed(app: &AppHandle, path: &str, value: &Value, revision: u64, source: &str) {
    let _ = app.emit(
        "state:changed",
        tishlang_broker::state_changed_payload(path, value, revision, source),
    );
}

/// Native surfaces queued with a Tish root fn before `run()` (hybrid attach).
pub static PENDING_NATIVE_ROOTS: once_cell::sync::Lazy<
    Mutex<Vec<(String, tishlang_core::Value)>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(Vec::new()));

/// Convenience for callers that still build the error inline.
#[allow(dead_code)]
pub fn unsupported_json(capability: &str) -> Value {
    unsupported(capability)
}

#[allow(dead_code)]
pub fn ok_shape() -> Value {
    json!({ "ok": true })
}
