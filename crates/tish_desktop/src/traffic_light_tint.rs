//! macOS traffic-light color TINT — theme-driven, isolated from `traffic_lights.rs`.
//!
//! Paints a themed colored disc over each native window button (close/minimize/zoom) while leaving
//! the native buttons — and every native window behavior (click, hover, the green button's
//! window-management menu) — fully intact underneath. The disc is a non-interactive overlay
//! (`hitTest:` → nil), so it never captures a mouse event.
//!
//! WHY THE OVERLAY LIVES IN THE BUTTON'S SUPERVIEW (not the button): the native orb is NOT drawn by
//! the button's own content — a subview of the button paints BEHIND it (confirmed empirically: the
//! disc's `drawRect` ran yet the orb stayed on top). The orb is drawn by a sibling/ancestor view, so
//! to cover it the disc must be a sibling of the button in its SUPERVIEW, ordered above it. There it
//! composites over the button entirely. The cost is that the overlay no longer auto-follows the
//! button's frame, so this module keeps each overlay aligned + on top via its own relayout observer.
//!
//! DRIVEN BY THE HOST: the webview pushes per-button close/minimize/zoom colors (+ optional
//! diameter / opacity) via the generic `window.trafficLightTint` command. Sending none leaves the
//! native buttons untouched. Where the host gets those colors (e.g. theme tokens) is its concern.
//! No-op off macOS.
//!
//! SEPARATION CONTRACT: this file owns its own app handle, config, overlay class, per-button overlay
//! map, and relayout observer. It shares NOTHING mutable with `traffic_lights.rs` (which owns button
//! POSITION) and nothing references it back — the dependency is one-way, so it can be edited or
//! removed on its own.

use crate::traffic_lights::guard_objc_callback;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBezierPath, NSButton, NSColor, NSView, NSViewFrameDidChangeNotification,
    NSWindow, NSWindowButton, NSWindowDidBecomeKeyNotification, NSWindowDidResignKeyNotification,
    NSWindowDidUpdateNotification, NSWindowOrderingMode, NSWindowWillCloseNotification,
};
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect,
    NSSize,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

/// One button's tint as straight sRGB in 0..=1 (alpha already folded with opacity), or `None`.
type Rgba = (f64, f64, f64, f64);

/// Active tint payload. `diameter <= 0` → fit the button's shorter side.
#[derive(Clone, Default)]
struct TintConfig {
    close: Option<Rgba>,
    minimize: Option<Rgba>,
    zoom: Option<Rgba>,
    diameter: f64,
}

/// Active tint, or `None` → feature off → every overlay removed → pure native look.
static CONFIG: Mutex<Option<TintConfig>> = Mutex::new(None);
/// App handle the scheduled re-assert + observer use to reach the windows. Ours alone.
static OBSERVER_APP: OnceLock<tauri::AppHandle> = OnceLock::new();

// ---- The non-interactive overlay view -------------------------------------------------------------

#[derive(Default)]
struct TintIvars {
    color: RefCell<Option<Retained<NSColor>>>,
    diameter: Cell<f64>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "TishTrafficTintView"]
    #[thread_kind = MainThreadOnly]
    #[ivars = TintIvars]
    struct TintView;

    impl TintView {
        // Fill a centered disc in the overlay's bounds with the theme color.
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
            // Follow native macOS: only the FOCUSED (key) window shows colored controls. When the
            // window is not key its native buttons grey out — so we paint nothing and let that grey
            // show through (the overlay is also hidden via setHidden on the same focus change; this
            // is the belt-and-suspenders guard against any stray repaint while still visible).
            let is_key = self.window().map(|w| w.isKeyWindow()).unwrap_or(false);
            if !is_key {
                return;
            }
            let ivars = self.ivars();
            let borrowed = ivars.color.borrow();
            let Some(color) = borrowed.as_ref() else {
                return;
            };
            let bounds = self.bounds();
            let raw = ivars.diameter.get();
            let d = if raw > 0.0 {
                raw
            } else {
                bounds.size.width.min(bounds.size.height)
            };
            let rect = NSRect::new(
                NSPoint::new(
                    bounds.origin.x + (bounds.size.width - d) / 2.0,
                    bounds.origin.y + (bounds.size.height - d) / 2.0,
                ),
                NSSize::new(d, d),
            );
            color.setFill();
            NSBezierPath::bezierPathWithOvalInRect(rect).fill();
        }

        // Transparent to hit-testing: clicks / hover fall through to the native button beneath.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }
    }
);

