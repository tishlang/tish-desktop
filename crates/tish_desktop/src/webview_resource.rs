//! Generic `<scheme>://localhost/<url-encoded-abs-path>` resource protocol.
//!
//! Lets a hosting app serve its OWN local files (css/js/images/fonts) to a sandboxed webview iframe
//! that needs to load them via a custom scheme (e.g. an extension host's `asWebviewUri()`). Nothing
//! app-specific lives here: the caller supplies the scheme and a single trusted absolute-path
//! substring (`allowed_segment`) via `RunConfig.resource_protocols`; only files whose decoded path
//! contains that substring — and have no `..` traversal — are served. This is a host primitive (a
//! URI-scheme hook must be registered on the Tauri builder), so it lives in the runtime; the policy
//! (which scheme, which trusted segment) comes entirely from the host's config.

use tauri::http::Response;

/// Trusted iff the path has no `..` traversal AND contains the host-supplied `allowed_segment`
/// (e.g. "/.dune/extensions/"). Root-independent by design — the segment check is the whole guard.
fn resource_allowed(path: &str, allowed_segment: &str) -> bool {
    let norm = path.replace('\\', "/");
    if norm.contains("..") {
        return false;
    }
    norm.contains(allowed_segment)
}

/// Content-type for the file: text/binary types a webview needs (with the charset params these
/// responses require), falling back to the common image/font table and then octet-stream.
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

/// Percent-decode a URL path (`%20` → space, etc.). Malformed escapes are left as-is.
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

/// Serve one `<scheme>://localhost/<path>` request. `raw_path` is the URI path component (still
/// percent-encoded); `allowed_segment` is the host-configured trusted substring. Registered per
/// `RunConfig.resource_protocols` entry via `register_uri_scheme_protocol(scheme, …)`.
pub fn serve(raw_path: &str, allowed_segment: &str) -> Response<Vec<u8>> {
    let decoded = percent_decode(raw_path);
    if !resource_allowed(&decoded, allowed_segment) {
        eprintln!("[resource-protocol] forbidden: {decoded}");
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
            eprintln!("[resource-protocol] not found: {decoded} ({e})");
            err(404, "not found")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SEG: &str = "/.dune/extensions/";

    #[test]
    fn rejects_traversal_and_out_of_segment_paths() {
        assert!(!resource_allowed("/Users/x/.dune/extensions/../secret", SEG));
        assert!(!resource_allowed("/etc/passwd", SEG));
        assert!(!resource_allowed("/Users/x/project/src/main.rs", SEG));
    }

    #[test]
    fn allows_files_under_the_trusted_segment() {
        assert!(resource_allowed(
            "/Users/x/.dune/extensions/pub.ext-1.0.0/media/main.css",
            SEG
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
