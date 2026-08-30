//! macOS dock-icon override.
//!
//! In a packaged app the dock icon comes from the bundle: `Info.plist`'s `CFBundleIconFile` ->
//! `Resources/icon.icns`. macOS composites that through the system icon mask, so it is already
//! correct — and it is on screen from the moment the app bounces, before any Rust runs.
//!
//! Tauri overrides it at Ready under `#[cfg(all(dev, target_os = "macos"))]` (see `tauri::app`) —
//! and the `dev` cfg is emitted for plain-cargo builds, so this happens in PACKAGED apps too, not
//! just `tauri dev`. The crate's embedded icon is a transparent placeholder, so whatever Tauri
//! sets at Ready is invisible: EVERY build must reassert at Ready or the dock goes blank.
//!
//! Tauri's override runs on `RuntimeRunEvent::Ready`, so `reassert_on_ready` -- called from the
//! host's run-event handler for `RunEvent::Ready` -- lands immediately after it, deterministically.
//! The timed schedule alone could not: `Ready` fires when the event loop is up, which on a slow dev
//! boot is well past the last 2000 ms tick, and the host's icon would show first and then be
//! replaced by Tauri's embedded one.
//!
//! The pre-Ready timed schedule stays dev-only: there the bundle's system-masked `.icns` is
//! already on screen and correct, and replacing it early was the original "icon flash".
//! `setApplicationIconImage:` draws the NSImage RAW,
//! with no system mask, so a host whose `RunConfig.icon` is a full-bleed square (the common case —
//! it doubles as the tray image) had its correctly-masked bundle icon replaced by a hard-cornered
//! square a beat after launch. That swap is the "icon flash".
//!
//! The tray icon is separate, via Tauri's `TrayIconBuilder::icon` — this is only the dock. objc2,
//! so apple + macos only.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{MainThreadMarker, NSBundle, NSString};

// The last icon this module applied: (source path, the exact NSImage instance set). Main-thread
// only (set_dock_icon bails off-main), so a thread_local RefCell suffices — no locks.
//
// Why this exists: on macOS 26 every `setApplicationIconImage:` with a NEW NSImage instance makes
// AppKit composite a fresh dock tile (multi-appearance, f16, GPU-backed) that is cached by image
// identity and never evicted — ~30MB of IOAccelerator memory pinned per call. A host that
// re-asserts its icon on a timer therefore leaks unboundedly. Skipping the set when our image is
// still the one installed makes repeated re-asserts free.
thread_local! {
    static LAST_ICON: RefCell<Option<(String, Retained<NSImage>)>> = const { RefCell::new(None) };
}

/// Whether the running process is a packaged `.app` whose `Info.plist` declares an icon.
///
/// This is the real question — NOT whether this is a debug or release build. `tauri::is_dev()` is
/// `!cfg!(feature = "custom-protocol")`, and a tish host builds with plain cargo rather than the
/// Tauri CLI that would set that feature, so it reports `true` even in a shipped bundle. Asking
/// the bundle directly works no matter how the binary was compiled or launched.
fn bundle_supplies_icon() -> bool {
    let bundle = NSBundle::mainBundle();
    // A loose binary (cargo run, `dist/tish-ide-shell` straight off disk) has no bundle path
    // ending in .app, and no icon key to read.
    let is_app = bundle.bundlePath().to_string().ends_with(".app");
    let key = NSString::from_str("CFBundleIconFile");
    let has_icon = bundle.objectForInfoDictionaryKey(&key).is_some();
    is_app && has_icon
}