impl TintView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(TintIvars::default());
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

// ---- Per-button overlay bookkeeping ---------------------------------------------------------------

thread_local! {
    /// button pointer → its overlay (the overlay lives in the button's superview, not the button).
    static OVERLAYS: RefCell<HashMap<usize, Retained<TintView>>> = RefCell::new(HashMap::new());
    /// Reentrancy guard: our setFrame / addSubview posts notifications the observer would re-enter on.
    static APPLYING: Cell<bool> = const { Cell::new(false) };
}

fn standard_buttons(window: &NSWindow) -> Option<[Retained<NSButton>; 3]> {
    Some([
        window.standardWindowButton(NSWindowButton::CloseButton)?,
        window.standardWindowButton(NSWindowButton::MiniaturizeButton)?,
        window.standardWindowButton(NSWindowButton::ZoomButton)?,
    ])
}

/// Place/refresh (or remove) the overlay for one button. `frames` are in the button's superview space
/// (a button's own `frame` is already expressed there), so the overlay simply mirrors `button.frame`.
fn apply_button(
    mtm: MainThreadMarker,
    button: &NSButton,
    color: Option<Rgba>,
    diameter: f64,
    is_key: bool,
) {
    let key = button as *const NSButton as usize;
    match color {
        Some((r, g, b, a)) => {
            let frame = button.frame();
            if frame.size.width < 1.0 || frame.size.height < 1.0 {
                return; // button not laid out yet; a later apply catches it
            }
            // SAFETY: superview() is a plain AppKit accessor; nil handled by `?`. Main thread only.
            let Some(superview) = (unsafe { button.superview() }) else {
                return;
            };
            let overlay = OVERLAYS.with(|m| m.borrow().get(&key).cloned());
            let overlay = overlay.unwrap_or_else(|| {
                let v = TintView::new(mtm, frame);
                OVERLAYS.with(|m| m.borrow_mut().insert(key, v.clone()));
                v
            });
            overlay.setFrame(frame);
            // Order the overlay ABOVE every sibling of the button (including the button itself), so it
            // paints over the native orb. Re-asserted each apply because a title-bar relayout can
            // reinsert the native views on top.
            superview.addSubview_positioned_relativeTo(&overlay, NSWindowOrderingMode::Above, None);
            let nscolor = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a);
            *overlay.ivars().color.borrow_mut() = Some(nscolor);
            overlay.ivars().diameter.set(diameter);
            // Only show the tint on the focused window — when the window is not key, hide the overlay
            // so the native (greyed-out) button shows through, matching standard macOS behavior.
            overlay.setHidden(!is_key);
            overlay.setNeedsDisplay(true);
        }
        None => {
            if let Some(overlay) = OVERLAYS.with(|m| m.borrow_mut().remove(&key)) {
                overlay.removeFromSuperview();
            }
        }
    }
}

// NOTE: there is deliberately no `WebviewWindow` -> `NSWindow` helper here any more. See
// `traffic_lights.rs` — `ns_window()` re-enters wry's window map and aborts the process. Windows are
// reached through the notification's own object, or through `NSApplication::windows()`.

/// A window this chrome manages: one of THIS process's windows carrying the three standard
/// title-bar buttons. Panels, sheets and borderless windows have no `standardWindowButton`.
///
/// Deliberately does NOT consult Tauri. Every caller runs inside an AppKit callback, and
/// `WebviewWindow::ns_window()` / `webview_windows()` dispatch a `WindowMessage` that re-enters
/// wry's `handle_user_message` — taking `windows.0.borrow()` while wry's own dispatch holds
/// `borrow_mut()` aborts the process with `RefCell already mutably borrowed`.
///
/// Gating on prior observation instead would be circular: a window is only ever recorded from
/// `install_observers`, which is only reached through this check.
fn is_managed_window(ns_window: &NSWindow) -> bool {
    standard_buttons(ns_window).is_some()
}

