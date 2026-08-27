//! `cargo:tishlang_desktop` / `cargo:tishlang_app` — cross-device app runtime host.
//!
//! Shell Tish configures handlers/surfaces then calls `run(config)`.
//! UI talks over broker protocol `desktop/v1` via `desktop_invoke` / events.
//! Shared microfrontend state is `state.*` (persisted KV remains `store.*`).

mod app_open;
mod broker;
mod local_invoke;
mod caps;
mod commands;
mod extensions;
mod fs_sandbox;
mod platform;
mod state;
mod value_util;
mod webview_resource;
mod windows;
// macOS window-chrome (traffic-light inset + tint), ported from the reference IDE. objc2, so apple+macos only.
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
mod app_icon;
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
mod traffic_lights;
#[cfg(all(feature = "platform-apple", target_os = "macos"))]
mod traffic_light_tint;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};
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

/// `emit(event, payload?, opts?)` — emit a Tauri event to the webview(s). `opts.window` targets a
/// single window by label; otherwise it broadcasts to all. This is how shell Tish (and, via
/// `emit_event`, Rust extension crates) push streaming output — pty/lsp/dap chunks, watcher
/// notifications, chat deltas — that the request/response `desktop_invoke` path cannot carry.
/// Returns `{ ok: bool }` (false when no AppHandle is live yet, e.g. before `run()`).
pub fn emit(args: &[Value]) -> Value {
    let Some(event) = arg_str(args, 0) else {
        return err_str("emit(event, payload?, opts?): event required");
    };
    let payload = args
        .get(1)
        .and_then(value_util::value_to_json)
        .unwrap_or(serde_json::Value::Null);
    let window = match args.get(2) {
        Some(Value::Object(m)) => match m.borrow().strings.get("window") {
            Some(Value::String(s)) => Some(s.to_string()),
            _ => None,
        },
        _ => None,
    };
    ok_json(serde_json::json!({ "ok": emit_event_to(&event, payload, window.as_deref()) }))
}

/// Emit a Tauri event from Rust extension code (broadcasts to all webviews). The Value-ABI
/// `emit()` above is the Tish-facing entry; this is the direct Rust one for `cargo:` extensions.
pub fn emit_event(event: &str, payload: serde_json::Value) -> bool {
    emit_event_to(event, payload, None)
}

fn emit_event_to(event: &str, payload: serde_json::Value, window: Option<&str>) -> bool {
    let Some(app) = local_invoke::APP_HANDLE.lock().clone() else {
        return false;
    };
    match window {
        Some(label) => match app.get_webview_window(label) {
            Some(w) => w.emit(event, payload).is_ok(),
            None => false,
        },
        None => app.emit(event, payload).is_ok(),
    }
}

/// `AppHandle::emit` from inside `on_window_event` re-enters wry's window map on the
/// main thread (`RefCell already mutably borrowed`) and kills the whole process — every
/// window dies with it. Off-thread emit uses the event-loop proxy instead of running
/// `handle_user_message` inline.
fn emit_from_window_event(app: &tauri::AppHandle, event: &'static str, payload: serde_json::Value) {
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = app.emit(event, payload);
    });
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

/// `createWindow(spec)` — queue a webview window (prefer `createSurface` for kind).
pub fn create_window(args: &[Value]) -> Value {
    let Some(mut spec_json) = arg_object_json(args, 0) else {
        return err_str("createWindow(spec) requires object");
    };
    if spec_json.get("kind").is_none() {
        if let Some(obj) = spec_json.as_object_mut() {
            obj.insert("kind".into(), serde_json::json!("webview"));
        }
    }
    queue_surface_spec(spec_json)
}

