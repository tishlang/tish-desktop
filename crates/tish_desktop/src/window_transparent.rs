//! Runtime window-transparency toggle — macOS.
//!
//! Window transparency is normally a CREATION-time choice (`WindowSpec.transparent` → wry sets the
//! NSWindow non-opaque and turns off the WKWebView's background draw). But a transparent window on
//! macOS is expensive: the compositor recomposites the full window every display frame even when
//! the page is static (tauri#15471), and on macOS 26 that churn feeds a WebKit/JSC compositor bug
//! that leaks 128MB IOAccelerator GPU slabs (oven-sh/bun#28234) — multi-GB per hour in a busy app.
//!
//! So hosts want the cheap default (opaque) and a way to opt IN to transparency at runtime — e.g.
//! only while a theme that uses see-through chrome is active. `window.setTransparent` flips, per
//! managed window: `NSWindow.isOpaque`, the window background color, and the WKWebView's
//! `drawsBackground` (via KVC — the property wry itself sets at creation), then invalidates the
//! window shadow so the shape updates.
//!
//! Pure-AppKit enumeration (NSApplication.windows), same rule as traffic_lights.rs: never call
//! back into Tauri's window map from here.

use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSApplication, NSColor, NSView, NSWindow, NSWindowStyleMask};
use objc2_foundation::{ns_string, MainThreadMarker, NSNumber};

/// Flip transparency on every managed (titled) window. Safe to call from any thread.
pub fn set_transparent(app: &tauri::AppHandle, transparent: bool) {
    let handle = app.clone();
    let _ = handle.run_on_main_thread(move || apply_all_ns(transparent));
}

fn apply_all_ns(transparent: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        // Only the real workbench windows — skips NSStatusBarWindow, panels, etc.
        if window.styleMask().contains(NSWindowStyleMask::Titled) {
            apply_ns(&window, transparent);
        }
    }
}

fn apply_ns(window: &NSWindow, transparent: bool) {
    window.setOpaque(!transparent);
    let color = if transparent {
        unsafe { NSColor::clearColor() }
    } else {
        unsafe { NSColor::windowBackgroundColor() }
    };
    window.setBackgroundColor(Some(&color));
    if let Some(content) = window.contentView() {
        set_webview_draws_background(&content, !transparent);
    }
    // A transparent window's shadow is derived from its content shape — recompute on flip.
    window.invalidateShadow();
}

/// Find WKWebView subviews and set `drawsBackground` via KVC — the same property wry configures at
/// creation for `transparent: true`. KVC because objc2 has no WebKit binding here and the setter is
/// what WKWebView exposes to AppKit hosts.
fn set_webview_draws_background(view: &NSView, draws: bool) {
    let is_webview = view.class().name().to_bytes().windows(9).any(|w| w == b"WKWebView");
    if is_webview {
        let value = NSNumber::numberWithBool(draws);
        let _: () = unsafe {
            msg_send![view, setValue: &*value as &AnyObject, forKey: ns_string!("drawsBackground")]
        };
        return;
    }
    for sub in view.subviews().iter() {
        set_webview_draws_background(&sub, draws);
    }
}