/// Set the process's dock icon from a PNG (or any NSImage-readable) file. Returns whether an
/// image was actually loaded and set — callers with a fallback must know. No-op off the main
/// thread or if the image can't be loaded (best-effort branding, never fatal).
///
/// Idempotent: when the requested path is the one already applied AND the application icon still
/// points at the exact NSImage instance we set, this returns true without decoding or setting
/// anything. The pointer compare (not the path compare alone) is load-bearing — if something
/// replaced the icon out from under us (Tauri's Ready override, AppKit), `applicationIconImage`
/// no longer returns our instance and we re-apply, so restore-after-clobber semantics survive.
pub fn set_dock_icon(path: &str) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let app = NSApplication::sharedApplication(mtm);
    let already_applied = LAST_ICON.with(|cell| {
        let cache = cell.borrow();
        let Some((cached_path, cached_img)) = cache.as_ref() else {
            return false;
        };
        if cached_path != path {
            return false;
        }
        match app.applicationIconImage() {
            Some(current) => Retained::as_ptr(&current) == Retained::as_ptr(cached_img),
            None => false,
        }
    });
    if already_applied {
        return true;
    }
    let ns_path = NSString::from_str(path);
    // initWithContentsOfFile: returns nil for an unreadable path — tolerate it.
    let image = unsafe { NSImage::initWithContentsOfFile(NSImage::alloc(), &ns_path) };
    let Some(image) = image else {
        eprintln!("[app_icon] could not load dock icon at {path}");
        return false;
    };
    unsafe { app.setApplicationIconImage(Some(&image)) };
    LAST_ICON.with(|cell| {
        *cell.borrow_mut() = Some((path.to_string(), image));
    });
    true
}

/// The bundle's own icon file (`Info.plist` `CFBundleIconFile` under `Contents/Resources`),
/// tolerating the extensionless convention. None for a loose binary or an iconless bundle.
fn bundle_icns_path() -> Option<String> {
    let bundle = NSBundle::mainBundle();
    let bundle_path = bundle.bundlePath().to_string();
    if !bundle_path.ends_with(".app") {
        return None;
    }
    let key = NSString::from_str("CFBundleIconFile");
    let name = bundle.objectForInfoDictionaryKey(&key)?;
    let name = name.downcast::<NSString>().ok()?.to_string();
    let mut path = format!("{bundle_path}/Contents/Resources/{name}");
    if !std::path::Path::new(&path).exists() {
        path.push_str(".icns");
    }
    std::path::Path::new(&path).exists().then_some(path)
}

/// Reassert the host's dock icon right after Tauri sets its own.
///
/// Tauri's `#[cfg(dev)]` override happens on `RuntimeRunEvent::Ready`
/// (`tauri::app::on_event_loop_event`), which the host observes as `RunEvent::Ready` — and the
/// `dev` cfg is emitted for plain-cargo builds, so it fires in PACKAGED apps too, not just dev
/// runs. Since the crate's embedded icon is a transparent placeholder, whatever Tauri set at
/// Ready is invisible: a packaged app that declines to reassert here ships a BLANK dock icon.
/// That is exactly what v1.3.x did in production when this function still returned early for
/// bundles.
///
/// So: unconditional, and never allowed to leave the transparent image standing. If the host's
/// own icon fails to load, fall back to the bundle's `.icns` (`CFBundleIconFile`), which macOS
/// showed from launch until Ready.
///
/// Must be called on the main thread — the run-event handler already is.
pub fn reassert_on_ready(path: &str) {
    if set_dock_icon(path) {
        return;
    }
    if let Some(icns) = bundle_icns_path() {
        let _ = set_dock_icon(&icns);
    }
}

/// Apply the host's dock icon, in dev only.
///
/// Under `tauri dev` Tauri sets its own embedded icon during startup, after `setup()` runs, so a
/// single early call loses the race. This covers the window before `Ready` (and any later AppKit
/// reset); `reassert_on_ready` is what guarantees the host icon wins Tauri's own set.
///
/// In a release build this is a no-op: the bundle's `.icns` is already the right icon, already
/// system-masked, and already on screen. Overwriting it with a raw NSImage is what made the icon
/// visibly change shortly after launch.
pub fn set_dock_icon_scheduled(app: &tauri::AppHandle, path: String) {
    if bundle_supplies_icon() {
        return;
    }
    for delay_ms in [0u64, 300, 900, 2000] {
        let app = app.clone();
        let path = path.clone();
        std::thread::spawn(move || {
            if delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
            let _ = app.run_on_main_thread(move || {
                let _ = set_dock_icon(&path);
            });
        });
    }
}
