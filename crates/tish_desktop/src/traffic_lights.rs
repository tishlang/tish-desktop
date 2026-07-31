//! macOS traffic-light (window control) inset — theme-driven.
//!
//! With `titleBarStyle: "Overlay"` the close/minimize/zoom buttons are the native macOS controls,
//! and Tauri v2 exposes no config for their position, so we move them natively via the objc2-app-kit
//! dependency already used by dock.rs. The positioning algorithm is a port of tauri-plugin-decorum's
//! `position_traffic_lights` (github.com/clearlysid/tauri-plugin-decorum): grow the title-bar
//! CONTAINER view (the close button's grandparent) so its title-bar subview autoresizes taller, then
//! center each button in that taller bar and inset it from the left.
//!
//! The inset is DRIVEN BY THE HOST (not hard-coded): the webview pushes pad-x/pad-y/spacing via the
//! generic `window.trafficLightInset` command whenever it wants a custom layout. `None` (the host
//! sends nothing) restores the untouched macOS default. What the host derives those values from
//! (e.g. theme tokens) is entirely the host's concern — this module only applies them.
//!
//! macOS re-runs its title-bar layout (resetting the button frames to default) on all sorts of
//! events — show, focus, move, resize, AND webview relayouts like selecting a file — far more than
//! Tauri surfaces as window events. Chasing individual triggers never covered them all, so we install
//! a single `NSWindowDidUpdate` notification observer that re-applies on EVERY relayout. `apply` is
//! idempotent (absolute positions + a "skip if already there" guard), so this can't drift or loop.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{
    NSButton, NSView, NSViewFrameDidChangeNotification, NSWindow, NSWindowButton,
    NSWindowDidUpdateNotification,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions,
    NSNotification, NSNotificationCenter, NSObjectNSKeyValueObserverRegistration, NSObjectProtocol,
    NSPoint, NSRect, NSSize, NSString,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::{Mutex, OnceLock};

/// Theme-provided inset. `spacing` is optional — `None` keeps the OS default button spacing.
#[derive(Clone, Copy)]
struct Inset {
    /// Left inset of the close button from the window's left edge.
    pad_x: f64,
    /// Distance from the window's TOP edge down to the TOP of the buttons. We do NOT grow the native
    /// title-bar container to move the buttons down (that bleeds into the content and reads as an
    /// over-tall header) — the container stays at its OS-default height and the buttons are just
    /// placed lower within it, so this is a pure vertical position with no effect on header height.
    pad_y: f64,
    spacing: Option<f64>,
}

/// Active inset, or `None` for the untouched macOS default. Set by `set_traffic_light_inset`.
static CONFIG: Mutex<Option<Inset>> = Mutex::new(None);

/// The untouched-default title-bar container height + button origins, captured ONCE before we first
/// move anything, so switching back to a no-inset theme can restore the exact OS default. The
/// container height is size-independent (title-bar height); button origins are in their (relative)
/// superview space, so both survive window resizes.
struct DefaultSnapshot {
    container_height: f64,
    button_origins: [NSPoint; 3],
}
static DEFAULTS: OnceLock<DefaultSnapshot> = OnceLock::new();

fn standard_buttons(window: &NSWindow) -> Option<[Retained<NSButton>; 3]> {
    let close = window.standardWindowButton(NSWindowButton::CloseButton)?;
    let minimize = window.standardWindowButton(NSWindowButton::MiniaturizeButton)?;
    let zoom = window.standardWindowButton(NSWindowButton::ZoomButton)?;
    Some([close, minimize, zoom])
}

/// Title-bar container = the close button's grandparent (button -> title-bar view -> container).
fn container_of(button: &NSButton) -> Option<Retained<NSView>> {
    // SAFETY: superview() is a plain AppKit accessor; nil is handled by `?`. Main thread only.
    unsafe { button.superview().and_then(|view| view.superview()) }
}

/// Move a button only if it isn't already at `target`. The relayout observer re-applies on every
/// window update, so this both avoids redundant work and prevents a setFrameOrigin→relayout loop.
fn set_origin_if_needed(button: &NSButton, target: NSPoint) {
    let current = button.frame().origin;
    if (current.x - target.x).abs() > 0.5 || (current.y - target.y).abs() > 0.5 {
        button.setFrameOrigin(target);
    }
}

fn capture_defaults(buttons: &[Retained<NSButton>; 3], container: &NSView) -> DefaultSnapshot {
    DefaultSnapshot {
        container_height: container.frame().size.height,
        button_origins: [
            buttons[0].frame().origin,
            buttons[1].frame().origin,
            buttons[2].frame().origin,
        ],
    }
}

