//! Thin launcher: prefers the `@tishlang/tish-desktop` CLI (`mode`) on PATH.
use std::env;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Ok(custom) = env::var("TISH_DESKTOP_CLI").or_else(|_| env::var("TISH_MODE_CLI")) {
        let status = Command::new(&custom).args(&args).status();
        match status {
            Ok(s) => exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("mode: failed to run CLI override={custom}: {e}");
                exit(1);
            }
        }
    }

    // Prefer a native CLI named mode-cli / mode.js to avoid recursion with this bin.
    for candidate in ["mode-cli", "mode.js", "mode"] {
        // Skip self if somehow shadowed — `mode` may be this binary; prefer -cli / .js first.
        if candidate == "mode" {
            continue;
        }
        if let Ok(s) = Command::new(candidate).args(&args).status() {
            exit(s.code().unwrap_or(1));
        }
    }

    // npx fallback (installs/runs the published package; bin is `mode`)
    let mut npx = Command::new("npx");
    npx.args(["--yes", "@tishlang/tish-desktop"]).args(&args);
    match npx.status() {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(_) => {
            eprintln!(
                "mode: install the CLI:\n  npm i -g @tishlang/tish-desktop\n  # or from the repo: node cli/bin/mode.js\nSet TISH_MODE_CLI (or TISH_DESKTOP_CLI) to a full path if needed."
            );
            exit(1);
        }
    }
}
