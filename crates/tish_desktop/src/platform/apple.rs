//! Thin adapter over tish-apple attach APIs (`macos.attach` / `attach_app`).

use serde_json::{json, Value};
use tishlang_core::{ObjectMap, Value as TishValue};
use std::sync::Arc;

use super::{PlatformHost, PlatformId};

pub struct ApplePlatformHost;

impl PlatformHost for ApplePlatformHost {
    fn id(&self) -> PlatformId {
        PlatformId::Apple
    }

    fn name(&self) -> &'static str {
        "apple"
    }

    fn supports_native_surface(&self) -> bool {
        cfg!(all(feature = "platform-apple", target_os = "macos"))
    }

    #[cfg(feature = "platform-apple")]
    fn attach_native(&self, root: &TishValue, options: &Value) -> Result<Value, String> {
        if matches!(root, TishValue::Null) {
            return Ok(json!({
                "ok": true,
                "queued": true,
                "host": "apple",
                "mode": "attach",
                "note": "root required — drained at run() via attach_app",
            }));
        }
        let opts = json_options_to_tish(options);
        let handle = tish_macos::attach_app(root.clone(), Some(opts));
        Ok(json!({
            "ok": true,
            "host": "apple",
            "mode": "attach",
            "attached": !matches!(handle, TishValue::Null),
        }))
    }

    #[cfg(not(feature = "platform-apple"))]
    fn attach_native(&self, _root: &TishValue, _options: &Value) -> Result<Value, String> {
        Err("platform-apple not enabled".into())
    }
}

#[cfg(feature = "platform-apple")]
fn json_options_to_tish(options: &Value) -> TishValue {
    let mut map = ObjectMap::default();
    if let Some(obj) = options.as_object() {
        for (k, v) in obj {
            map.insert(Arc::from(k.as_str()), crate::value_util::json_to_value(v));
        }
    }
    // Defaults for hybrid outer host.
    if !map.contains_key("outerHost") {
        map.insert(Arc::from("outerHost"), TishValue::Bool(true));
    }
    if !map.contains_key("autoRunEventLoop") {
        map.insert(Arc::from("autoRunEventLoop"), TishValue::Bool(false));
    }
    TishValue::object(map)
}
