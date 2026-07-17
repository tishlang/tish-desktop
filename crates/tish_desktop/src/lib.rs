//! `cargo:tish_desktop` — Tauri 2 host for Tish-first desktop apps.
//!
//! Shell Tish configures handlers/windows then calls `run(config)`.
//! UI talks over broker protocol `desktop/v1` via `desktop_invoke` / events.

mod commands;
mod extensions;
mod fs_sandbox;
mod state;
mod value_util;
mod windows;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tishlang_core::{value_call, NativeFn, Value};

use crate::state::{AppState, PluginFlags, RunConfig, PENDING_CONFIG, PENDING_HANDLERS};
use crate::value_util::{arg_object_json, arg_str, err_str, json_to_value, ok_json};

/// `handle(name, fn)` — register a Tish command handler before `run()`.
pub fn handle(args: &[Value]) -> Value {
    let Some(name) = arg_str(args, 0) else {
        return err_str("handle(name, fn): name required");
    };
    let Some(Value::Function(f)) = args.get(1).cloned() else {
        return err_str("handle(name, fn): function required");
    };
    PENDING_HANDLERS.lock().insert(name, f);
    ok_json(serde_json::json!({ "ok": true }))
}

/// `useExtensions(["id", ...])` — record enabled extension ids for the next run.
pub fn use_extensions(args: &[Value]) -> Value {
    let ids: Vec<String> = match args.first() {
        Some(Value::Array(a)) => a
            .borrow()
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        Some(Value::String(s)) => vec![s.to_string()],
        _ => Vec::new(),
    };
    let mut cfg = PENDING_CONFIG.lock();
    let mut c = cfg.clone().unwrap_or_default();
    c.extensions = ids.clone();
    for id in &ids {
        let _ = extensions::REGISTRY.register_tish(extensions::ExtensionManifest {
            id: id.clone(),
            shell: None,
            ui: None,
            commands: vec![],
            permissions: vec!["fs:scoped".into()],
        });
    }
    *cfg = Some(c);
    ok_json(serde_json::json!({ "ok": true, "extensions": ids }))
}

/// `registerRustExtension(name)` — mark a Rust/cargo module as loaded.
pub fn register_rust_extension(args: &[Value]) -> Value {
    let Some(name) = arg_str(args, 0) else {
        return err_str("name required");
    };
    extensions::REGISTRY.register_rust_module(name);
    ok_json(serde_json::json!({ "ok": true }))
}

/// `createWindow(spec)` — only valid after run has started; prefer windows in run(config).
pub fn create_window(args: &[Value]) -> Value {
    let Some(spec_json) = arg_object_json(args, 0) else {
        return err_str("createWindow(spec) requires object");
    };
    // Queued into pending config if not running yet
    let mut cfg = PENDING_CONFIG.lock();
    let mut c = cfg.clone().unwrap_or_default();
    match serde_json::from_value::<state::WindowSpec>(spec_json) {
        Ok(spec) => {
            c.windows.push(spec);
            *cfg = Some(c);
            ok_json(serde_json::json!({ "ok": true, "queued": true }))
        }
        Err(e) => err_str(e.to_string()),
    }
}

pub fn list_windows(_args: &[Value]) -> Value {
    ok_json(serde_json::json!([]))
}

pub fn focus_window(_args: &[Value]) -> Value {
    err_str("focusWindow: use broker invoke window.focus after run()")
}

pub fn close_window(_args: &[Value]) -> Value {
    err_str("closeWindow: use broker invoke window.close after run()")
}

/// `run(config?)` — start Tauri event loop (blocks until app exits).
pub fn run(args: &[Value]) -> Value {
    let raw = arg_object_json(args, 0);
    let mut config = match &raw {
        Some(v) => match serde_json::from_value::<RunConfig>(v.clone()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tish_desktop: run config parse failed ({e}); using defaults + fallbacks");
                RunConfig::default()
            }
        },
        None => RunConfig::default(),
    };
    // Fallback if full struct parse failed but tickMs is present (Tish numbers are f64).
    if config.tick_ms.is_none() {
        if let Some(ms) = raw
            .as_ref()
            .and_then(|v| v.get("tickMs"))
            .and_then(|n| n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)))
        {
            config.tick_ms = Some(ms);
        }
    }

    if let Some(pending) = PENDING_CONFIG.lock().clone() {
        if config.windows.is_empty() {
            config.windows = pending.windows;
        } else {
            config.windows.extend(pending.windows);
        }
        if config.fs_root.is_none() {
            config.fs_root = pending.fs_root;
        }
        if config.extensions.is_empty() {
            config.extensions = pending.extensions;
        }
        // Do not merge pending.plugins — createWindow/useExtensions only queue
        // windows/extensions; plugin flags come solely from run(config).
        // (Pending RunConfig::default() used to AND-disable every plugin.)
        if config.tick_ms.is_none() {
            config.tick_ms = pending.tick_ms;
        }
    }

    if config.windows.is_empty() {
        config.windows.push(state::WindowSpec {
            label: "main".into(),
            url: Some("http://localhost:5173/".into()),
            title: Some("Tish Desktop".into()),
            width: 960.0,
            height: 640.0,
            title_bar_style: "transparent".into(),
            hidden_title: false,
            decorations: true,
        });
    }

    let handlers = std::mem::take(&mut *PENDING_HANDLERS.lock());
    let tick_ms = config.tick_ms.unwrap_or(0);
    if tick_ms > 0 {
        eprintln!("tish_desktop: tick loop interval {tick_ms}ms");
    }
    let plugins = config.plugins.clone();
    let window_specs = config.windows.clone();

    let state = AppState::new(config);
    for (k, v) in handlers {
        state.register_handler(k, v);
    }

    let result = build_and_run(state, window_specs, plugins, tick_ms);
    match result {
        Ok(()) => Value::Null,
        Err(e) => {
            eprintln!("tish_desktop run error: {e}");
            err_str(e)
        }
    }
}

