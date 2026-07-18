//! `cargo:tish_app` — re-exports `tish_desktop` under the public app-runtime name.
//!
//! Prefer this import in new apps; `cargo:tish_desktop` remains supported.

pub use tish_desktop::{
    broker_invoke, close_window, create_surface, create_window, focus_window, handle, list_windows,
    pending_native_roots, register_rust_extension, run, state_get, state_set, use_extensions,
};
