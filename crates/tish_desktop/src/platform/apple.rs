//! Thin adapter over tish-apple attach APIs (`macos.attach` / outerHost).

use serde_json::{json, Value};
use tishlang_core::Value as TishValue;

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

    #[cfg(all(feature = "platform-apple", target_os = "macos"))]
    fn attach_native(&self, root: &TishValue, options: &Value) -> Result<Value, String> {
        // Documented contract: callers should prefer Tish `macos.attach` from shell code.
        // This Rust path records intent for hybrid createSurface(kind: native).
        let _ = (root, options);
        Ok(json!({
            "ok": true,
            "host": "apple",
            "mode": "attach",
            "note": "Invoke macos.attach from shell Tish; outerHost skips menu/timer",
        }))
    }

    #[cfg(not(all(feature = "platform-apple", target_os = "macos")))]
    fn attach_native(&self, _root: &TishValue, _options: &Value) -> Result<Value, String> {
        Err("platform-apple not enabled or not macOS".into())
    }
}
