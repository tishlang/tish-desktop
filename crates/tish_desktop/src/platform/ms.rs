//! Future Windows native host stub (`tish-ms`).

use serde_json::Value;
use tishlang_core::Value as TishValue;

use crate::broker;
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
        false
    }

    fn attach_native(&self, _root: &TishValue, _options: &Value) -> Result<Value, String> {
        Ok(broker::unsupported("platform.ms.attach"))
    }
}
