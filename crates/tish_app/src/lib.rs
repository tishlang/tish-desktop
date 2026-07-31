//! `cargo:tishlang_app` — re-exports `tishlang_desktop` under the public app-runtime name.
//!
//! Prefer this import in new apps; `cargo:tishlang_desktop` remains supported.

pub use tishlang_desktop::{
    broker_invoke, close_window, create_surface, create_window, emit, emit_event, focus_window,
    handle, list_windows, pending_native_roots, register_rust_extension, run, state_get, state_set,
    use_extensions,
};
