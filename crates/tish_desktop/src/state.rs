use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tishlang_core::{value_call, NativeFn, Value};

use crate::fs_sandbox::FsWatcher;

pub const PROTOCOL_VERSION: &str = "desktop/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSpec {
    pub label: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default = "default_width")]
    pub width: f64,
    #[serde(default = "default_height")]
    pub height: f64,
    /// macOS title bar: `visible` | `transparent` | `overlay`. Default `transparent`
    /// so the bar blends with the window background (matches prior chrome).
    #[serde(default = "default_title_bar_style")]
    pub title_bar_style: String,
    /// Hide the native title text (useful with `overlay` + a custom toolbar).
    #[serde(default)]
    pub hidden_title: bool,
    /// Native window decorations (traffic lights / borders). Default true.
    #[serde(default = "default_true")]
    pub decorations: bool,
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
}

impl Default for PluginFlags {
    /// All plugins on — must match serde defaults. `#[derive(Default)]` would
    /// zero bools to false and, via pending `createWindow` + AND-merge, disable
    /// every plugin (including notification → `state() called before manage()`).
    fn default() -> Self {
        Self {
            dialog: true,
            tray: true,
            menu: true,
            deep_link: true,
            opener: true,
            single_instance: true,
            notification: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    #[serde(default)]
    pub windows: Vec<WindowSpec>,
    #[serde(default)]
    pub plugins: PluginFlags,
    #[serde(default)]
    pub fs_root: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Tish numbers are f64 → JSON floats; accept both integer and float forms.
    #[serde(default, deserialize_with = "deserialize_opt_u64_from_number")]
    pub tick_ms: Option<u64>,
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
        // Omitted keys use serde default_true — and Default must match.
        assert!(cfg.plugins.notification);
        assert!(PluginFlags::default().notification);
    }
}

pub struct AppState {
    pub fs_root: Mutex<Option<PathBuf>>,
    pub fs_watcher: FsWatcher,
    pub handlers: Mutex<HashMap<String, NativeFn>>,
    pub permissions: Mutex<Vec<String>>,
    pub extensions: Mutex<Vec<String>>,
    pub config: Mutex<RunConfig>,
    /// Status-item / tray icon (set during setup when tray plugin enabled).
    pub tray: Mutex<Option<tauri::tray::TrayIcon>>,
}

impl AppState {
    pub fn new(config: RunConfig) -> Self {
        let fs_root = config
            .fs_root
            .as_ref()
            .map(PathBuf::from);
        Self {
            fs_root: Mutex::new(fs_root),
            fs_watcher: FsWatcher::default(),
            handlers: Mutex::new(HashMap::new()),
            permissions: Mutex::new(vec![
                "dialog".into(),
                "tray".into(),
                "menu".into(),
                "deep-link".into(),
                "notification".into(),
                "fs:scoped".into(),
            ]),
            extensions: Mutex::new(config.extensions.clone()),
            config: Mutex::new(config),
            tray: Mutex::new(None),
        }
    }

    pub fn register_handler(&self, name: String, f: NativeFn) {
        self.handlers.lock().insert(name, f);
    }

    pub fn call_handler(&self, name: &str, args_json: serde_json::Value) -> Result<serde_json::Value, String> {
        let handlers = self.handlers.lock();
        let Some(f) = handlers.get(name) else {
            return Err(format!("unknown command: {name}"));
        };
        let arg = crate::value_util::json_to_value(&args_json);
        let result = value_call(&Value::Function(Arc::clone(f)), &[arg]);
        crate::value_util::value_to_json(&result).ok_or_else(|| "handler returned non-JSON value".into())
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.lock().iter().any(|p| p == perm)
    }
}

/// Pending run config set from Tish before `run()` blocks.
pub static PENDING_CONFIG: once_cell::sync::Lazy<Mutex<Option<RunConfig>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

pub static PENDING_HANDLERS: once_cell::sync::Lazy<Mutex<HashMap<String, NativeFn>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
