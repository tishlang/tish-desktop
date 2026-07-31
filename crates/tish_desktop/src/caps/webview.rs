//! Wrap `webview.*` as a CapProvider (Tauri panes + apple WK).

use serde_json::Value;
use tauri::AppHandle;

use super::{CapSupport, CapabilityProvider};
use crate::commands::webview_cap;
use crate::state::AppState;

pub struct WebviewProvider;

impl CapabilityProvider for WebviewProvider {
    fn id(&self) -> &str {
        "webview"
    }

    fn supported(&self) -> CapSupport {
        CapSupport::Partial
    }

    fn try_invoke(
        &self,
        app: &AppHandle,
        state: &AppState,
        cmd: &str,
        args: &Value,
    ) -> Option<Result<Value, String>> {
        if !cmd.starts_with("webview.") {
            return None;
        }
        webview_cap::try_dispatch(app, state, cmd, args)
    }
}
