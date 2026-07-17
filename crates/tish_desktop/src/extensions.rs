//! Extension registry for Tish and Rust desktop extensions.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub ui: Option<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

pub struct ExtensionRegistry {
    manifests: Mutex<Vec<ExtensionManifest>>,
    rust_modules: Mutex<Vec<String>>,
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self {
            manifests: Mutex::new(Vec::new()),
            rust_modules: Mutex::new(Vec::new()),
        }
    }
}

impl ExtensionRegistry {
    pub fn register_tish(&self, manifest: ExtensionManifest) -> Result<(), String> {
        let known = ["dialog", "tray", "menu", "deep-link", "fs:scoped", "opener"];
        for perm in &manifest.permissions {
            if !known.contains(&perm.as_str()) {
                return Err(format!("unknown permission requested: {perm}"));
            }
        }
        {
            let existing = self.manifests.lock();
            for m in existing.iter() {
                if m.id == manifest.id {
                    return Err(format!("extension already registered: {}", manifest.id));
                }
                for cmd in &manifest.commands {
                    let namespaced = format!("{}.{}", manifest.id, cmd);
                    for other in &m.commands {
                        let other_ns = format!("{}.{}", m.id, other);
                        if namespaced == other_ns {
                            return Err(format!("command collision: {namespaced}"));
                        }
                    }
                }
            }
        }
        self.manifests.lock().push(manifest);
        Ok(())
    }

    pub fn register_rust_module(&self, name: impl Into<String>) {
        self.rust_modules.lock().push(name.into());
    }

    pub fn list(&self) -> serde_json::Value {
        serde_json::json!({
            "protocol": crate::state::PROTOCOL_VERSION,
            "tish": *self.manifests.lock(),
            "rust": *self.rust_modules.lock(),
        })
    }
}

pub static REGISTRY: once_cell::sync::Lazy<ExtensionRegistry> =
    once_cell::sync::Lazy::new(ExtensionRegistry::default);
