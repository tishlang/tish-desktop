//! macOS dock-icon override, for DEV RUNS ONLY.
//!
//! In a packaged app the dock icon comes from the bundle: `Info.plist`'s `CFBundleIconFile` ->
//! `Resources/icon.icns`. macOS composites that through the system icon mask, so it is already
//! correct — and it is on screen from the moment the app bounces, before any Rust runs.
//!
//! Tauri only overrides it under `#[cfg(all(dev, target_os = "macos"))]` (see `tauri::app`), i.e.
//! `tauri dev`, where there is no bundle and the embedded `tauri.conf.json` icon stands in. That
//! is the ONLY case a host needs to reassert its own icon, so that is the only case we do it.
//!
//! Tauri's override runs on `RuntimeRunEvent::Ready`, so `reassert_on_ready` -- called from the
//! host's run-event handler for `RunEvent::Ready` -- lands immediately after it, deterministically.
//! The timed schedule alone could not: `Ready` fires when the event loop is up, which on a slow dev
//! boot is well past the last 2000 ms tick, and the host's icon would show first and then be
//! replaced by Tauri's embedded one.
//!
//! Doing it in release was actively harmful: `setApplicationIconImage:` draws the NSImage RAW,
//! with no system mask, so a host whose `RunConfig.icon` is a full-bleed square (the common case —
//! it doubles as the tray image) had its correctly-masked bundle icon replaced by a hard-cornered
//! square a beat after launch. That swap is the "icon flash".
//!
//! The tray icon is separate, via Tauri's `TrayIconBuilder::icon` — this is only the dock. objc2,
//! so apple + macos only.

use objc2::AnyThread;
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{MainThreadMarker, NSBundle, NSString};

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

/// Set the process's dock icon from a PNG (or any NSImage-readable) file. No-op off the main thread
/// or if the image can't be loaded (best-effort branding, never fatal).
pub fn set_dock_icon(path: &str) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let ns_path = NSString::from_str(path);
    // initWithContentsOfFile: returns nil for an unreadable path — tolerate it.
    let image = unsafe { NSImage::initWithContentsOfFile(NSImage::alloc(), &ns_path) };
    let Some(image) = image else {
        eprintln!("[app_icon] could not load dock icon at {path}");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.setApplicationIconImage(Some(&image)) };
}

/// Reassert the host's dock icon right after Tauri sets its own.
///
/// Tauri's dev-only override happens on `RuntimeRunEvent::Ready` (`tauri::app::on_event_loop_event`),
/// which the host observes as `RunEvent::Ready`. Running here is exact rather than a race: no delay
/// can be "late enough" in general, because `Ready` waits on the event loop coming up.
///
/// Must be called on the main thread — the run-event handler already is.
pub fn reassert_on_ready(path: &str) {
    if bundle_supplies_icon() {
        return;
    }
    set_dock_icon(path);
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
            let _ = app.run_on_main_thread(move || set_dock_icon(&path));
        });
    }
}
