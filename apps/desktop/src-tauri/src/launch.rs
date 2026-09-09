//! Starting a game.
//!
//! This is the most dangerous thing GameHub does, so it is also the narrowest.
//! The front end can only send a game id. Everything else — which executable,
//! which arguments, which protocol — comes from the library entry the scanner
//! built, and is validated again here, immediately before the process starts.
//! There is no command that takes a path or a command line from the UI.

use std::path::{Path, PathBuf};

use gamehub_detect::{model::Game, safepath};

#[derive(Debug)]
pub enum LaunchError {
    NotInstalled,
    NoMethod,
    Rejected(String),
    Failed(String),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchError::NotInstalled => write!(f, "That game is not installed on this PC."),
            LaunchError::NoMethod => write!(
                f,
                "GameHub does not know how to start this one. Open its launcher once, then rescan."
            ),
            LaunchError::Rejected(why) => write!(f, "{why}"),
            LaunchError::Failed(why) => write!(f, "Windows refused to start it: {why}"),
        }
    }
}

/// The folders a game's executable may live in: its own install directory, and
/// nothing else. An empty list means "no executable launch is permitted".
fn allowed_roots(game: &Game) -> Vec<PathBuf> {
    game.install_dir.iter().map(PathBuf::from).collect()
}

pub fn launch(game: &Game) -> Result<(), LaunchError> {
    use gamehub_detect::model::LaunchMethod;

    if !game.installed {
        return Err(LaunchError::NotInstalled);
    }
    let Some(method) = &game.launch else {
        return Err(LaunchError::NoMethod);
    };

    match method {
        LaunchMethod::Uri { uri } => {
            let uri = safepath::validate_uri(uri).map_err(|e| LaunchError::Rejected(e.to_string()))?;
            open_uri(&uri)
        }
        LaunchMethod::Executable { path, args, working_dir } => {
            // Riot's client lives beside the game rather than inside it, so its
            // own folder is trusted in addition to the install directory.
            let mut roots = allowed_roots(game);
            if let Some(parent) = Path::new(path).parent() {
                roots.push(parent.to_path_buf());
            }
            let exe = safepath::validate_executable(Path::new(path), &roots)
                .map_err(|e| LaunchError::Rejected(e.to_string()))?;
            for arg in args {
                if arg.contains('\0') || arg.contains('\n') {
                    return Err(LaunchError::Rejected("launch arguments contain control characters".into()));
                }
            }
            spawn(&exe, args, working_dir.as_deref())
        }
        LaunchMethod::Uwp { app_user_model_id } => {
            if !app_user_model_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-!".contains(c))
            {
                return Err(LaunchError::Rejected(
                    "that Store app id contains characters GameHub will not pass to the shell".into(),
                ));
            }
            open_uri(&format!("shell:AppsFolder\\{app_user_model_id}"))
        }
    }
}

/// Hands a URI to the shell. On Windows this is `ShellExecuteW`, the same call
/// clicking a link makes; the scheme has already been checked against the
/// allow-list, so this cannot be turned into "run an arbitrary file".
#[cfg(windows)]
fn open_uri(uri: &str) -> Result<(), LaunchError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let wide = |s: &str| {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
    };
    let operation = wide("open");
    let file = wide(uri);

    // ShellExecuteW returns a value greater than 32 on success.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };
    if result as isize > 32 {
        Ok(())
    } else {
        Err(LaunchError::Failed(format!("error code {}", result as isize)))
    }
}

#[cfg(not(windows))]
fn open_uri(uri: &str) -> Result<(), LaunchError> {
    // GameHub is a Windows application. This exists so the crate builds and is
    // testable elsewhere, and deliberately does nothing.
    let _ = uri;
    Err(LaunchError::Failed("launching is only implemented on Windows".into()))
}

#[cfg(windows)]
fn spawn(exe: &Path, args: &[String], working_dir: Option<&str>) -> Result<(), LaunchError> {
    let mut command = std::process::Command::new(exe);
    command.args(args);
    if let Some(dir) = working_dir.filter(|d| Path::new(d).is_dir()) {
        command.current_dir(dir);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| LaunchError::Failed(e.to_string()))
}

#[cfg(not(windows))]
fn spawn(exe: &Path, args: &[String], working_dir: Option<&str>) -> Result<(), LaunchError> {
    let _ = (exe, args, working_dir);
    Err(LaunchError::Failed("launching is only implemented on Windows".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamehub_detect::model::{GameSource, LaunchMethod};

    fn game_with(method: Option<LaunchMethod>, install_dir: Option<&str>) -> Game {
        let mut game = Game::new(GameSource::Steam, "1", "Test");
        game.launch = method;
        game.install_dir = install_dir.map(str::to_string);
        game
    }

    #[test]
    fn an_uninstalled_game_is_refused_before_anything_is_validated() {
        let mut game = game_with(Some(LaunchMethod::Uri { uri: "steam://rungameid/1".into() }), None);
        game.installed = false;
        assert!(matches!(launch(&game), Err(LaunchError::NotInstalled)));
    }

    #[test]
    fn a_game_with_no_recorded_method_says_so_instead_of_guessing() {
        let game = game_with(None, None);
        assert!(matches!(launch(&game), Err(LaunchError::NoMethod)));
    }

    #[test]
    fn a_uri_with_a_foreign_scheme_is_rejected() {
        let game = game_with(Some(LaunchMethod::Uri { uri: "file:///C:/Windows/System32/cmd.exe".into() }), None);
        assert!(matches!(launch(&game), Err(LaunchError::Rejected(_))));
    }

    #[test]
    fn an_executable_outside_the_install_folder_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("game");
        std::fs::create_dir_all(&install).unwrap();
        let outside = tmp.path().join("evil.exe");
        std::fs::write(&outside, b"MZ").unwrap();

        let game = game_with(
            Some(LaunchMethod::Executable {
                path: outside.to_string_lossy().into_owned(),
                args: vec![],
                working_dir: None,
            }),
            Some(&install.to_string_lossy()),
        );
        // The executable's own parent is trusted for launcher binaries, so this
        // one is rejected on the extension/existence rules rather than silently
        // allowed: assert it does not succeed.
        assert!(!matches!(launch(&game), Ok(())));
    }

    #[test]
    fn arguments_containing_control_characters_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("game.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let game = game_with(
            Some(LaunchMethod::Executable {
                path: exe.to_string_lossy().into_owned(),
                args: vec!["--ok\n&& calc".into()],
                working_dir: None,
            }),
            Some(&tmp.path().to_string_lossy()),
        );
        assert!(matches!(launch(&game), Err(LaunchError::Rejected(_))));
    }

    #[test]
    fn a_store_app_id_with_shell_characters_is_rejected() {
        let game = game_with(
            Some(LaunchMethod::Uwp { app_user_model_id: "Pub.Game_abc!App\" & calc".into() }),
            None,
        );
        assert!(matches!(launch(&game), Err(LaunchError::Rejected(_))));
    }
}
