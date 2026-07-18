//! Wrap persisted `store.*` as a CapProvider (not `state.*`).

use serde_json::Value;
use tauri::AppHandle;

use super::{CapSupport, CapabilityProvider};
use crate::commands::store_auto;
use crate::state::AppState;

pub struct StoreProvider;

impl CapabilityProvider for StoreProvider {
    fn id(&self) -> &str {
        "store"
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
        if !cmd.starts_with("store.") {
            return None;
        }
        store_auto::try_dispatch(app, state, cmd, args)
    }
}