fn apply_ns(ns_window: &NSWindow) {
    if APPLYING.with(|f| f.get()) {
        return;
    }
    APPLYING.with(|f| f.set(true));
    if let Some(mtm) = MainThreadMarker::new() {
        let config = CONFIG.lock().unwrap().clone();
        install_observers(ns_window);
        let is_key = ns_window.isKeyWindow();
        if let Some(buttons) = standard_buttons(ns_window) {
            match &config {
                Some(cfg) => {
                    apply_button(mtm, &buttons[0], cfg.close, cfg.diameter, is_key);
                    apply_button(mtm, &buttons[1], cfg.minimize, cfg.diameter, is_key);
                    apply_button(mtm, &buttons[2], cfg.zoom, cfg.diameter, is_key);
                }
                None => {
                    for button in &buttons {
                        apply_button(mtm, button, None, 0.0, is_key);
                    }
                }
            }
        }
    }
    APPLYING.with(|f| f.set(false));
}

fn apply_from_object(obj: Option<&AnyObject>) {
    let Some(obj) = obj else {
        return;
    };
    if let Some(window) = obj.downcast_ref::<NSWindow>() {
        if is_managed_window(window) {
            apply_ns(window);
        }
        return;
    }
    if let Some(view) = obj.downcast_ref::<NSView>() {
        if let Some(window) = view.window() {
            if is_managed_window(&window) {
                apply_ns(&window);
            }
        }
    }
}

/// Drop every per-window registration and overlay on `NSWindowWillClose`, while the buttons and
/// their superviews are still alive.
///
/// Without this the `OVERLAYS` map retains a `Retained<TintView>` per button of a destroyed window
/// forever, and `OBSERVED_BUTTONS` keeps raw pointers that a later allocation can reuse — silently
/// suppressing overlay setup for a new window's buttons.
fn forget_window(ns_window: &NSWindow) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(buttons) = standard_buttons(ns_window) else {
        return;
    };
    let center = NSNotificationCenter::defaultCenter();
    let observer = observer(mtm);
    for button in &buttons {
        let key = Retained::as_ptr(button) as usize;
        if let Some(overlay) = OVERLAYS.with(|m| m.borrow_mut().remove(&key)) {
            overlay.removeFromSuperview();
        }
        if !OBSERVED_BUTTONS.with(|set| set.borrow_mut().remove(&key)) {
            continue;
        }
        unsafe {
            center.removeObserver_name_object(
                &observer,
                Some(NSViewFrameDidChangeNotification),
                Some(button),
            );
        }
    }
}

// ---- Relayout observer: keep overlays aligned + on top the instant a button moves -----------------

thread_local! {
    static RELAYOUT_OBSERVER: RefCell<Option<Retained<TintObserver>>> = const { RefCell::new(None) };
    static OBSERVED_BUTTONS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TishTrafficTintObserver"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct TintObserver;

    unsafe impl NSObjectProtocol for TintObserver {}

    impl TintObserver {
        #[unsafe(method(onRelayout:))]
        fn on_relayout(&self, note: Option<&NSNotification>) {
            guard_objc_callback("onRelayout:", || {
                apply_from_object(note.and_then(|n| n.object()).as_deref());
            });
        }

        #[unsafe(method(onWindowWillClose:))]
        fn on_window_will_close(&self, note: Option<&NSNotification>) {
            guard_objc_callback("onWindowWillClose:", || {
                let Some(obj) = note.and_then(|n| n.object()) else {
                    return;
                };
                if let Some(window) = obj.downcast_ref::<NSWindow>() {
                    forget_window(window);
                }
            });
        }
    }
);

impl TintObserver {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn apply_all_ns() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        if is_managed_window(&window) {
            apply_ns(&window);
        }
    }
}

fn observer(mtm: MainThreadMarker) -> Retained<TintObserver> {
    RELAYOUT_OBSERVER.with(|cell| {
        cell.borrow_mut()
            .get_or_insert_with(|| TintObserver::new(mtm))
            .clone()
    })
}

