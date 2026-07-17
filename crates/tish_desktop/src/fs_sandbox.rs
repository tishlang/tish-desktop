//! Allowlisted filesystem access for webview-facing broker commands.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

pub const MAX_READ_BYTES: u64 = 256 * 1024;
pub const MAX_LIST_ENTRIES: usize = 2000;

#[derive(Debug, Clone, Serialize)]
pub struct DirEntryDto {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub mtime: u64,
}

pub fn resolve_under_root(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let root = std::fs::canonicalize(root).map_err(|e| format!("canonicalize root: {e}"))?;
    let candidate = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        root.join(requested)
    };
    let canon = std::fs::canonicalize(&candidate).map_err(|e| format!("canonicalize path: {e}"))?;
    if !canon.starts_with(&root) {
        return Err("path escapes allowlisted root".into());
    }
    Ok(canon)
}

pub fn list_dir(root: &Path, path: &str) -> Result<Vec<DirEntryDto>, String> {
    let dir = resolve_under_root(root, path)?;
    if !dir.is_dir() {
        return Err("not a directory".into());
    }
    let mut entries = Vec::new();
    let rd = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for ent in rd.flatten() {
        if entries.len() >= MAX_LIST_ENTRIES {
            break;
        }
        let meta = match ent.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = ent.file_name().to_string_lossy().to_string();
        let full = ent.path();
        let rel = full
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| full.to_string_lossy().to_string());
        let kind = if meta.is_dir() {
            "dir"
        } else if meta.is_file() {
            "file"
        } else {
            "other"
        };
        entries.push(DirEntryDto {
            name,
            path: rel,
            kind: kind.into(),
            size: meta.len(),
            mtime: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        });
    }
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

pub fn read_text(root: &Path, path: &str) -> Result<String, String> {
    let file = resolve_under_root(root, path)?;
    if !file.is_file() {
        return Err("not a file".into());
    }
    let meta = std::fs::metadata(&file).map_err(|e| e.to_string())?;
    if meta.len() > MAX_READ_BYTES {
        return Err(format!(
            "file exceeds {} byte read cap",
            MAX_READ_BYTES
        ));
    }
    std::fs::read_to_string(&file).map_err(|e| e.to_string())
}

pub fn stat_path(root: &Path, path: &str) -> Result<DirEntryDto, String> {
    let p = resolve_under_root(root, path)?;
    let meta = std::fs::metadata(&p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = if meta.is_dir() {
        "dir"
    } else if meta.is_file() {
        "file"
    } else {
        "other"
    };
    Ok(DirEntryDto {
        name,
        path: p.to_string_lossy().to_string(),
        kind: kind.into(),
        size: meta.len(),
        mtime: meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0),
    })
}

pub struct FsWatcher {
    _watcher: Mutex<Option<RecommendedWatcher>>,
}

impl Default for FsWatcher {
    fn default() -> Self {
        Self {
            _watcher: Mutex::new(None),
        }
    }
}

pub fn start_watch(app: AppHandle, state: &AppState, path: &str) -> Result<(), String> {
    let root = state.fs_root.lock().clone();
    let Some(root) = root else {
        return Err("fs root not configured".into());
    };
    let watch_path = resolve_under_root(&root, path)?;
    let app2 = app.clone();
    let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        if let Ok(event) = res {
            let kind = format!("{:?}", event.kind);
            let paths: Vec<String> = event
                .paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let payload = serde_json::json!({
                "kind": kind,
                "paths": paths,
                "ts": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            });
            let _ = app2.emit("fs:changed", payload);
        }
    })
    .map_err(|e| e.to_string())?;

    watcher
        .watch(&watch_path, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    *state.fs_watcher._watcher.lock() = Some(watcher);
    Ok(())
}

pub fn stop_watch(state: &AppState) {
    *state.fs_watcher._watcher.lock() = None;
}
