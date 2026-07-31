//! Capability providers — broker commands with a stable unsupported shape.
//!
//! Plan sketch used `CapSupport` + method name `support`. This crate exposes
//! [`CapSupport`] and [`CapabilityProvider::supported`] (same meaning).

use serde_json::Value;
use tauri::AppHandle;

use crate::broker;
use crate::state::AppState;

mod dialog;
mod notification;
mod store;
mod stubs;
mod webview;

pub use dialog::DialogProvider;
pub use notification::NotificationProvider;
pub use store::StoreProvider;
#[allow(unused_imports)]
pub use stubs::{LinStubProvider, MsStubProvider, UnsupportedProvider};
pub use webview::WebviewProvider;

/// Capability availability (`Full` | `Partial` | `Unsupported`).
/// Plan alias: CapSupport / `support` on the provider sketch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapSupport {
    Full,
    Partial,
    Unsupported,
}

pub trait CapabilityProvider: Send + Sync {
    fn id(&self) -> &str;
    /// Plan sketch name: `support`. Returns [`CapSupport`].
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
    let providers: [&dyn CapabilityProvider; 4] = [
        &NotificationProvider,
        &DialogProvider,
        &StoreProvider,
        &WebviewProvider,
    ];
    for p in providers {
        if let Some(r) = p.try_invoke(app, state, cmd, args) {
            return Some(r);
        }
    }
    None
}

/// Stable JSON for unsupported capabilities (also available without AppHandle).
#[allow(dead_code)]
pub fn unsupported_result(cmd: &str) -> Value {
    broker::unsupported(cmd)
}
