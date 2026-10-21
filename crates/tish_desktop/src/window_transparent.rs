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
//! Transparency alone is not enough for shaped chrome: a TITLED NSWindow's theme frame still clips
//! the content to the system's rounded corners and draws a rounded-rect shadow, so a see-through
//! theme renders inside a fake rounded window. While transparent the window therefore drops
//! the title bar (tao `set_decorations(false)`, keeping resizable / miniaturizable / closable / full-size content):
//! the page's own alpha is the window's geometry and the shadow follows it. Traffic lights go with
//! the title bar; shaped_controls.rs puts the system's own buttons back (full native behavior). Going opaque restores `Titled` — the overlay title bar settings persist.
//!
//! Pure-AppKit enumeration (NSApplication.windows), same rule as traffic_lights.rs: never call
//! back into Tauri's window map from here.

use objc2::msg_send;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSApplication, NSColor, NSView, NSWindow, NSWindowStyleMask};
use objc2_foundation::NSObjectProtocol;
use tauri::Manager;
use objc2_foundation::{ns_string, MainThreadMarker, NSNumber};

/// Flip transparency on every managed workbench window. Safe to call from any thread.
pub fn set_transparent(app: &tauri::AppHandle, transparent: bool) {
    let handle = app.clone();
    // Title bar on/off goes through tao (set_decorations) so its own bookkeeping stays in sync;
    // poking NSWindow.styleMask directly is undone by tao.
    for (_, w) in handle.webview_windows() {
        let _ = w.set_decorations(!transparent);
        if !transparent {
            let _ = w.set_title_bar_style(tauri::TitleBarStyle::Overlay);
        }
        let _ = w.set_shadow(true);
    }
    let _ = handle.run_on_main_thread(move || apply_all_ns(transparent));
    // tao applies its decoration style mask ASYNC on the main queue, after the pass above — which
    // clobbers the Closable/Miniaturizable bits restored above. Re-apply
    // once it has landed (idempotent: buttons are replaced, not duplicated).
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let _ = handle.run_on_main_thread(move || apply_all_ns(transparent));
    });
}

fn apply_all_ns(transparent: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        // Only the real workbench windows (they host a WKWebView) — skips NSStatusBarWindow,
        // panels, etc. Not keyed on `Titled`: a shaped window has dropped it.
        if window.contentView().is_some_and(|v| has_webview(&v)) {
            apply_ns(&window, transparent);
        }
    }
}

fn apply_ns(window: &NSWindow, transparent: bool) {
    window.setHasShadow(true);
    if transparent {
        // tao's borderless mask is Borderless|Resizable; restore Closable/Miniaturizable so close,
        // minimize (⌘W / ⌘M / Window menu) and the host-drawn caption controls still work.
        let mut mask = window.styleMask();
        mask.insert(NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable);
        window.setStyleMask(mask);
    }
    crate::shaped_controls::sync(window, transparent);
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

/// A real WKWebView (wry's WryWebView subclass included) — NOT wry's WryWebViewParent container,
/// which matches a name search but has no `drawsBackground` key (KVC on it throws).
fn is_wk_webview(view: &NSView) -> bool {
    match objc2::runtime::AnyClass::get(c"WKWebView") {
        Some(cls) => view.isKindOfClass(cls),
        None => false,
    }
}

fn has_webview(view: &NSView) -> bool {
    if is_wk_webview(view) {
        return true;
    }
    view.subviews().iter().any(|sub| has_webview(&sub))
}

/// Find WKWebView subviews and set `drawsBackground` via KVC — the same property wry configures at
/// creation for `transparent: true`. KVC because objc2 has no WebKit binding here and the setter is
/// what WKWebView exposes to AppKit hosts.
fn set_webview_draws_background(view: &NSView, draws: bool) {
    let is_webview = is_wk_webview(view);
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
