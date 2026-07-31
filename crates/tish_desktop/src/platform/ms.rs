//! Windows native host adapter (`tish-ms`).

use serde_json::Value;
use tishlang_core::Value as TishValue;

use super::{PlatformHost, PlatformId};

pub struct MsPlatformHost;

impl PlatformHost for MsPlatformHost {
    fn id(&self) -> PlatformId {
        PlatformId::Ms
    }

    fn name(&self) -> &'static str {
        "ms"
    }

    fn supports_native_surface(&self) -> bool {
        cfg!(feature = "platform-ms")
    }

    fn attach_native(&self, _root: &TishValue, options: &Value) -> Result<Value, String> {
        #[cfg(feature = "platform-ms")]
        {
            let title = options
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish");
            return tish_ms::attach_native(title);
        }
        #[cfg(not(feature = "platform-ms"))]
        {
            let _ = options;
            Err("platform-ms not enabled".into())
        }
    }
}
