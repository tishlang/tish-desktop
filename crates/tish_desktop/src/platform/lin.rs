//! Linux native host adapter (`tish-lin`).

use serde_json::Value;
use tishlang_core::Value as TishValue;

use super::{PlatformHost, PlatformId};

pub struct LinPlatformHost;

impl PlatformHost for LinPlatformHost {
    fn id(&self) -> PlatformId {
        PlatformId::Lin
    }

    fn name(&self) -> &'static str {
        "lin"
    }

    fn supports_native_surface(&self) -> bool {
        cfg!(feature = "platform-lin")
    }

    fn attach_native(&self, _root: &TishValue, options: &Value) -> Result<Value, String> {
        #[cfg(feature = "platform-lin")]
        {
            let title = options
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Tish");
            return tish_lin::attach_native(title);
        }
        #[cfg(not(feature = "platform-lin"))]
        {
            let _ = options;
            Err("platform-lin not enabled".into())
        }
    }
}
