use std::path::{Path, PathBuf};

/// Makes sure `frontendDist` exists before Tauri's macros look for it.
///
/// `tauri::generate_context!()` checks that folder *while the macro expands*,
/// which means every cargo command — `check`, `test`, `clippy` — fails with
/// "the frontendDist configuration is set to ../dist but this path doesn't
/// exist" unless the front end happens to have been built first. `tauri build`
/// handles that itself through `beforeBuildCommand`; a bare cargo command does
/// not, and CI kept tripping over it.
///
/// Rather than depend on every caller remembering the order, a clearly-labelled
/// placeholder is written when the folder is missing. Vite empties and replaces
/// the directory on a real build, so this file never reaches an installer — and
/// if it somehow did, it says so on screen instead of showing a blank window.
fn ensure_frontend_dist() {
    let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let manifest_dir = PathBuf::from(manifest_dir);

    let dist = frontend_dist(&manifest_dir).unwrap_or_else(|| manifest_dir.join("../dist"));
    if dist.join("index.html").is_file() {
        return;
    }

    if std::fs::create_dir_all(&dist).is_err() {
        return;
    }
    let _ = std::fs::write(
        dist.join("index.html"),
        "<!doctype html><meta charset=\"utf-8\"><title>GameHub</title>\n\
         <body style=\"font:14px system-ui;background:#0b0d12;color:#e8eaf0;padding:40px\">\n\
         <h1>The front end was not built</h1>\n\
         <p>This placeholder exists so <code>cargo test</code> can compile the crate.\n\
         Run <code>pnpm --filter @gamehub/desktop build</code>, or just\n\
         <code>pnpm installer</code>, which builds it for you.</p>\n",
    );
    println!("cargo:warning=frontendDist was empty; wrote a placeholder so the crate can compile. Run `pnpm --filter @gamehub/desktop build` for the real one.");
}

/// Reads `frontendDist` out of tauri.conf.json so this keeps working if the
/// path is ever changed there.
fn frontend_dist(manifest_dir: &Path) -> Option<PathBuf> {
    let config = std::fs::read_to_string(manifest_dir.join("tauri.conf.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&config).ok()?;
    let relative = value.get("build")?.get("frontendDist")?.as_str()?;
    // A URL means a dev server rather than a folder; nothing to create.
    if relative.starts_with("http") {
        return None;
    }
    Some(manifest_dir.join(relative))
}

fn main() {
    // Re-run if the config changes, since frontendDist lives there — and if the
    // dist index appears or disappears, or cargo replays a cached run and skips
    // the guard exactly when it is needed. Cargo tracks paths that do not
    // exist, so deleting the folder counts as a change.
    println!("cargo:rerun-if-changed=tauri.conf.json");
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        if let Some(dist) = frontend_dist(Path::new(&manifest_dir)) {
            println!("cargo:rerun-if-changed={}", dist.join("index.html").display());
        }
    }
    ensure_frontend_dist();
    tauri_build::build()
}