/// `createSurface({ id?, label, kind: "webview"|"native", url?, root?, … })`
pub fn create_surface(args: &[Value]) -> Value {
    let Some(spec_val) = args.first() else {
        return err_str("createSurface(spec) requires object");
    };
    // Pull `root` off the live Value first — JSON conversion omits functions.
    let root_fn = match spec_val {
        Value::Object(o) => o.borrow().strings.get("root").cloned(),
        _ => None,
    };
    let Some(mut spec_json) = arg_object_json(args, 0) else {
        return err_str("createSurface(spec) requires object");
    };
    let kind = spec_json
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("webview")
        .to_string();

    let has_root = root_fn.is_some();

    if kind == "native" {
        let host = platform::current_host();
        let id = spec_json
            .get("id")
            .or_else(|| spec_json.get("label"))
            .and_then(|v| v.as_str())
            .unwrap_or("native")
            .to_string();
        broker::GLOBAL_SURFACES.register(broker::SurfaceInfo {
            id: id.clone(),
            kind: broker::SurfaceKind::Native,
            platform: Some(host.name().into()),
            label: Some(id.clone()),
        });
        if let Some(root) = root_fn {
            broker::PENDING_NATIVE_ROOTS
                .lock()
                .push((id.clone(), root));
        }
        if !host.supports_native_surface() {
            return ok_json(broker::unsupported_on("createSurface.native", host.name()));
        }
        return ok_json(serde_json::json!({
            "ok": true,
            "queued": true,
            "kind": "native",
            "host": host.name(),
            "hasRoot": has_root,
            "hint": "run({ platformAttach: { apple: { outerHost: true } } }) drains PENDING_NATIVE_ROOTS via attach_app",
        }));
    }

    if let Some(obj) = spec_json.as_object_mut() {
        obj.remove("root"); // not part of WindowSpec JSON
    }
    queue_surface_spec(spec_json)
}

fn queue_surface_spec(spec_json: serde_json::Value) -> Value {
    let mut cfg = PENDING_CONFIG.lock();
    let mut c = cfg.clone().unwrap_or_default();
    match serde_json::from_value::<state::WindowSpec>(spec_json) {
        Ok(spec) => {
            let id = spec.id.clone().unwrap_or_else(|| spec.label.clone());
            let kind = match spec.kind.as_deref() {
                Some("native") => broker::SurfaceKind::Native,
                Some("web") => broker::SurfaceKind::Web,
                _ => broker::SurfaceKind::Webview,
            };
            broker::GLOBAL_SURFACES.register(broker::SurfaceInfo {
                id,
                kind,
                platform: None,
                label: Some(spec.label.clone()),
            });
            c.windows.push(spec);
            *cfg = Some(c);
            ok_json(serde_json::json!({ "ok": true, "queued": true }))
        }
        Err(e) => err_str(e.to_string()),
    }
}

/// Shell helper: `stateSet(path, value)` — broker shared state (plan `path` contract).
pub fn state_set(args: &[Value]) -> Value {
    let Some(path) = arg_str(args, 0) else {
        return err_str("stateSet(path, value): path required");
    };
    let value = args
        .get(1)
        .and_then(|v| value_util::value_to_json(v))
        .unwrap_or(serde_json::Value::Null);
    match local_invoke::invoke_local(
        "state.set",
        serde_json::json!({ "path": path, "value": value, "source": "shell" }),
    ) {
        Ok(out) => ok_json(out),
        Err(e) => err_str(e),
    }
}

/// `brokerInvoke(cmd, args?)` — in-process invoke for shell / WK `onBridgeInvoke`.
pub fn broker_invoke(args: &[Value]) -> Value {
    local_invoke::broker_invoke(args)
}

/// Shell helper: `stateGet(path)`.
pub fn state_get(args: &[Value]) -> Value {
    let Some(path) = arg_str(args, 0) else {
        return err_str("stateGet(path): path required");
    };
    let (value, revision) = broker::GLOBAL_SHARED_STATE.get(&path);
    ok_json(serde_json::json!({
        "ok": true,
        "path": path,
        "value": value,
        "revision": revision,
    }))
}

/// `pendingNativeRoots()` — ids queued via `createSurface({ kind: "native", root })`.
pub fn pending_native_roots(_args: &[Value]) -> Value {
    let roots = broker::PENDING_NATIVE_ROOTS.lock();
    let ids: Vec<String> = roots.iter().map(|(id, _)| id.clone()).collect();
    ok_json(serde_json::json!({ "ok": true, "ids": ids, "count": ids.len() }))
}

pub fn list_windows(_args: &[Value]) -> Value {
    if let Some(app) = local_invoke::APP_HANDLE.lock().clone() {
        return ok_json(serde_json::json!(windows::list_labels(&app)));
    }
    ok_json(serde_json::json!([]))
}