/// Wire a window-level relayout backstop (once) + a per-button frame observer (once each), so an
/// overlay re-mirrors its button the moment macOS / `traffic_lights` moves it. Idempotent.
fn install_observers(ns_window: &NSWindow) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if OBSERVER_APP.get().is_none() {
        return;
    }
    let center = NSNotificationCenter::defaultCenter();
    let observer = observer(mtm);

    static WINDOW_OBSERVER_INSTALLED: OnceLock<()> = OnceLock::new();
    if WINDOW_OBSERVER_INSTALLED.set(()).is_ok() {
        unsafe {
            center.addObserver_selector_name_object(
                &observer,
                sel!(onRelayout:),
                Some(NSWindowDidUpdateNotification),
                None,
            );
            // Re-assert the tint on focus changes so the overlay tracks macOS: hidden (native grey)
            // when the window resigns key, re-tinted when it becomes key again.
            center.addObserver_selector_name_object(
                &observer,
                sel!(onRelayout:),
                Some(NSWindowDidBecomeKeyNotification),
                None,
            );
            center.addObserver_selector_name_object(
                &observer,
                sel!(onRelayout:),
                Some(NSWindowDidResignKeyNotification),
                None,
            );
            // Teardown hook: drop this window's overlays + observers while they are still valid.
            center.addObserver_selector_name_object(
                &observer,
                sel!(onWindowWillClose:),
                Some(NSWindowWillCloseNotification),
                None,
            );
        }
    }

    let Some(buttons) = standard_buttons(ns_window) else {
        return;
    };
    for button in &buttons {
        let key = Retained::as_ptr(button) as usize;
        if OBSERVED_BUTTONS.with(|set| set.borrow().contains(&key)) {
            continue;
        }
        button.setPostsFrameChangedNotifications(true);
        unsafe {
            center.addObserver_selector_name_object(
                &observer,
                sel!(onRelayout:),
                Some(NSViewFrameDidChangeNotification),
                Some(button),
            );
        }
        OBSERVED_BUTTONS.with(|set| set.borrow_mut().insert(key));
    }
}

/// Re-assert NOW, then a few more times as the window settles (boot + first show re-lay the title bar
/// and move the buttons). Idempotent; the observer handles everything after settle.
fn apply_all_scheduled(app: &tauri::AppHandle) {
    for delay_ms in [0u64, 120, 300, 700, 1400] {
        let handle = app.clone();
        std::thread::spawn(move || {
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            let _ = handle.run_on_main_thread(apply_all_ns);
        });
    }
}

// ---- Public API -----------------------------------------------------------------------------------

/// Store the app handle used by the scheduled re-assert + observer. Call once at setup.
pub fn init(app: &tauri::AppHandle) {
    let _ = OBSERVER_APP.set(app.clone());
}

/// Parse `#rgb` / `#rrggbb` / `#rrggbbaa` into straight sRGB 0..=1 components.
fn parse_hex(s: &str) -> Option<Rgba> {
    let h = s.trim().trim_start_matches('#');
    let hx = |slice: &str| u8::from_str_radix(slice, 16).ok();
    let dup = |slice: &str| u8::from_str_radix(slice, 16).ok().map(|v| v * 16 + v);
    let (r, g, b, a) = match h.len() {
        3 => (dup(&h[0..1])?, dup(&h[1..2])?, dup(&h[2..3])?, 255u8),
        6 => (hx(&h[0..2])?, hx(&h[2..4])?, hx(&h[4..6])?, 255u8),
        8 => (hx(&h[0..2])?, hx(&h[2..4])?, hx(&h[4..6])?, hx(&h[6..8])?),
        _ => return None,
    };
    Some((
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        a as f64 / 255.0,
    ))
}

/// Store the theme-provided tint (hex; `None`/unparseable → that button stays native) and re-apply.
pub fn set_tint(
    app: &tauri::AppHandle,
    close: Option<String>,
    minimize: Option<String>,
    zoom: Option<String>,
    diameter: Option<f64>,
    opacity: Option<f64>,
) {
    let op = opacity.unwrap_or(1.0).clamp(0.0, 1.0);
    let parse = |s: Option<String>| {
        s.as_deref()
            .and_then(parse_hex)
            .map(|(r, g, b, a)| (r, g, b, a * op))
    };
    let cfg = TintConfig {
        close: parse(close),
        minimize: parse(minimize),
        zoom: parse(zoom),
        diameter: diameter.unwrap_or(0.0).max(0.0),
    };
    let any = cfg.close.is_some() || cfg.minimize.is_some() || cfg.zoom.is_some();
    *CONFIG.lock().unwrap() = if any { Some(cfg) } else { None };
    apply_all_scheduled(app);
}

/// Re-assert the current tint (config unchanged) — e.g. after the frontend reveals the window.
pub fn reapply(app: &tauri::AppHandle) {
    apply_all_scheduled(app);
}
