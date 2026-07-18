//! Future Linux native host stub (`tish-lin`).

use serde_json::Value;
use tishlang_core::Value as TishValue;

use crate::broker;
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
        false
    }

    fn attach_native(&self, _root: &TishValue, _options: &Value) -> Result<Value, String> {
        Ok(broker::unsupported("platform.lin.attach"))
    }
}
