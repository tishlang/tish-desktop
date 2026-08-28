use std::cell::RefCell;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, Manager, TitleBarStyle, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::broker::{SurfaceInfo, SurfaceKind, GLOBAL_SURFACES};
use crate::state::{AppState, WindowSpec};

/// slate-950 — matches example UI chrome so the native layer isn't white.
const WINDOW_BG: Color = Color(0x02, 0x06, 0x17, 255);

/// Invoking webview for the current `desktop_invoke` (and nested `brokerInvoke`).
thread_local! {
    static CALLER_LABEL: RefCell<Option<String>> = RefCell::new(None);
}

struct ManagerState {
    last_focused: Option<String>,
    /// Open windows in creation order, `(label, title)`. Kept HERE, off to the side of Tauri,
    /// because the Dock-menu callback (`applicationDockMenu:`) must not call into wry: AppKit
    /// invokes it mid-run-loop, and `webview_windows()` re-enters wry's window map — the same
    /// `RefCell already mutably borrowed` abort the observer rework fixed.
    rows: Vec<(String, String)>,
}

impl ManagerState {
    fn new() -> Self {
        Self {
            last_focused: None,
            rows: Vec::new(),
        }
    }
}

static MANAGER: Lazy<Mutex<ManagerState>> = Lazy::new(|| Mutex::new(ManagerState::new()));

pub fn set_caller_label(label: Option<&str>) {
    CALLER_LABEL.with(|c| *c.borrow_mut() = label.map(|s| s.to_string()));
}

pub fn caller_label() -> Option<String> {
    CALLER_LABEL.with(|c| c.borrow().clone())
}

pub fn parse_title_bar_style(s: &str) -> TitleBarStyle {
    match s.to_ascii_lowercase().as_str() {
        "overlay" => TitleBarStyle::Overlay,
        "visible" => TitleBarStyle::Visible,
        _ => TitleBarStyle::Transparent,
    }
}

