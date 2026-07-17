//! Thin launcher: prefers the Tish `@tish-desktop/cli` binary on PATH.
use std::env;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Ok(custom) = env::var("TISH_DESKTOP_CLI") {
        let status = Command::new(&custom).args(&args).status();
        match status {
            Ok(s) => exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("tish-desktop: failed to run TISH_DESKTOP_CLI={custom}: {e}");
                exit(1);
            }
        }
    }

    // Prefer a native CLI named tish-desktop-cli to avoid recursion with this bin.
    for candidate in ["tish-desktop-cli", "tish-desktop.js"] {
        if let Ok(s) = Command::new(candidate).args(&args).status() {
            exit(s.code().unwrap_or(1));
        }
    }

    // npx fallback
    let mut npx = Command::new("npx");
    npx.args(["--yes", "@tish-desktop/cli"]).args(&args);
    match npx.status() {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(_) => {
            eprintln!(
                "tish-desktop: install the Tish CLI:\n  npm i -g @tish-desktop/cli\n  # or from the repo: node cli/bin/tish-desktop.js\nSet TISH_DESKTOP_CLI to a full path if needed."
            );
            exit(1);
        }
    }
}