fn reposition(ns_window: &NSWindow, inset: Inset) {
    let Some(buttons) = standard_buttons(ns_window) else {
        return;
    };
    let Some(container) = container_of(&buttons[0]) else {
        return;
    };
    // Capture the untouched default the first time we ever move the buttons (they're at OS default
    // here, since a no-inset theme only ever restores, never moves).
    let snapshot = DEFAULTS.get_or_init(|| capture_defaults(&buttons, &container));

    let button_height = buttons[0].frame().size.height;

    // Keep the container at its OS-default height (NO growth — growing it is what pushed the header
    // taller). The buttons' superview fills the container, so a button whose TOP sits `pad_y` below
    // the window top is at superview-y = container_height - pad_y - button_height (AppKit y is
    // bottom-up, container top pinned to the window top).
    let _ = container;
    let button_y = snapshot.container_height - inset.pad_y - button_height;
    let spacing = inset
        .spacing
        .unwrap_or(snapshot.button_origins[1].x - snapshot.button_origins[0].x);
    for (i, button) in buttons.iter().enumerate() {
        set_origin_if_needed(
            button,
            NSPoint {
                x: inset.pad_x + (i as f64) * spacing,
                y: button_y,
            },
        );
    }
}

/// Put the container + buttons back to the captured macOS default (used when the active theme
/// provides no inset). No-op if we've never insetted (buttons are already at the OS default).
fn restore_default(ns_window: &NSWindow) {
    let Some(snapshot) = DEFAULTS.get() else {
        return;
    };
    let Some(buttons) = standard_buttons(ns_window) else {
        return;
    };
    let Some(container) = container_of(&buttons[0]) else {
        return;
    };
    let window_size = ns_window.frame().size;
    // Default container height is size-independent; recompute origin.y/width for the current size.
    container.setFrame(NSRect {
        origin: NSPoint {
            x: 0.0,
            y: window_size.height - snapshot.container_height,
        },
        size: NSSize {
            width: window_size.width,
            height: snapshot.container_height,
        },
    });
    for (i, button) in buttons.iter().enumerate() {
        set_origin_if_needed(button, snapshot.button_origins[i]);
    }
}

fn with_ns_window<F: FnOnce(&NSWindow)>(window: &tauri::WebviewWindow, f: F) {
    let Ok(ns_window_ptr) = window.ns_window() else {
        return;
    };
    if ns_window_ptr.is_null() {
        return;
    }
    // SAFETY: on macOS Tauri returns this webview window's live `NSWindow*`. We only read/write
    // AppKit view geometry, and only from the main thread (Tauri delivers the setup call and window
    // events on the main thread on macOS; the command path dispatches via run_on_main_thread).
    let ns_window: &NSWindow = unsafe { &*(ns_window_ptr as *const NSWindow) };
    f(ns_window);
}

thread_local! {
    // Reentrancy guard: our own `setFrameOrigin` synchronously posts NSViewFrameDidChange / KVO,
    // which re-enter `apply` via the observers. Without this, a single external reset cascades into
    // nested re-applies (3 buttons × N window updates), which — layered under boot-time title-bar
    // relayout — was a big part of the main-thread stall. macOS's *real* resets always arrive with the
    // guard clear (they're separate run-loop turns), so those are still caught; only our own
    // move-induced callbacks are suppressed.
    static APPLYING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Apply the active inset (or restore the OS default) to this window. Idempotent and cheap; called
/// on window creation, on window events, and (via the observers) on every title-bar relayout.
pub fn apply(window: &tauri::WebviewWindow) {
    if APPLYING.with(|f| f.get()) {
        return;
    }
    APPLYING.with(|f| f.set(true));
    let config = *CONFIG.lock().unwrap();
    with_ns_window(window, |ns_window| {
        install_observers(ns_window);
        match config {
            Some(inset) => reposition(ns_window, inset),
            None => restore_default(ns_window),
        }
    });
    APPLYING.with(|f| f.set(false));
}

// ---- Relayout observers: re-assert the inset the instant macOS resets the buttons ----
//
// Two notifications:
//   * NSViewFrameDidChange on each button — posted SYNCHRONOUSLY when macOS moves a button, so we
//     reposition it before the frame is painted (no flicker). This is what catches the reset from
//     opening a file (which retitles the window / relays the title bar).
//   * NSWindowDidUpdate — a coarse backstop for relayouts that don't move the buttons via setFrame.

static OBSERVER_APP: OnceLock<tauri::AppHandle> = OnceLock::new();

thread_local! {
    static RELAYOUT_OBSERVER: RefCell<Option<Retained<TrafficLightObserver>>> =
        const { RefCell::new(None) };
    // Buttons we've already wired frame-change observers to (by pointer), so we do it once each.
    static OBSERVED_BUTTONS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TishTrafficLightObserver"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct TrafficLightObserver;

    unsafe impl NSObjectProtocol for TrafficLightObserver {}

    impl TrafficLightObserver {
        #[unsafe(method(onWindowRelayout:))]
        fn on_window_relayout(&self, _note: Option<&NSNotification>) {
            // No reentrancy guard needed: `set_origin_if_needed` no-ops once a button is at target, so
            // our own setFrameOrigin (which posts this same notification) converges in a few nested
            // calls instead of looping. Suppressing reentrancy here would instead drop macOS's own
            // resets that land mid-apply (e.g. the zoom button's second layout pass) → residual flicker.
            reapply_all();
        }

        // KVO on each button's `frame`: fires SYNCHRONOUSLY on ANY frame change, including the
        // Auto-Layout-driven reposition of the zoom (green) button during a file load — which posts
        // no NSViewFrameDidChange, so it's the only synchronous way to catch that reset before paint.
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            _key_path: Option<&NSString>,
            _object: Option<&AnyObject>,
            _change: Option<&NSDictionary<NSKeyValueChangeKey, AnyObject>>,
            _context: *mut c_void,
        ) {
            reapply_all();
        }
    }
);

