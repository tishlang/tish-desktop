use std::sync::Arc;
use tishlang_core::{ObjectMap, Value};

pub fn arg_str(args: &[Value], i: usize) -> Option<String> {
    match args.get(i) {
        Some(Value::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

pub fn arg_num(args: &[Value], i: usize) -> Option<f64> {
    match args.get(i) {
        Some(Value::Number(n)) => Some(*n),
        _ => None,
    }
}

pub fn arg_bool(args: &[Value], i: usize) -> Option<bool> {
    match args.get(i) {
        Some(Value::Bool(b)) => Some(*b),
        _ => None,
    }
}

pub fn arg_object_json(args: &[Value], i: usize) -> Option<serde_json::Value> {
    let v = args.get(i)?;
    value_to_json(v)
}

pub fn ok_json(v: serde_json::Value) -> Value {
    json_to_value(&v)
}

pub fn err_str(msg: impl Into<String>) -> Value {
    let mut m = ObjectMap::default();
    m.insert(Arc::from("ok"), Value::Bool(false));
    m.insert(Arc::from("error"), Value::String(msg.into().into()));
    Value::object(m)
}

pub fn value_to_json(v: &Value) -> Option<serde_json::Value> {
    match v {
        Value::Null => Some(serde_json::Value::Null),
        Value::Bool(b) => Some(serde_json::Value::Bool(*b)),
        Value::Number(n) => serde_json::Number::from_f64(*n).map(serde_json::Value::Number),
        Value::String(s) => Some(serde_json::Value::String(s.to_string())),
        Value::Array(a) => {
            // Clone elems first — under send-values, VmRef is a non-reentrant Mutex.
            // Holding borrow() while recursing into nested arrays/objects self-deadlocks.
            let items: Vec<Value> = a.borrow().clone();
            let out: Option<Vec<_>> = items.iter().map(value_to_json).collect();
            out.map(serde_json::Value::Array)
        }
        Value::Object(o) => {
            // Same: drop the object lock before recursing into property values.
            let entries: Vec<(String, Value)> = o
                .borrow()
                .strings
                .iter()
                .map(|(k, val)| (k.to_string(), val.clone()))
                .collect();
            let mut map = serde_json::Map::new();
            for (k, val) in entries {
                // Skip functions / natives — JSON round-trip can't preserve them
                // (e.g. createSurface({ root: Sidebar })).
                if let Some(j) = value_to_json(&val) {
                    map.insert(k, j);
                }
            }
            Some(serde_json::Value::Object(map))
        }
        // Functions, natives, etc. — not JSON-representable
        _ => None,
    }
}

pub fn json_to_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::String(s.clone().into()),
        serde_json::Value::Array(arr) => {
            Value::array(arr.iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut m = ObjectMap::default();
            for (k, val) in obj {
                m.insert(Arc::from(k.as_str()), json_to_value(val));
            }
            Value::object(m)
        }
    }
}
