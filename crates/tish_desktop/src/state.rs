use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tishlang_core::{value_call, NativeFn, Value};

use crate::broker::{SharedState, SurfaceInfo, SurfaceKind, SurfaceRegistry};
use crate::fs_sandbox::FsWatcher;

pub const PROTOCOL_VERSION: &str = "desktop/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSpec {
    pub label: String,
    /// Surface kind: `webview` (default) or `native` (apple attach when enabled).
    #[serde(default)]
    pub kind: Option<String>,
    /// Optional stable surface id (defaults to `label`).
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_width")]
    pub width: f64,
    #[serde(default = "default_height")]
    pub height: f64,
    #[serde(default = "default_title_bar_style")]
    pub title_bar_style: String,
    #[serde(default)]
    pub hidden_title: bool,
    #[serde(default = "default_true")]
    pub decorations: bool,
    /// Native layout hint: `content` | `sidebar` (apple).
    #[serde(default)]
    pub layout: Option<String>,
}

fn default_width() -> f64 {
    960.0
}
fn default_height() -> f64 {
    640.0
}
fn default_title_bar_style() -> String {
    "transparent".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellAllowEntry {
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub args_prefix: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    #[serde(default)]
    pub token_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFlags {
    #[serde(default = "default_true")]
    pub dialog: bool,
    #[serde(default = "default_true")]
    pub tray: bool,
    #[serde(default = "default_true")]
    pub menu: bool,
    #[serde(default = "default_true")]
    pub deep_link: bool,
    #[serde(default = "default_true")]
    pub opener: bool,
    #[serde(default = "default_true")]
    pub single_instance: bool,
    #[serde(default = "default_true")]
    pub notification: bool,
    #[serde(default = "default_true")]
    pub clipboard: bool,
    #[serde(default = "default_true")]
    pub global_shortcut: bool,
    #[serde(default = "default_true")]
    pub window_state: bool,
    #[serde(default = "default_true")]
    pub os: bool,
    #[serde(default = "default_true")]
    pub store: bool,
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default)]
    pub updater: bool,
    #[serde(default = "default_true")]
    pub process: bool,
    #[serde(default)]
    pub shell: bool,
    #[serde(default)]
    pub http: bool,
    #[serde(default = "default_true")]
    pub auth: bool,
}

