//! `dune-webview://localhost/<url-encoded-abs-path>` resource protocol.
//!
//! Serves an extension's local webview files (css/js/images/fonts) to its sandboxed iframe. The
//! extension host's `webview.asWebviewUri()` mints these URLs and `webview.cspSource` is
//! `dune-webview://localhost`, so the extension's own CSP allows them. Path-restricted to
//! dune-managed extension dirs (`…/.dune/extensions/…`); any `..` is rejected.
//!
//! Ported verbatim (behaviour-for-behaviour) from dune-ide's `src-tauri` — `serve_webview_resource`
//! + `ext_paths::webview_resource_allowed` + `webview_resource_content_type`. Registering the scheme
//! is what lets extension webview panels (custom editors, view containers) load their assets; without
//! it the iframe requests fail (`Load failed`) and the panel renders blank. This is a host primitive
//! (a URI-scheme hook), so it lives in the runtime rather than in shell tish.

use tauri::http::Response;

/// The only trusted path segment: dune installs every extension under a `.dune/extensions` root.
/// Mirrors `DUNE_EXTENSIONS_PATH_SEGMENT` in the shared `extPaths` module.
const DUNE_EXTENSIONS_SEGMENT: &str = "/.dune/extensions/";

/// Only dune-managed extension installs are trusted; reject any `..` traversal. Root-independent by
/// design (matches `ext_paths::webview_resource_allowed`) — the segment check is the whole guard.
fn resource_allowed(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    if norm.contains("..") {
        return false;
    }
    norm.contains(DUNE_EXTENSIONS_SEGMENT)
}

/// Content-type for the file, matching `webview_resource_content_type`: text/binary types the webview
/// needs (with the charset params these responses require), falling back to the common image/font
/// table and then `application/octet-stream`.
fn content_type(path: &str) -> &'static str {
    let p = path.to_lowercase();
    if p.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if p.ends_with(".js") || p.ends_with(".mjs") || p.ends_with(".cjs") {
        "text/javascript; charset=utf-8"
    } else if p.ends_with(".html") || p.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if p.ends_with(".json") || p.ends_with(".map") {
        "application/json; charset=utf-8"
    } else if p.ends_with(".wasm") {
        "application/wasm"
    } else if p.ends_with(".svg") {
        "image/svg+xml"
    } else if p.ends_with(".png") {
        "image/png"
    } else if p.ends_with(".jpg") || p.ends_with(".jpeg") {
        "image/jpeg"
    } else if p.ends_with(".gif") {
        "image/gif"
    } else if p.ends_with(".webp") {
        "image/webp"
    } else if p.ends_with(".ico") {
        "image/x-icon"
    } else if p.ends_with(".woff2") {
        "font/woff2"
    } else if p.ends_with(".woff") {
        "font/woff"
    } else if p.ends_with(".ttf") {
        "font/ttf"
    } else if p.ends_with(".otf") {
        "font/otf"
    } else {
        "application/octet-stream"
    }
}

/// Percent-decode a URL path (`%20` → space, etc.). Malformed escapes are left as-is, mirroring the
/// lossy `urlencoding::decode(...).unwrap_or(raw)` the Rust backend used.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn err(code: u16, msg: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .header("Access-Control-Allow-Origin", "*")
        .body(msg.as_bytes().to_vec())
        .unwrap()
}

/// Serve one `dune-webview://localhost/<path>` request. `raw_path` is the URI path component
/// (still percent-encoded). Registered via `register_uri_scheme_protocol("dune-webview", …)`.
pub fn serve(raw_path: &str) -> Response<Vec<u8>> {
    let decoded = percent_decode(raw_path);
    if !resource_allowed(&decoded) {
        eprintln!("[dune-webview] forbidden: {decoded}");
        return err(403, "forbidden");
    }
    match std::fs::read(&decoded) {
        Ok(bytes) => Response::builder()
            .status(200)
            .header("Content-Type", content_type(&decoded))
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "no-cache")
            .body(bytes)
            .unwrap(),
        Err(e) => {
            eprintln!("[dune-webview] not found: {decoded} ({e})");
            err(404, "not found")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_non_extension_paths() {
        assert!(!resource_allowed("/Users/x/.dune/extensions/../secret"));
        assert!(!resource_allowed("/etc/passwd"));
        assert!(!resource_allowed("/Users/x/project/src/main.rs"));
    }

    #[test]
    fn allows_dune_managed_extension_files() {
        assert!(resource_allowed(
            "/Users/x/.dune/extensions/pub.ext-1.0.0/media/main.css"
        ));
    }

    #[test]
    fn percent_decodes_spaces() {
        assert_eq!(percent_decode("/a%20b/c.js"), "/a b/c.js");
        assert_eq!(percent_decode("/plain/path.css"), "/plain/path.css");
    }

    #[test]
    fn content_types_cover_webview_needs() {
        assert_eq!(content_type("/x/a.css"), "text/css; charset=utf-8");
        assert_eq!(content_type("/x/a.js"), "text/javascript; charset=utf-8");
        assert_eq!(content_type("/x/a.PNG"), "image/png");
        assert_eq!(content_type("/x/a.unknown"), "application/octet-stream");
    }
}