fn build_and_run(
    state: AppState,
    window_specs: Vec<state::WindowSpec>,
    plugins: PluginFlags,
    tick_ms: u64,
) -> Result<(), String> {
    let mut builder = tauri::Builder::default();

    if plugins.single_instance {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = windows::focus(app, "main");
        }));
    }
    if plugins.dialog {
        builder = builder.plugin(tauri_plugin_dialog::init());
    }
    if plugins.opener {
        builder = builder.plugin(tauri_plugin_opener::init());
    }
    if plugins.deep_link {
        builder = builder.plugin(tauri_plugin_deep_link::init());
    }
    if plugins.notification {
        builder = builder.plugin(tauri_plugin_notification::init());
    }

    builder
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::desktop_protocol,
            commands::desktop_invoke,
            commands::desktop_emit_tick,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            if plugins.deep_link {
                use tauri_plugin_deep_link::DeepLinkExt;
                let dl = handle.clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let _ = dl.emit("deep-link", serde_json::json!({ "url": url.to_string() }));
                    }
                });
            }

            if plugins.menu {
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let show = MenuItem::with_id(app, "menu:show", "Show Main", true, None::<&str>)?;
                let sep = PredefinedMenuItem::separator(app)?;
                let file_menu = Submenu::with_items(app, "File", true, &[&show, &sep, &quit])?;
                let menu = Menu::with_items(app, &[&file_menu])?;
                app.set_menu(menu)?;
                let h = handle.clone();
                app.on_menu_event(move |_app, event| {
                    let id = event.id().0.clone();
                    if id == "quit" {
                        _app.exit(0);
                    } else {
                        let _ = h.emit("menu:action", serde_json::json!({ "id": id }));
                        if id == "menu:show" {
                            let _ = windows::focus(&h, "main");
                        }
                    }
                });
            }

            if plugins.tray {
                let show_i = MenuItem::with_id(app, "tray:show", "Show", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;
                let tray_menu = Menu::with_items(app, &[&show_i, &quit_i])?;
                let tray = TrayIconBuilder::new()
                    .menu(&tray_menu)
                    .tooltip("Tish Desktop")
                    .on_menu_event(move |app, event| {
                        let id = event.id().0.clone();
                        let _ = app.emit("tray:action", serde_json::json!({ "id": id }));
                        if id == "tray:quit" {
                            app.exit(0);
                        } else if id == "tray:show" {
                            let _ = windows::focus(app, "main");
                        }
                    })
                    .on_tray_icon_event(move |tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            let _ = windows::focus(app, "main");
                        }
                    })
                    .build(app)?;
                *app.state::<AppState>().tray.lock() = Some(tray);
            }

            for spec in &window_specs {
                windows::create_from_spec(&handle, spec)?;
            }

            if tick_ms > 0 {
                commands::spawn_tick_loop(handle.clone(), tick_ms);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| e.to_string())
}

/// Export module object for `tish:` npm-style packages (optional).
pub fn tish_desktop_object() -> Value {
    use tishlang_core::ObjectMap;
    let mut m = ObjectMap::default();
    m.insert(Arc::from("run"), Value::native(run));
    m.insert(Arc::from("handle"), Value::native(handle));
    m.insert(Arc::from("useExtensions"), Value::native(use_extensions));
    m.insert(
        Arc::from("registerRustExtension"),
        Value::native(register_rust_extension),
    );
    m.insert(Arc::from("createWindow"), Value::native(create_window));
    m.insert(Arc::from("listWindows"), Value::native(list_windows));
    m.insert(Arc::from("focusWindow"), Value::native(focus_window));
    m.insert(Arc::from("closeWindow"), Value::native(close_window));
    m.insert(
        Arc::from("protocol"),
        Value::String(state::PROTOCOL_VERSION.into()),
    );
    Value::object(m)
}

// silence unused import warnings for helpers used by extensions path
#[allow(dead_code)]
fn _touch_native(f: NativeFn) {
    let _ = value_call(&Value::Function(f), &[]);
}

#[allow(dead_code)]
fn _touch_json(v: Value) -> Value {
    json_to_value(&value_util::value_to_json(&v).unwrap_or(serde_json::Value::Null))
}