impl Default for PluginFlags {
    fn default() -> Self {
        Self {
            dialog: true,
            tray: true,
            menu: true,
            deep_link: true,
            opener: true,
            single_instance: true,
            notification: true,
            clipboard: true,
            global_shortcut: true,
            window_state: true,
            os: true,
            store: true,
            autostart: true,
            updater: false,
            process: true,
            shell: false,
            http: false,
            auth: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleAttachConfig {
    /// Hybrid default: false — outer host (Tauri) owns NSApplication.run.
    #[serde(default = "default_false")]
    pub auto_run_event_loop: bool,
    /// Hybrid default: true — skip apple menu/timer/activation clobber.
    #[serde(default = "default_true")]
    pub outer_host: bool,
}

impl Default for AppleAttachConfig {
    fn default() -> Self {
        Self {
            auto_run_event_loop: false,
            outer_host: true,
        }
    }
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAttachConfig {
    #[serde(default)]
    pub apple: Option<AppleAttachConfig>,
}

/// One host-defined `<scheme>://localhost/<abs-path>` resource protocol (see `RunConfig::resource_protocols`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProtocol {
    /// URI scheme name, e.g. "app-assets" — served as `<scheme>://localhost/<abs-path>`.
    pub scheme: String,
    /// Trusted absolute-path substring (e.g. "/my-app/assets/"); only files whose decoded path
    /// contains it (and have no `..`) are served. This is the entire allowlist for the scheme.
    pub path_contains: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    /// Runtime profile: `desktop` | `ios` | `web` (informational + attach defaults).
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub platform_attach: Option<PlatformAttachConfig>,
    #[serde(default)]
    pub windows: Vec<WindowSpec>,
    #[serde(default)]
    pub plugins: PluginFlags,
    #[serde(default)]
    pub fs_root: Option<String>,
    /// Path to a PNG the app should use for its tray icon and (macOS) dock icon, overriding the
    /// tish-desktop default. Lets a hosting app brand itself without rebuilding the
    /// runtime's bundled icons.
    #[serde(default)]
    pub icon: Option<String>,
    /// A custom URI scheme the hosting app handles as a deep link, registered at runtime so
    /// `<scheme>://…` opens route to `on_open_url` → the `deep-link` event. Packaged builds also
    /// need the scheme in their OWN bundle Info.plist; this covers dev + the running-instance case.
    #[serde(default)]
    pub deep_link_scheme: Option<String>,
    /// Custom `<scheme>://localhost/<abs-path>` resource protocols the hosting app registers to serve
    /// its own local files to sandboxed webview iframes (e.g. an extension's assets). Each is
    /// path-restricted to files whose absolute path contains `path_contains` (plus a `..` traversal
    /// guard). Nothing app-specific lives in the runtime — the scheme + trusted segment come from here.
    #[serde(default)]
    pub resource_protocols: Vec<ResourceProtocol>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_opt_u64_from_number")]
    pub tick_ms: Option<u64>,
    #[serde(default)]
    pub shell_allow: Vec<ShellAllowEntry>,
    #[serde(default)]
    pub http_allow: Vec<String>,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

fn deserialize_opt_u64_from_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match v {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .or_else(|| n.as_i64().map(|i| i.max(0) as u64))
            .or_else(|| n.as_f64().map(|f| f.max(0.0) as u64)),
        Some(_) => None,
    })
}

pub fn permissions_from_plugins(p: &PluginFlags) -> Vec<String> {
    let mut perms = vec!["fs:scoped".into()];
    if p.dialog {
        perms.push("dialog".into());
    }
    if p.tray {
        perms.push("tray".into());
    }
    if p.menu {
        perms.push("menu".into());
    }
    if p.deep_link {
        perms.push("deep-link".into());
    }
    if p.opener {
        perms.push("opener".into());
    }
    if p.notification {
        perms.push("notification".into());
    }
    if p.clipboard {
        perms.push("clipboard".into());
    }
    if p.global_shortcut {
        perms.push("global-shortcut".into());
    }
    if p.window_state {
        perms.push("window-state".into());
    }
    if p.os {
        perms.push("os".into());
    }
    if p.store {
        perms.push("store".into());
    }
    if p.autostart {
        perms.push("autostart".into());
    }
    if p.updater {
        perms.push("updater".into());
    }
    if p.process {
        perms.push("process".into());
    }
    if p.shell {
        perms.push("shell".into());
    }
    if p.http {
        perms.push("http".into());
    }
    if p.auth {
        perms.push("auth".into());
        perms.push("secrets".into());
    }
    perms
}

#[cfg(test)]
mod tests {
    use super::{PluginFlags, RunConfig};

    #[test]
    fn parses_tick_ms_camel_case_from_f64() {
        let v = serde_json::json!({
            "tickMs": 1000.0,
            "plugins": {
                "dialog": true,
                "tray": true,
                "menu": true,
                "deepLink": true,
                "opener": true,
                "singleInstance": true
            }
        });
        let cfg: RunConfig = serde_json::from_value(v).expect("parse");
        assert_eq!(cfg.tick_ms, Some(1000));
        assert!(cfg.plugins.notification);
        assert!(PluginFlags::default().notification);
        assert!(PluginFlags::default().opener);
    }
}

/// In-flight OAuth PKCE session awaiting a redirect callback (loopback or scheme).
#[derive(Debug, Clone)]
pub struct PendingOAuth {
    pub verifier: String,
    pub csrf_state: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
    /// OIDC nonce when openid / explicit oidc login; checked against `id_token` payload.
    pub nonce: Option<String>,
    pub revocation_endpoint: Option<String>,
}

/// Persisted (in-memory) session metadata after a successful login.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub client_id: String,
    pub revocation_endpoint: Option<String>,
}

pub struct AppState {
    pub fs_root: Mutex<Option<PathBuf>>,
    pub fs_watcher: FsWatcher,
    pub handlers: Mutex<HashMap<String, NativeFn>>,
    pub permissions: Mutex<Vec<String>>,
    pub extensions: Mutex<Vec<String>>,
    pub config: Mutex<RunConfig>,
    pub tray: Mutex<Option<tauri::tray::TrayIcon>>,
    pub sleep_blocks: Mutex<u32>,
    pub shortcuts: Mutex<HashMap<String, String>>,
    /// Held while `sleep_blocks > 0`; dropping it re-allows the system to sleep.
    pub keepawake_guard: Mutex<Option<keepawake::KeepAwake>>,
    /// In-memory cache of the last-issued OAuth access token: `(token, expires_at_unix_secs)`.
    pub auth_cache: Mutex<Option<(String, u64)>>,
    pub pending_oauth: Mutex<Option<PendingOAuth>>,
    pub auth_session: Mutex<Option<AuthSession>>,
    /// Broker shared microfrontend state (`state.*`).
    pub shared_state: Arc<SharedState>,
    pub surfaces: Arc<SurfaceRegistry>,
}

impl AppState {
    pub fn new(config: RunConfig) -> Self {
        let fs_root = config.fs_root.as_ref().map(PathBuf::from);
        let permissions = permissions_from_plugins(&config.plugins);
        let shared_state = Arc::clone(&crate::broker::GLOBAL_SHARED_STATE);
        let surfaces = Arc::clone(&crate::broker::GLOBAL_SURFACES);
        for w in &config.windows {
            let id = w.id.clone().unwrap_or_else(|| w.label.clone());
            let kind = match w.kind.as_deref() {
                Some("native") => SurfaceKind::Native,
                Some("web") => SurfaceKind::Web,
                _ => SurfaceKind::Webview,
            };
            surfaces.register(SurfaceInfo {
                id,
                kind,
                platform: None,
                label: Some(w.label.clone()),
            });
        }
        Self {
            fs_root: Mutex::new(fs_root),
            fs_watcher: FsWatcher::default(),
            handlers: Mutex::new(HashMap::new()),
            permissions: Mutex::new(permissions),
            extensions: Mutex::new(config.extensions.clone()),
            config: Mutex::new(config),
            tray: Mutex::new(None),
            sleep_blocks: Mutex::new(0),
            shortcuts: Mutex::new(HashMap::new()),
            keepawake_guard: Mutex::new(None),
            auth_cache: Mutex::new(None),
            pending_oauth: Mutex::new(None),
            auth_session: Mutex::new(None),
            shared_state,
            surfaces,
        }
    }

