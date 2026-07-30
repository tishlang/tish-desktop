//! macOS dock-icon override. Tauri sets the app (dock) icon from the bundled `tauri.conf.json`
//! icons at build time; a hosted app that wants its OWN dock icon (Dune IDE, not the tish-desktop
//! default) sets it at runtime here via `NSApplication.setApplicationIconImage:`. The tray icon is
//! handled separately through Tauri's `TrayIconBuilder::icon` — this is only the dock. objc2, so
//! apple + macos only.

use objc2::AnyThread;
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{MainThreadMarker, NSString};

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

/// Apply the dock icon on the main thread at a few delays after launch. Tauri sets its OWN
/// (bundled) app icon during startup — often AFTER `setup()` runs — so a single early call is
/// overwritten. Re-asserting on later run-loop turns makes our icon win (mirrors the traffic-light
/// scheduled re-apply).
pub fn set_dock_icon_scheduled(app: &tauri::AppHandle, path: String) {
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
