//! Wrap existing `notification.*` commands as a CapProvider.

use serde_json::Value;
use tauri::AppHandle;

use super::{CapSupport, CapabilityProvider};
use crate::commands::chrome;
use crate::state::AppState;

pub struct NotificationProvider;

impl CapabilityProvider for NotificationProvider {
    fn id(&self) -> &str {
        "notification"
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
        if !cmd.starts_with("notification.") {
            return None;
        }
        chrome::try_dispatch(app, state, cmd, args)
    }
}
