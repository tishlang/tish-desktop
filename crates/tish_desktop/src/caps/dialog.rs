//! Wrap `dialog.*` as a CapProvider (plan CapSupport API).

use serde_json::Value;
use tauri::AppHandle;

use super::{CapSupport, CapabilityProvider};
use crate::commands::{chrome, dialog_extra};
use crate::state::AppState;

pub struct DialogProvider;

impl CapabilityProvider for DialogProvider {
    fn id(&self) -> &str {
        "dialog"
    }

    fn supported(&self) -> CapSupport {
        CapSupport::Full
    }

    fn try_invoke(
        &self,
        app: &AppHandle,
        state: &AppState,
        cmd: &str,
        args: &Value,
    ) -> Option<Result<Value, String>> {
        if !cmd.starts_with("dialog.") {
            return None;
        }
        if let Some(r) = dialog_extra::try_dispatch(app, state, cmd, args) {
            return Some(r);
        }
        // dialog.message lives in chrome
        chrome::try_dispatch(app, state, cmd, args)
    }
}
