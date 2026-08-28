//! macOS Dock icon menu: "New Window" plus the list of open windows.
//!
//! Ported from the reference IDE's Tauri-era `dock.rs`. AppKit asks the application delegate for
//! `applicationDockMenu:` on every right-click; a method is grafted onto Tauri's existing
//! delegate class at install time, same as the original.
//!
//! Two rules keep this safe under the host's event loop:
//! - The menu is built ONLY from `windows::window_rows()`, a snapshot held beside Tauri.
//!   `webview_windows()` from inside an AppKit callback re-enters wry's window map
//!   (`RefCell already mutably borrowed`, a process abort).
//! - Menu ACTIONS hop through a thread + `run_on_main_thread` rather than calling window
//!   create/focus inline, so the work runs on a fresh run-loop turn instead of inside the
//!   menu-tracking callback.

use crate::state::AppState;
use crate::windows;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Imp, ProtocolObject, Sel};
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};
use std::cell::RefCell;
use std::ffi::CStr;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

thread_local! {
    static DOCK_MENU_TARGET: RefCell<Option<Retained<DockMenuTarget>>> = const { RefCell::new(None) };
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TishDesktopDockMenuTarget"]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    struct DockMenuTarget;

    unsafe impl NSObjectProtocol for DockMenuTarget {}

    impl DockMenuTarget {
        #[unsafe(method(dockNewWindow:))]
        fn dock_new_window(&self, _sender: Option<&AnyObject>) {
            dispatch(DockAction::NewWindow);
        }

        #[unsafe(method(dockFocusWindow:))]
        fn dock_focus_window(&self, sender: Option<&AnyObject>) {
            let Some(sender) = sender else { return };
            let Some(item) = sender.downcast_ref::<NSMenuItem>() else { return };
            let Some(repr) = item.representedObject() else { return };
            let Some(key) = repr.downcast_ref::<NSString>() else { return };
            dispatch(DockAction::Focus(key.to_string()));
        }
    }
);

impl DockMenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

enum DockAction {
    NewWindow,
    Focus(String),
}

/// Run the action on a FRESH main-thread turn. Directly creating a webview (or even focusing)
/// from inside the menu-action callback runs while AppKit is still tracking the menu; the proxy
/// hop is the same pattern `emit_from_window_event` uses.
fn dispatch(action: DockAction) {
    let Some(app) = APP_HANDLE.get() else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || match action {
            DockAction::NewWindow => {
                // "New Window" clones the BOOT window's spec — same URL, chrome and policy —
                // with the label cleared so create_from_spec allocates win-2, win-3, ….
                let spec = handle
                    .try_state::<AppState>()
                    .and_then(|s| s.config.lock().windows.first().cloned());
                if let Some(mut spec) = spec {
                    spec.label = String::new();
                    let _ = windows::create_from_spec(&handle, &spec);
                }
            }
            DockAction::Focus(label) => {
                let _ = windows::focus(&handle, &label);
            }
        });
    });
}

fn dock_menu_target(mtm: MainThreadMarker) -> Retained<DockMenuTarget> {
    DOCK_MENU_TARGET.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(target) = slot.as_ref() {
            return target.clone();
        }
        let target = DockMenuTarget::new(mtm);
        *slot = Some(target.clone());
        target
    })
}

fn build_ns_dock_menu(rows: &[(String, String, bool)]) -> Retained<NSMenu> {
    let mtm = MainThreadMarker::new().expect("dock menu requires main thread");
    let menu = NSMenu::new(mtm);
    let target = dock_menu_target(mtm);

    let new_item = NSMenuItem::new(mtm);
    new_item.setTitle(&NSString::from_str("New Window"));
    new_item.setEnabled(true);
    unsafe {
        new_item.setTarget(Some(&target));
        new_item.setAction(Some(sel!(dockNewWindow:)));
    }
    menu.addItem(&new_item);

    if !rows.is_empty() {
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        for (label, title, focused) in rows {
            let item = NSMenuItem::new(mtm);
            item.setTitle(&NSString::from_str(title));
            item.setEnabled(true);
            unsafe {
                item.setTarget(Some(&target));
                item.setAction(Some(sel!(dockFocusWindow:)));
                item.setRepresentedObject(Some(&NSString::from_str(label)));
            }
            item.setState(if *focused {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
            menu.addItem(&item);
        }
    }

    menu
}

extern "C" fn application_dock_menu(
    _this: *mut AnyObject,
    _sel: Sel,
    _sender: *mut AnyObject,
) -> *mut AnyObject {
    // windows::window_rows() only — never Tauri — from this callback.
    let rows = windows::window_rows();
    let menu = build_ns_dock_menu(&rows);
    Retained::into_raw(menu) as *mut AnyObject
}

/// Install `applicationDockMenu:` on Tauri's application delegate. Call from `setup` on the main
/// thread. Best-effort: a host without a delegate simply keeps the default Dock menu.
pub fn install(app: &AppHandle) -> Result<(), String> {
    let _ = APP_HANDLE.set(app.clone());
    let mtm =
        MainThreadMarker::new().ok_or_else(|| "dock delegate requires main thread".to_string())?;
    let _ = dock_menu_target(mtm);
    let ns_app = NSApplication::sharedApplication(mtm);
    let delegate = ns_app
        .delegate()
        .ok_or_else(|| "NSApplication has no delegate".to_string())?;
    let delegate_obj: &AnyObject = ProtocolObject::as_ref(&*delegate);
    let delegate_cls: *const AnyClass = delegate_obj.class();
    let sel = sel!(applicationDockMenu:);
    let types = CStr::from_bytes_with_nul(b"@@:@\0").map_err(|e| e.to_string())?;
    let imp: Imp = unsafe {
        std::mem::transmute::<
            unsafe extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
            Imp,
        >(application_dock_menu as unsafe extern "C" fn(_, _, _) -> _)
    };
    let ok = unsafe { objc2::ffi::class_addMethod(delegate_cls.cast_mut(), sel, imp, types.as_ptr()) };
    if ok == objc2::runtime::Bool::NO {
        // Method may already exist on this delegate class — fine, first install wins.
    }
    Ok(())
}