pub fn focus_window(args: &[Value]) -> Value {
    let Some(app) = local_invoke::APP_HANDLE.lock().clone() else {
        return err_str("focusWindow: host is not running");
    };
    let label = arg_str(args, 0).map(|s| s.to_string());
    let result = match label.as_deref() {
        Some(l) => windows::focus(&app, l),
        None => windows::focus_last(&app),
    };
    match result {
        Ok(()) => ok_json(serde_json::json!({ "ok": true })),
        Err(e) => err_str(e),
    }
}

pub fn close_window(args: &[Value]) -> Value {
    let Some(app) = local_invoke::APP_HANDLE.lock().clone() else {
        return err_str("closeWindow: host is not running");
    };
    let Some(label) = arg_str(args, 0) else {
        return err_str("closeWindow(label): label required");
    };
    match windows::close_from_broker(&app, &label, windows::caller_label().as_deref()) {
        Ok(()) => ok_json(serde_json::json!({ "ok": true })),
        Err(e) => err_str(e),
    }
}

/// `run(config?)` — start Tauri event loop (blocks until app exits).
pub fn run(args: &[Value]) -> Value {
    let raw = arg_object_json(args, 0);
    let mut config = match &raw {
        Some(v) => match serde_json::from_value::<RunConfig>(v.clone()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("tishlang_desktop: run config parse failed ({e}); using defaults + fallbacks");
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

    let apple_attach = config
        .platform_attach
        .as_ref()
        .and_then(|p| p.apple.as_ref())
        .cloned();
    let pending_native = broker::PENDING_NATIVE_ROOTS.lock().len();
    // Pure-native: AppKit owns NSApplication.run (no Tauri). Only when the shell did not
    // request Tauri as outerHost — hybrid SC4 with plugins uses outerHost:true + webview(s).
    let want_tauri_host = apple_attach
        .as_ref()
        .map(|a| a.outer_host)
        .unwrap_or(false);
    let pure_native_apple = apple_attach.is_some()
        && config.windows.is_empty()
        && pending_native > 0
        && !want_tauri_host;

    if config.windows.is_empty() && !pure_native_apple {
        config.windows.push(state::WindowSpec {
            label: "main".into(),
            kind: Some("webview".into()),
            id: Some("main".into()),
            url: Some("http://localhost:5173/".into()),
            title: Some("Tish Desktop".into()),
            width: 960.0,
            height: 640.0,
            title_bar_style: "transparent".into(),
            hidden_title: false,
            decorations: true,
            layout: None,
            visible: true,
            transparent: false,
        });
    }

    // Keep PENDING_HANDLERS for brokerInvoke / WK bridge; AppState gets clones.
    let handlers = PENDING_HANDLERS.lock().clone();
    let tick_ms = config.tick_ms.unwrap_or(0);
    if tick_ms > 0 {
        eprintln!("tishlang_desktop: tick loop interval {tick_ms}ms");
    }
    if let Some(profile) = config.profile.as_deref() {
        eprintln!("tishlang_desktop: run profile={profile}");
    }
    let plugins = config.plugins.clone();
    let window_specs = config.windows.clone();

    if let Some(ref a) = apple_attach {
        let roots = std::mem::take(&mut *broker::PENDING_NATIVE_ROOTS.lock());
        let outer_host = if pure_native_apple {
            false
        } else {
            a.outer_host
        };
        let auto_run = if pure_native_apple {
            true
        } else {
            a.auto_run_event_loop
        };
        eprintln!(
            "tishlang_desktop: platformAttach.apple outerHost={} autoRunEventLoop={} pureNative={} attachingNativeRoots={}",
            outer_host,
            auto_run,
            pure_native_apple,
            roots.len()
        );
        let host = platform::current_host();
        let opts = serde_json::json!({
            "outerHost": outer_host,
            "autoRunEventLoop": auto_run,
            "autoShow": true,
            "title": "Hybrid",
        });
        for (id, root) in roots {
            match host.attach_native(&root, &opts) {
                Ok(v) => eprintln!("tishlang_desktop: attached native surface id={id} → {v}"),
                Err(e) => eprintln!("tishlang_desktop: attach native id={id} failed: {e}"),
            }
        }
        if pure_native_apple {
            // attach_native blocked in NSApplication.run when autoRunEventLoop=true.
            eprintln!("tishlang_desktop: pure native AppKit event loop exited");
            return Value::Null;
        }
    }

    let state = AppState::new(config);
    for (k, v) in handlers {
        state.register_handler(k, v);
    }

    let result = build_and_run(state, window_specs, plugins, tick_ms);
    match result {
        Ok(()) => Value::Null,
        Err(e) => {
            eprintln!("tishlang_desktop run error: {e}");
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
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let _ = windows::focus_last(app);
            // A second launch (`<binary> <path>` / Finder "Open With" on a packaged build) delivers
            // its args here — forward file/folder paths to the running instance (open-paths event).
            // Skip argv[0] (the binary) and any -flags; app_open tags each path with isDir.
            let paths: Vec<String> = argv
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with('-'))
                .cloned()
                .collect();
            app_open::emit_open_paths(app, &paths);
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
    if plugins.clipboard {
        builder = builder.plugin(tauri_plugin_clipboard_manager::init());
    }
    if plugins.global_shortcut {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }
    if plugins.window_state {
        builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
    }
    if plugins.os {
        builder = builder.plugin(tauri_plugin_os::init());
    }
    if plugins.store {
        builder = builder.plugin(tauri_plugin_store::Builder::default().build());
    }
    if plugins.autostart {
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    }
    if plugins.updater {
        builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    }

    // Host-defined resource protocols (RunConfig.resource_protocols): register each `<scheme>://`
    // generically — the scheme + trusted path substring come from config, nothing app-specific here.
    // A hosting app uses this to serve its own local files (e.g. extension assets) to sandboxed
    // webview iframes; without it those iframe requests fail ("Load failed").
    let resource_protocols = state.config.lock().resource_protocols.clone();
    for rp in resource_protocols {
        let path_contains = rp.path_contains;
        builder = builder.register_uri_scheme_protocol(rp.scheme, move |_app, request| {
            webview_resource::serve(request.uri().path(), &path_contains)
        });
    }

    builder
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::desktop_protocol,
            commands::desktop_invoke,
            commands::desktop_emit_tick,
        ])
        .on_window_event(|window, event| match event {
            WindowEvent::ThemeChanged(theme) => {
                let theme = format!("{theme:?}").to_lowercase();
                emit_from_window_event(
                    window.app_handle(),
                    "theme:changed",
                    serde_json::json!({ "theme": theme, "label": window.label() }),
                );
            }
            WindowEvent::Focused(true) => {
                windows::note_focused(window.label());
            }
            WindowEvent::Destroyed => {
                let label = window.label().to_string();
                windows::on_destroyed(&label);
                emit_from_window_event(
                    window.app_handle(),
                    "window-close",
                    serde_json::json!({ "label": label }),
                );
                emit_from_window_event(
                    window.app_handle(),
                    "window-destroyed",
                    serde_json::json!({ "label": label }),
                );
            }
            _ => {}
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            local_invoke::set_app_handle(handle.clone());
            // Optional app-branding icon (RunConfig.icon) for the tray + macOS dock.
            let icon_path = app.state::<AppState>().config.lock().icon.clone();
            // Optional custom deep-link scheme (RunConfig.deep_link_scheme), registered below.
            let deep_link_scheme = app.state::<AppState>().config.lock().deep_link_scheme.clone();

            // macOS traffic-light chrome: install the relayout observers (idempotent, no-op elsewhere).
            #[cfg(all(feature = "platform-apple", target_os = "macos"))]
            {
                traffic_lights::init(&handle);
                traffic_light_tint::init(&handle);
                if let Some(ref p) = icon_path {
                    app_icon::set_dock_icon_scheduled(&handle, p.clone());
                }
            }

            if plugins.deep_link {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Register the app's custom scheme at runtime so the RUNNING instance handles
                // `<scheme>://…` (dev + already-running). Packaged default-handler status still comes
                // from the bundle Info.plist. Best-effort — a failure just means no deep links in dev.
                if let Some(ref scheme) = deep_link_scheme {
                    let _ = app.deep_link().register(scheme.as_str());
                }
                let dl = handle.clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let s = url.to_string();
                        if commands::secrets_auth::is_oauth_scheme_callback(&s) {
                            let _ = commands::secrets_auth::complete_oauth_callback(&dl, &s);
                        }
                        let _ = dl.emit("deep-link", serde_json::json!({ "url": s }));
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
                            let _ = windows::focus_last(&h);
                        }
                    }
                });
            }

            if plugins.tray {
                let show_i = MenuItem::with_id(app, "tray:show", "Show", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;
                let tray_menu = Menu::with_items(app, &[&show_i, &quit_i])?;
                let mut tray_builder = TrayIconBuilder::new().menu(&tray_menu).tooltip("Tish Desktop");
                // Brand the menu-bar icon from RunConfig.icon (else the tray shows blank/default).
                if let Some(ref p) = icon_path {
                    if let Ok(img) = tauri::image::Image::from_path(p) {
                        tray_builder = tray_builder.icon(img);
                    }
                }
                let tray = tray_builder
                    .on_menu_event(move |app, event| {
                        let id = event.id().0.clone();
                        // Generic: emit `tray:action` { id } and let the hosting app map ids to its
                        // own actions (a dynamic menu set via tray.setMenu). Two conventional ids get
                        // built-in behavior so the default tray works with no host wiring.
                        let _ = app.emit("tray:action", serde_json::json!({ "id": id }));
                        if id == "tray:quit" {
                            app.exit(0);
                        } else if id == "tray:show" {
                            let _ = windows::focus_last(app);
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
                            let _ = windows::focus_last(app);
                        }
                    })
                    .build(app)?;
                *app.state::<AppState>().tray.lock() = Some(tray);
            }

            for spec in &window_specs {
                windows::create_from_spec(&handle, spec)?;
            }

            if std::env::var("TISH_WINDOW_LIFECYCLE_TEST").as_deref() == Ok("1") {
                windows::spawn_lifecycle_selftest(handle.clone());
            }

            if tick_ms > 0 {
                commands::spawn_tick_loop(handle.clone(), tick_ms);
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| e.to_string())?
        .run(|app_handle, event| {
            // Tauri sets ITS OWN embedded icon on Ready in dev builds (see
            // tauri::app::on_event_loop_event), which is after setup() and, on a slow dev boot,
            // after the last scheduled reassert — so the host icon showed first and was then
            // replaced by Tauri's. Reassert here: same event, immediately after, no race.
            // No-op in a packaged app, where the bundle already supplies the right icon.
            #[cfg(all(feature = "platform-apple", target_os = "macos"))]
            if matches!(event, tauri::RunEvent::Ready) {
                let icon = app_handle
                    .try_state::<AppState>()
                    .and_then(|s| s.config.lock().icon.clone());
                if let Some(path) = icon {
                    app_icon::reassert_on_ready(&path);
                }
            }
            // macOS: files/folders dropped on the running dock icon (or Finder "Open With") arrive
            // as RunEvent::Opened — forward them to the webview as an `open-paths` event.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &event {
                let paths: Vec<String> = urls
                    .iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                app_open::emit_open_paths(app_handle, &paths);
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (app_handle, event);
            }
        });
    Ok(())
}

/// Export module object for `tish:` npm-style packages (optional).
pub fn tishlang_desktop_object() -> Value {
    use tishlang_core::ObjectMap;
    let mut m = ObjectMap::default();
    m.insert(Arc::from("run"), Value::native(run));
    m.insert(Arc::from("handle"), Value::native(handle));
    m.insert(Arc::from("emit"), Value::native(emit));
    m.insert(Arc::from("useExtensions"), Value::native(use_extensions));
    m.insert(
        Arc::from("registerRustExtension"),
        Value::native(register_rust_extension),
    );
    m.insert(Arc::from("createWindow"), Value::native(create_window));
    m.insert(Arc::from("createSurface"), Value::native(create_surface));
    m.insert(Arc::from("brokerInvoke"), Value::native(broker_invoke));
    m.insert(Arc::from("stateSet"), Value::native(state_set));
    m.insert(Arc::from("stateGet"), Value::native(state_get));
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
