//! Android native host adapter (`tish-android`).

use serde_json::Value;
use tishlang_core::Value as TishValue;

use super::{PlatformHost, PlatformId};

pub struct AndroidPlatformHost;

impl PlatformHost for AndroidPlatformHost {
    fn id(&self) -> PlatformId {
        PlatformId::Android
    }

    fn name(&self) -> &'static str {
        "android"
    }

    fn supports_native_surface(&self) -> bool {
        cfg!(feature = "platform-android")
    }

    fn attach_native(&self, _root: &TishValue, options: &Value) -> Result<Value, String> {
        #[cfg(feature = "platform-android")]
        {
            let title = options
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish");
            return tish_android::attach_native(title);
        }
        #[cfg(not(feature = "platform-android"))]
        {
            let _ = options;
            Err("platform-android not enabled".into())
        }
    }
}
