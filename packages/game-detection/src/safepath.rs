//! Path rules.
//!
//! GameHub launches programs, so the difference between "a path an adapter
//! found inside a Steam library" and "a path something else supplied" is the
//! whole security model. Nothing here trusts a string; every path that will be
//! executed goes through `validate_executable` immediately before it is used,
//! not only when it was discovered.

use std::path::{Component, Path, PathBuf};

use crate::error::{DetectError, Result};

/// Extensions GameHub is willing to start directly. Everything else — .bat,
/// .cmd, .ps1, .msi, .lnk, .scr — is refused: those are script and installer
/// formats, and a game that only ships one of them is better launched through
/// its own launcher.
pub const ALLOWED_EXECUTABLE_EXTENSIONS: &[&str] = &["exe"];

/// Protocol schemes GameHub will hand to the shell. A launcher URI is the
/// safest way to start a game, but only if the scheme is one we know — an
/// open-ended `ShellExecute` on a string from a manifest is not acceptable.
pub const ALLOWED_URI_SCHEMES: &[&str] = &[
    "steam",
    "com.epicgames.launcher",
    "uplay",
    "origin2",
    "link2ea",
    "battlenet",
    "goggalaxy",
    "riot",
    "minecraft",
];

/// Normalises a path lexically: no `..`, no `.`, consistent separators. This
/// does not touch the filesystem, so it works on paths that do not exist yet
/// and cannot be defeated by a race.
pub fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// True when `candidate` is `root` or lives under it, after normalising both.
/// Comparison is case-insensitive because Windows paths are.
pub fn is_within(root: &Path, candidate: &Path) -> bool {
    let root = normalise(root);
    let candidate = normalise(candidate);
    let root_s = root.to_string_lossy().to_lowercase().replace('\\', "/");
    let cand_s = candidate.to_string_lossy().to_lowercase().replace('\\', "/");
    let root_s = root_s.trim_end_matches('/').to_string();
    cand_s == root_s || cand_s.starts_with(&format!("{root_s}/"))
}

/// Joins a caller-supplied relative path onto a trusted root and refuses
/// anything that escapes it. Used for mod files, profile folders and metadata
/// cache entries, where the name comes from a file on disk or from the UI.
pub fn join_within(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.contains('\0') {
        return Err(DetectError::Rejected("path contains a null byte".into()));
    }
    let candidate = normalise(&root.join(relative));
    if !is_within(root, &candidate) {
        return Err(DetectError::Rejected(format!(
            "\"{relative}\" would escape {}",
            root.display()
        )));
    }
    Ok(candidate)
}

/// A path is only launchable if it exists, is a regular file, has an allowed
/// extension, and sits inside one of the roots GameHub already trusts for that
/// game. All four are checked at launch time.
pub fn validate_executable(path: &Path, allowed_roots: &[PathBuf]) -> Result<PathBuf> {
    let normalised = normalise(path);

    let extension = normalised
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !ALLOWED_EXECUTABLE_EXTENSIONS.contains(&extension.as_str()) {
        return Err(DetectError::Rejected(format!(
            "{} is not an executable GameHub will start",
            normalised.display()
        )));
    }

    if !allowed_roots.is_empty() && !allowed_roots.iter().any(|root| is_within(root, &normalised)) {
        return Err(DetectError::Rejected(format!(
            "{} is outside every folder GameHub knows about",
            normalised.display()
        )));
    }

    let meta = std::fs::metadata(&normalised).map_err(crate::error::io(normalised.clone()))?;
    if !meta.is_file() {
        return Err(DetectError::Rejected(format!(
            "{} is not a file",
            normalised.display()
        )));
    }

    Ok(normalised)
}

/// Accepts a launcher URI only if its scheme is one of ours and it has no
/// embedded control characters or quotes.
pub fn validate_uri(uri: &str) -> Result<String> {
    let scheme = uri
        .split_once("://")
        .map(|(s, _)| s)
        .or_else(|| uri.split_once(':').map(|(s, _)| s))
        .unwrap_or("")
        .to_lowercase();

    if !ALLOWED_URI_SCHEMES.contains(&scheme.as_str()) {
        return Err(DetectError::Rejected(format!(
            "\"{scheme}\" is not a launcher protocol GameHub recognises"
        )));
    }
    if uri.chars().any(|c| c.is_control() || c == '"' || c == '\'') {
        return Err(DetectError::Rejected(
            "launcher URI contains characters that are not allowed".into(),
        ));
    }
    if uri.len() > 2048 {
        return Err(DetectError::Rejected("launcher URI is implausibly long".into()));
    }
    Ok(uri.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn normalise_removes_traversal() {
        assert_eq!(
            normalise(Path::new("C:/Games/../Games/./sub/x.exe")),
            PathBuf::from("C:/Games/sub/x.exe")
        );
    }

    #[test]
    fn is_within_is_case_insensitive_and_not_prefix_fooled() {
        assert!(is_within(Path::new("C:/Games"), Path::new("c:/games/sub/a.exe")));
        assert!(is_within(Path::new("C:/Games/"), Path::new("C:/Games")));
        // "C:/GamesEvil" must not count as being inside "C:/Games".
        assert!(!is_within(Path::new("C:/Games"), Path::new("C:/GamesEvil/a.exe")));
    }

    #[test]
    fn join_within_refuses_to_escape() {
        let root = Path::new("/library/mods");
        assert!(join_within(root, "sodium.jar").is_ok());
        assert!(join_within(root, "../../etc/passwd").is_err());
        assert!(join_within(root, "sub/../../../outside.jar").is_err());
        assert!(join_within(root, "ok\0.jar").is_err());
    }

    #[test]
    fn validate_executable_checks_extension_root_and_existence() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let exe = root.join("game.exe");
        fs::write(&exe, b"MZ").unwrap();
        let script = root.join("run.bat");
        fs::write(&script, b"echo").unwrap();

        assert!(validate_executable(&exe, &[root.clone()]).is_ok());
        // Wrong extension.
        assert!(validate_executable(&script, &[root.clone()]).is_err());
        // Outside the trusted root.
        assert!(validate_executable(Path::new("/bin/sh.exe"), &[root.clone()]).is_err());
        // Does not exist.
        assert!(validate_executable(&root.join("missing.exe"), &[root.clone()]).is_err());
        // A directory that happens to be named like an executable.
        let fake = root.join("dir.exe");
        fs::create_dir(&fake).unwrap();
        assert!(validate_executable(&fake, &[root.clone()]).is_err());
    }

    #[test]
    fn traversal_out_of_a_trusted_root_is_refused_even_if_the_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("trusted");
        fs::create_dir_all(&root).unwrap();
        let outside = dir.path().join("evil.exe");
        fs::write(&outside, b"MZ").unwrap();
        let sneaky = root.join("../evil.exe");
        assert!(validate_executable(&sneaky, &[root]).is_err());
    }

    #[test]
    fn only_known_launcher_schemes_are_accepted() {
        assert!(validate_uri("steam://rungameid/440").is_ok());
        assert!(validate_uri("com.epicgames.launcher://apps/Fortnite?action=launch").is_ok());
        assert!(validate_uri("file:///C:/Windows/System32/cmd.exe").is_err());
        assert!(validate_uri("http://example.com").is_err());
        assert!(validate_uri("steam://rungameid/440\" && calc").is_err());
    }
}
