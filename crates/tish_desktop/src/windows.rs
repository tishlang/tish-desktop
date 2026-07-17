use serde_json::json;
use tauri::webview::{PageLoadEvent, PageLoadPayload};
use tauri::{AppHandle, Emitter, Manager, TitleBarStyle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri::window::Color;

use crate::state::WindowSpec;

/// slate-950 — matches example UI chrome so the native layer isn't white.
const WINDOW_BG: Color = Color(0x02, 0x06, 0x17, 255);

pub fn parse_title_bar_style(s: &str) -> TitleBarStyle {
    match s.to_ascii_lowercase().as_str() {
        "overlay" => TitleBarStyle::Overlay,
        "visible" => TitleBarStyle::Visible,
        _ => TitleBarStyle::Transparent,
    }
}

pub fn create_from_spec(app: &AppHandle, spec: &WindowSpec) -> Result<(), String> {
    if app.get_webview_window(&spec.label).is_some() {
        return focus(app, &spec.label);
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

    let mut builder = WebviewWindowBuilder::new(app, &spec.label, url)
        .title(spec.title.clone().unwrap_or_else(|| "Tish Desktop".into()))
        .inner_size(spec.width, spec.height)
        .visible(false)
        .decorations(spec.decorations)
        .background_color(WINDOW_BG)
        .on_page_load(|window: WebviewWindow, payload: PageLoadPayload<'_>| {
            if payload.event() == PageLoadEvent::Finished {
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

    // Safety: if the page never reaches Finished, still surface the window.
    let label = spec.label.clone();
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(8));
        if let Some(w) = app2.get_webview_window(&label) {
            let _ = w.show();
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
    Ok(())
}

pub fn close(app: &AppHandle, label: &str) -> Result<(), String> {
    let win = app
        .get_webview_window(label)
        .ok_or_else(|| format!("window not found: {label}"))?;
    win.close().map_err(|e| e.to_string())
}

pub fn list_labels(app: &AppHandle) -> Vec<String> {
    app.webview_windows().keys().cloned().collect()
}
