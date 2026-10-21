//! Native traffic lights for SHAPED (borderless, transparent) windows — macOS.
//!
//! A shaped window has no title bar, so AppKit gives it no close / minimize / zoom buttons. We put
//! the system's OWN buttons back (`+[NSWindow standardWindowButton:forStyleMask:]`) — not lookalikes
//! — so every native behavior stays: click, the hover glyphs, press-and-hold / hover on zoom (the
//! window-tiling menu), the focus greying.
//!
//! The hover glyphs are drawn only when the buttons' SUPERVIEW reports the mouse inside the button
//! group (`_mouseInGroup:`, which the title bar implements). So the buttons live in a small container
//! view that tracks mouse enter/exit over itself and answers `_mouseInGroup:` from that — exactly
//! what the title bar does.
//!
//! Position follows the host's traffic-light inset (`traffic_lights::current_inset`), and
//! `traffic_light_tint.rs` finds these buttons through [`buttons`], so a theme's tint applies here
//! the same as on a titled window.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSButton, NSEvent, NSView, NSWindow, NSWindowButton,
    NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::Cell;

/// Tag of the container; buttons are tagged CONTAINER_TAG + 1..=3 (close, minimize, zoom).
const CONTAINER_TAG: isize = 0x7154_4c00;
/// The OS-default gap between the buttons.
const DEFAULT_SPACING: f64 = 6.0;

#[derive(Default)]
struct GroupIvars {
    hovered: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "TishShapedTrafficLights"]
    #[thread_kind = MainThreadOnly]
    #[ivars = GroupIvars]
    struct GroupView;

    impl GroupView {
        // The standard window buttons ask their superview whether the mouse is in the group to
        // decide whether to draw their glyphs (the title bar's private contract).
        #[unsafe(method(_mouseInGroup:))]
        fn mouse_in_group(&self, _button: Option<&AnyObject>) -> Bool {
            Bool::new(self.ivars().hovered.get())
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.set_hovered(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.set_hovered(false);
        }

        #[unsafe(method(tag))]
        fn tag(&self) -> isize {
            CONTAINER_TAG
        }
    }
);

impl GroupView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(GroupIvars::default());
        let view: Retained<Self> = unsafe { msg_send![super(this), initWithFrame: frame] };
        view.install_tracking();
        view
    }

    /// One tracking area over the whole group, kept in sync with its bounds by AppKit.
    fn install_tracking(&self) {
        let Some(cls) = AnyClass::get(c"NSTrackingArea") else {
            return;
        };
        // MouseEnteredAndExited (0x01) | ActiveAlways (0x80) | InVisibleRect (0x200)
        let options: usize = 0x01 | 0x80 | 0x200;
        let area: *mut AnyObject = unsafe { msg_send![cls, alloc] };
        let area: *mut AnyObject = unsafe {
            msg_send![area, initWithRect: NSRect::ZERO, options: options, owner: self, userInfo: std::ptr::null::<AnyObject>()]
        };
        if !area.is_null() {
            // addTrackingArea: retains it; balance our +1 from alloc/init.
            unsafe {
                let _: () = msg_send![self, addTrackingArea: area];
                let _: () = msg_send![area, release];
            }
        }
    }

    fn set_hovered(&self, hovered: bool) {
        self.ivars().hovered.set(hovered);
        for sub in self.subviews().iter() {
            sub.setNeedsDisplay(true);
        }
    }
}

fn container(window: &NSWindow) -> Option<Retained<NSView>> {
    let content = window.contentView()?;
    content.subviews().iter().find(|v| v.tag() == CONTAINER_TAG)
}

/// The shaped window's close / minimize / zoom buttons, if it carries them.
pub(crate) fn buttons(window: &NSWindow) -> Option<[Retained<NSButton>; 3]> {
    let group = container(window)?;
    let find = |tag: isize| -> Option<Retained<NSButton>> {
        let view = group.subviews().iter().find(|v| v.tag() == tag)?;
        view.downcast::<NSButton>().ok()
    };
    Some([find(CONTAINER_TAG + 1)?, find(CONTAINER_TAG + 2)?, find(CONTAINER_TAG + 3)?])
}

/// Add (shaped) or remove (titled again — the title bar brings its own) the native buttons.
pub(crate) fn sync(window: &NSWindow, shaped: bool) {
    if let Some(old) = container(window) {
        old.removeFromSuperview();
    }
    if !shaped {
        return;
    }
    let (Some(mtm), Some(content)) = (MainThreadMarker::new(), window.contentView()) else {
        return;
    };
    let (pad_x, pad_y, spacing) =
        crate::traffic_lights::current_inset().unwrap_or((12.0, 12.0, None));
    let spacing = spacing.unwrap_or(DEFAULT_SPACING);
    let mask = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;
    let kinds = [
        (NSWindowButton::CloseButton, sel!(performClose:)),
        (NSWindowButton::MiniaturizeButton, sel!(performMiniaturize:)),
        (NSWindowButton::ZoomButton, sel!(performZoom:)),
    ];
    let mut made: Vec<Retained<NSButton>> = Vec::new();
    for (kind, action) in kinds {
        let Some(button) = NSWindow::standardWindowButton_forStyleMask(kind, mask, mtm) else {
            return;
        };
        unsafe {
            button.setTarget(Some(window));
            button.setAction(Some(action));
        }
        made.push(button);
    }
    let size = made[0].frame().size;
    let width = size.width * 3.0 + spacing * 2.0;
    let height = content.frame().size.height;
    let flipped = content.isFlipped();
    let y = if flipped { pad_y } else { height - pad_y - size.height };
    let group = GroupView::new(mtm, NSRect::new(NSPoint::new(pad_x, y), NSSize::new(width, size.height)));
    let top = if flipped {
        NSAutoresizingMaskOptions::ViewMaxYMargin
    } else {
        NSAutoresizingMaskOptions::ViewMinYMargin
    };
    group.setAutoresizingMask(NSAutoresizingMaskOptions::ViewMaxXMargin | top);
    for (i, button) in made.iter().enumerate() {
        button.setFrameOrigin(NSPoint::new(i as f64 * (size.width + spacing), 0.0));
        button.setTag(CONTAINER_TAG + 1 + i as isize);
        group.addSubview(button);
    }
    content.addSubview(&group);
}

/// Re-place the buttons of every shaped window (the host pushed a new inset).
pub(crate) fn relayout_all() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    for window in app.windows().iter() {
        if container(&window).is_some() {
            sync(&window, true);
        }
    }
}