pub fn next_window_label(app: &AppHandle) -> String {
    let mut n = 2u32;
    loop {
        let label = format!("win-{n}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
        n += 1;
        if n > 10_000 {
            return format!(
                "win-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
        }
    }
}

fn register_surface(spec: &WindowSpec) {
    let id = spec.id.clone().unwrap_or_else(|| spec.label.clone());
    let kind = match spec.kind.as_deref() {
        Some("native") => SurfaceKind::Native,
        Some("web") => SurfaceKind::Web,
        _ => SurfaceKind::Webview,
    };
    GLOBAL_SURFACES.register(SurfaceInfo {
        id,
        kind,
        platform: None,
        label: Some(spec.label.clone()),
    });
}

pub fn unregister_surface(label: &str) {
    for s in GLOBAL_SURFACES.list() {
        if s.id == label || s.label.as_deref() == Some(label) {
            GLOBAL_SURFACES.unregister(&s.id);
        }
    }
}

fn register_label(label: &str, title: &str) {
    let mut mgr = MANAGER.lock();
    if mgr.last_focused.is_none() {
        mgr.last_focused = Some(label.to_string());
    }
    if !mgr.rows.iter().any(|(l, _)| l == label) {
        mgr.rows.push((label.to_string(), title.to_string()));
    }
}

/// Keep the Dock-menu row title in sync when the host retitles a window (`window.title`).
pub fn note_title(label: &str, title: &str) {
    let mut mgr = MANAGER.lock();
    if let Some(row) = mgr.rows.iter_mut().find(|(l, _)| l == label) {
        row.1 = title.to_string();
    }
}

/// Snapshot for the Dock menu: `(label, title, focused)`. Reads only MANAGER — safe from
/// AppKit callbacks, never touches Tauri.
pub fn window_rows() -> Vec<(String, String, bool)> {
    let mgr = MANAGER.lock();
    let focused = mgr.last_focused.clone();
    mgr.rows
        .iter()
        .map(|(l, t)| (l.clone(), t.clone(), focused.as_deref() == Some(l.as_str())))
        .collect()
}

pub fn note_focused(label: &str) {
    let mut mgr = MANAGER.lock();
    mgr.last_focused = Some(label.to_string());
}

pub fn last_focused_label() -> Option<String> {
    MANAGER.lock().last_focused.clone()
}

/// Resolve the target window: explicit `args.label`, else the invoking webview.
/// Never falls back to `"main"` unless that is the caller.
pub fn resolve(
    app: &AppHandle,
    args: &serde_json::Value,
    caller: Option<&str>,
) -> Result<WebviewWindow, String> {
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .or(caller)
        .ok_or_else(|| "window label required (no invoking window)".to_string())?;
    app.get_webview_window(label)
        .ok_or_else(|| format!("window not found: {label}"))
}

fn apply_create_policy(app: &AppHandle, spec: &mut WindowSpec) {
    let existing = list_labels(app);
    if existing.is_empty() {
        return;
    }
    // Extra windows are always shown. `visible: false` is boot-flash for the first window only.
    spec.visible = true;
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(first) = state.config.lock().windows.first() {
            spec.transparent = first.transparent;
        }
    }
}

pub fn create_from_spec(app: &AppHandle, spec: &WindowSpec) -> Result<(), String> {
    let mut spec = spec.clone();
    if spec.label.is_empty() {
        spec.label = next_window_label(app);
    }
    apply_create_policy(app, &mut spec);
    if spec.label.is_empty() {
        return Err("label required".into());
    }

    let url = if let Some(u) = &spec.url {
        if u.starts_with("http://") || u.starts_with("https://") {
            WebviewUrl::External(u.parse().map_err(|e| format!("bad url: {e}"))?)
        } else {
            WebviewUrl::App(u.clone().into())
        }
    } else {
        WebviewUrl::App("index.html".into())
    };

    let show_on_pageload = spec.visible;
    let mut builder = WebviewWindowBuilder::new(app, &spec.label, url)
        .title(spec.title.clone().unwrap_or_else(|| "Tish Desktop".into()))
        .inner_size(spec.width, spec.height)
        .visible(false)
        .decorations(spec.decorations)
        .initialization_script(format!(
            "window.__TISH_DESKTOP__ = true; window.__TISH_WINDOW_LABEL__ = {};",
            serde_json::to_string(&spec.label).unwrap_or_else(|_| "\"main\"".into())
        ));

    if spec.transparent {
        builder = builder
            .transparent(true)
            .background_color(Color(0, 0, 0, 0));
    } else {
        builder = builder.background_color(WINDOW_BG);
    }

    builder = builder.on_page_load(move |window: WebviewWindow, payload: PageLoadPayload<'_>| {
        // Default: first Finished reveals a window that started hidden. Later
        // Finished events (iframes, HMR, in-page navigations) must NOT show()
        // again: on macOS show() is makeKeyAndOrderFront and steals key focus.
        // `visible: false` defers reveal to the webview (Dune waits until the
        // workbench is themed + mounted) so the user never sees a blank shell.
        if show_on_pageload
            && payload.event() == PageLoadEvent::Finished
            && !window.is_visible().unwrap_or(false)
        {
            let _ = window.show();
        }
    });

    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(parse_title_bar_style(&spec.title_bar_style))
            .hidden_title(spec.hidden_title);
    }

    #[cfg(debug_assertions)]
    {
        builder = builder.devtools(true);
    }

    let win = builder.build().map_err(|e| e.to_string())?;
    register_surface(&spec);
    register_label(
        &spec.label,
        spec.title.as_deref().unwrap_or(&spec.label),
    );

    let label_for_drop = spec.label.clone();
    let app_for_drop = app.clone();
    win.on_webview_event(move |event| {
        use tauri::{DragDropEvent, WebviewEvent};
        if let WebviewEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
            let paths: Vec<String> = paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let _ = app_for_drop.emit(
                "file-drop",
                json!({ "paths": paths, "label": label_for_drop }),
            );
        }
    });

    Ok(())
}

pub fn focus(app: &AppHandle, label: &str) -> Result<(), String> {
    let win = app
        .get_webview_window(label)
        .ok_or_else(|| format!("window not found: {label}"))?;
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
    note_focused(label);
    Ok(())
}