fn reapply_all() {
    let Some(app) = OBSERVER_APP.get() else {
        return;
    };
    use tauri::Manager;
    for (_, window) in app.webview_windows() {
        apply(&window);
    }
}

impl TrafficLightObserver {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn observer(mtm: MainThreadMarker) -> Retained<TrafficLightObserver> {
    RELAYOUT_OBSERVER.with(|cell| {
        cell.borrow_mut()
            .get_or_insert_with(|| TrafficLightObserver::new(mtm))
            .clone()
    })
}

/// Wire the window-level backstop observer (once) and a per-button synchronous frame observer for
/// this window's buttons (once each). Idempotent; called from `apply` on the main thread.
fn install_observers(ns_window: &NSWindow) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if OBSERVER_APP.get().is_none() {
        return;
    }
    let center = NSNotificationCenter::defaultCenter();
    let observer = observer(mtm);

    // Window-level backstop, once.
    static WINDOW_OBSERVER_INSTALLED: OnceLock<()> = OnceLock::new();
    if WINDOW_OBSERVER_INSTALLED.set(()).is_ok() {
        unsafe {
            center.addObserver_selector_name_object(
                &observer,
                sel!(onWindowRelayout:),
                Some(NSWindowDidUpdateNotification),
                None,
            );
        }
    }

    // Per-button synchronous frame observers, once per button.
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
                sel!(onWindowRelayout:),
                Some(NSViewFrameDidChangeNotification),
                Some(button),
            );
            // KVO on `frame` too: catches the zoom button's Auto-Layout reposition (which posts no
            // NSViewFrameDidChange) synchronously — the notification path only covers close/minimize.
            button.addObserver_forKeyPath_options_context(
                &observer,
                ns_string!("frame"),
                NSKeyValueObservingOptions::empty(),
                NonNull::dangling().as_ptr(),
            );
        }
        OBSERVED_BUTTONS.with(|set| set.borrow_mut().insert(key));
    }
}

/// Store the app handle the observers use to reach the windows.
pub fn init(app: &tauri::AppHandle) {
    let _ = OBSERVER_APP.set(app.clone());
    // NOTE: there is deliberately NO periodic correction poll. An earlier version re-applied the
    // inset at ~60fps on the main thread to chase the zoom button's reset during a file load, but the
    // real cause of that reset — the window TITLE being rewritten on file-select, which invalidates
    // the title bar — is now fixed at the source (native title reflects the workspace only). A forever
    // main-thread poll only starved the run loop (a multi-second beachball at boot, when macOS is
    // already relaying the title bar hard). Positioning is now purely event-driven: window
    // create/reveal, Resized/Moved/Focused, the theme push schedule, and the relayout observers below.
}

/// Apply the current config to every window NOW, then a few more times as the window settles.
/// macOS re-runs its title-bar layout AFTER our apply (during boot, and again right after the window
/// is shown), which resets the buttons to default and leaves them there until the next focus change
/// — so re-applying on a short schedule rides out that settle. Called from `set_inset` (anchored to
/// the theme push) AND from `reapply` (anchored to the window reveal, which is the later reset).
fn apply_all_scheduled(app: &tauri::AppHandle) {
    for delay_ms in [0u64, 120, 300, 700, 1400] {
        let handle = app.clone();
        std::thread::spawn(move || {
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            let inner = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                use tauri::Manager;
                for (_, window) in inner.webview_windows() {
                    apply(&window);
                }
            });
        });
    }
}

/// Store the theme-provided inset (`None` for the OS default) and re-apply to every window. Called
/// from the `set_traffic_light_inset` Tauri command.
pub fn set_inset(
    app: &tauri::AppHandle,
    pad_x: Option<f64>,
    pad_y: Option<f64>,
    spacing: Option<f64>,
) {
    let config = match (pad_x, pad_y) {
        (Some(x), Some(y)) => Some(Inset {
            pad_x: x,
            pad_y: y,
            spacing,
        }),
        _ => None,
    };
    *CONFIG.lock().unwrap() = config;
    apply_all_scheduled(app);
}

/// Re-assert the current inset (config unchanged). The frontend calls this right after it reveals the
/// window (`w.show()`), whose title-bar layout is the reset that otherwise left the buttons at default
/// until a manual focus change.
pub fn reapply(app: &tauri::AppHandle) {
    apply_all_scheduled(app);
}
