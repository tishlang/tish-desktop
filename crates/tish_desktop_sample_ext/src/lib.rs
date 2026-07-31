//! Sample Rust extension: `cargo:tishlang_desktop_sample_ext`
use std::sync::Arc;
use tishlang_core::{ObjectMap, Value};

/// `samplePing()` → { ok: true, ext: "sample" }
pub fn sample_ping(_args: &[Value]) -> Value {
    let mut m = ObjectMap::default();
    m.insert(Arc::from("ok"), Value::Bool(true));
    m.insert(Arc::from("ext"), Value::String("sample".into()));
    Value::object(m)
}
