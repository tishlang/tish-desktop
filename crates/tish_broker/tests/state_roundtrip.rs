use serde_json::json;
use tishlang_broker::{invoke, GLOBAL_SHARED_STATE};

#[test]
fn state_set_get_roundtrip() {
    let set = invoke(
        "state.set",
        json!({ "path": "test.doc", "value": "hello" }),
        "test",
        None,
    )
    .unwrap();
    assert_eq!(set["ok"], true);
    let get = invoke(
        "state.get",
        json!({ "path": "test.doc" }),
        "test",
        None,
    )
    .unwrap();
    assert_eq!(get["value"], "hello");
    let _ = GLOBAL_SHARED_STATE.delete("test.doc");
}

#[test]
fn tray_unsupported_stable_shape() {
    let r = invoke("tray.setIcon", json!({}), "ios", None).unwrap();
    assert_eq!(r["ok"], false);
    assert_eq!(r["code"], "unsupported");
    assert_eq!(r["platform"], "ios");
}
