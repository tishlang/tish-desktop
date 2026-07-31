//! Future platform stubs — return stable `{ ok: false, code: "unsupported" }`.

use serde_json::Value;
use tauri::AppHandle;

use super::{CapSupport, CapabilityProvider};
use crate::broker;
use crate::state::AppState;

pub struct UnsupportedProvider {
    pub id_str: &'static str,
    pub prefix: &'static str,
}

impl CapabilityProvider for UnsupportedProvider {
    fn id(&self) -> &str {
        self.id_str
    }

    fn supported(&self) -> CapSupport {
        CapSupport::Unsupported
    }

    fn try_invoke(
        &self,
        _app: &AppHandle,
        _state: &AppState,
        cmd: &str,
        _args: &Value,
    ) -> Option<Result<Value, String>> {
        if cmd.starts_with(self.prefix) {
            Some(Ok(broker::unsupported(self.id_str)))
        } else {
            None
        }
    }
}

/// Placeholder for future `tish-ms` capability backends.
pub struct MsStubProvider;

impl CapabilityProvider for MsStubProvider {
    fn id(&self) -> &str {
        "ms"
    }

    fn supported(&self) -> CapSupport {
        CapSupport::Unsupported
    }

    fn try_invoke(
        &self,
        _app: &AppHandle,
        _state: &AppState,
        cmd: &str,
        _args: &Value,
    ) -> Option<Result<Value, String>> {
        if cmd.starts_with("ms.") {
            Some(Ok(broker::unsupported_on("ms", "windows")))
        } else {
            None
        }
    }
}

/// Placeholder for future `tish-lin` capability backends.
pub struct LinStubProvider;

impl CapabilityProvider for LinStubProvider {
    fn id(&self) -> &str {
        "lin"
    }

    fn supported(&self) -> CapSupport {
        CapSupport::Unsupported
    }

    fn try_invoke(
        &self,
        _app: &AppHandle,
        _state: &AppState,
        cmd: &str,
        _args: &Value,
    ) -> Option<Result<Value, String>> {
        if cmd.starts_with("lin.") {
            Some(Ok(broker::unsupported_on("lin", "linux")))
        } else {
            None
        }
    }
}