pub fn focus_last(app: &AppHandle) -> Result<(), String> {
    let label = last_focused_label()
        .or_else(|| list_labels(app).into_iter().next())
        .ok_or_else(|| "no windows".to_string())?;
    focus(app, &label)
}

fn destroy_label(app: &AppHandle, label: &str) -> Result<(), String> {
    let win = app
        .get_webview_window(label)
        .ok_or_else(|| format!("window not found: {label}"))?;
    win.destroy().map_err(|e| e.to_string())
}

/// CloseRequested on `label` — destroy that window only.
pub fn close_from_native(app: &AppHandle, label: &str) -> Result<(), String> {
    destroy_label(app, label)
}

/// Broker `window.close`. Closing `main` is refused unless the caller is `main`.
pub fn close_from_broker(app: &AppHandle, label: &str, caller: Option<&str>) -> Result<(), String> {
    if label == "main" && caller != Some("main") {
        return Err("refusing to close main from another window".into());
    }
    destroy_label(app, label)
}

pub fn close(app: &AppHandle, label: &str) -> Result<(), String> {
    close_from_broker(app, label, caller_label().as_deref())
}

pub fn on_destroyed(label: &str) {
    unregister_surface(label);
    let mut mgr = MANAGER.lock();
    if mgr.last_focused.as_deref() == Some(label) {
        mgr.last_focused = None;
    }
    mgr.rows.retain(|(l, _)| l != label);
}

pub fn list_labels(app: &AppHandle) -> Vec<String> {
    app.webview_windows().keys().cloned().collect()
}

/// `TISH_WINDOW_LIFECYCLE_TEST=1`: create win-2, close it, assert main survived, recreate win-2.
pub fn spawn_lifecycle_selftest(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2000));
        let url = app.try_state::<crate::state::AppState>().and_then(|s| {
            s.config
                .lock()
                .windows
                .first()
                .and_then(|w| w.url.clone())
        });
        let spec_for = |label: &str| WindowSpec {
            label: label.into(),
            kind: Some("webview".into()),
            id: None,
            url: url.clone(),
            title: Some(label.into()),
            width: 640.0,
            height: 480.0,
            title_bar_style: "overlay".into(),
            hidden_title: true,
            decorations: true,
            layout: None,
            visible: true,
            transparent: false,
        };

        let create1 = create_from_spec(&app, &spec_for("win-2"));
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let after_create1 = list_labels(&app);

        let refuse_main = close_from_broker(&app, "main", Some("win-2"));
        std::thread::sleep(std::time::Duration::from_millis(200));
        let after_refuse = list_labels(&app);

        let close = close_from_native(&app, "win-2");
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let after_close = list_labels(&app);

        let create2 = create_from_spec(&app, &spec_for("win-2"));
        std::thread::sleep(std::time::Duration::from_millis(1200));
        let after_create2 = list_labels(&app);

        let has = |labels: &[String], l: &str| labels.iter().any(|x| x == l);
        let main_survived_close = has(&after_close, "main") && !has(&after_close, "win-2");
        let ok = create1.is_ok()
            && refuse_main.is_err()
            && has(&after_refuse, "main")
            && close.is_ok()
            && create2.is_ok()
            && has(&after_create1, "main")
            && has(&after_create1, "win-2")
            && main_survived_close
            && has(&after_create2, "main")
            && has(&after_create2, "win-2");

        let report = json!({
            "ok": ok,
            "create1": format!("{create1:?}"),
            "after_create1": after_create1,
            "refuse_main": format!("{refuse_main:?}"),
            "after_refuse": after_refuse,
            "close": format!("{close:?}"),
            "after_close": after_close,
            "main_survived_close": main_survived_close,
            "create2": format!("{create2:?}"),
            "after_create2": after_create2,
        });
        let _ = std::fs::write("/tmp/tish-win-lifecycle.json", report.to_string());
        if !ok && !has(&after_close, "main") {
            eprintln!("TISH_WINDOW_LIFECYCLE_TEST: main disappeared after closing win-2");
        }
        app.exit(if ok { 0 } else { 1 });
    });
}
