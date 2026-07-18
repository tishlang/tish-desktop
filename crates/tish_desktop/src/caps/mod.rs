//! Capability providers — broker commands with a stable unsupported shape.

use serde_json::Value;
use tauri::AppHandle;

use crate::broker;
use crate::state::AppState;

mod notification;
mod stubs;

pub use notification::NotificationProvider;
#[allow(unused_imports)]
pub use stubs::{LinStubProvider, MsStubProvider, UnsupportedProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapSupport {
    Full,
    Partial,
    Unsupported,
}

pub trait CapabilityProvider: Send + Sync {
    fn id(&self) -> &str;
    fn supported(&self) -> CapSupport;
    fn try_invoke(
        &self,
        app: &AppHandle,
        state: &AppState,
        cmd: &str,
        args: &Value,
    ) -> Option<Result<Value, String>>;
}

/// Try registered providers; returns `None` if no provider claims the command.
pub fn try_caps(
    app: &AppHandle,
    state: &AppState,
    cmd: &str,
    args: &Value,
) -> Option<Result<Value, String>> {
    // Built-in providers (order matters for overlapping prefixes).
    let notification = NotificationProvider;
    if let Some(r) = notification.try_invoke(app, state, cmd, args) {
        return Some(r);
    }
    None
}

/// Stable JSON for unsupported capabilities (also available without AppHandle).
#[allow(dead_code)]
pub fn unsupported_result(cmd: &str) -> Value {
    broker::unsupported(cmd)
}