    pub fn register_handler(&self, name: String, f: NativeFn) {
        self.handlers.lock().insert(name, f);
    }

    pub fn call_handler(
        &self,
        name: &str,
        args_json: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Clone the handler Arc OUT of the map and release the lock BEFORE running it. Handlers
        // legitimately call brokerInvoke re-entrantly (e.g. appChrome's set_traffic_light_inset →
        // window.trafficLightInset), which re-enters dispatch → handlers.lock(). Holding the lock
        // across value_call would deadlock — parking_lot::Mutex is NOT re-entrant — and, because the
        // wedged thread never releases it, every later desktop_invoke blocks at its is_handler check,
        // freezing the whole backend (no themes / palette / extensions on boot).
        let f = {
            let handlers = self.handlers.lock();
            match handlers.get(name) {
                Some(f) => Arc::clone(f),
                None => return Err(format!("unknown command: {name}")),
            }
        };
        let arg = crate::value_util::json_to_value(&args_json);
        let result = value_call(&Value::Function(f), &[arg]);
        crate::value_util::value_to_json(&result)
            .ok_or_else(|| "handler returned non-JSON value".into())
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.lock().iter().any(|p| p == perm)
    }
}

pub static PENDING_CONFIG: once_cell::sync::Lazy<Mutex<Option<RunConfig>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

pub static PENDING_HANDLERS: once_cell::sync::Lazy<Mutex<HashMap<String, NativeFn>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
