//! Every keyboard shortcut GameHub has, in one place.
//!
//! The point of a single registry is that Settings can list every shortcut that
//! actually exists — not a hand-maintained page that drifts from reality — and
//! that a rebind is checked against all the others before it is accepted.
//!
//! A shortcut is either **global** (works while a game has focus, registered
//! with the OS) or **app-only** (handled by the window). The distinction
//! matters to the user, so it is part of the model rather than a footnote.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Registered with Windows; fires even while a game is in front.
    Global,
    /// Only while the GameHub window has focus.
    App,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    OpenGameHub,
    QuickTools,
    Screenshot,
    SaveReplay,
    ToggleOverlay,
    Search,
    Library,
    Quests,
    Performance,
    Rescan,
    Screenshots,
    Clips,
    Home,
    Settings,
    Favorites,
    Streaks,
    Calendar,
    Clipboard,
    /// Turns the replay buffer on or off. Global, so it can be done while a
    /// game is in front — which is the only time it matters.
    ToggleReplay,
    /// Freezes the running game where it stands — or, if it is already
    /// frozen, lets it continue. Global: the whole point is a cutscene that
    /// will not wait.
    FreezeGame,
    /// Go to the freeze points page.
    Freezes,
}

impl Action {
    pub fn id(self) -> &'static str {
        match self {
            Action::OpenGameHub => "open_gamehub",
            Action::QuickTools => "quick_tools",
            Action::Screenshot => "screenshot",
            Action::SaveReplay => "save_replay",
            Action::ToggleOverlay => "toggle_overlay",
            Action::Search => "search",
            Action::Library => "library",
            Action::Quests => "quests",
            Action::Performance => "performance",
            Action::Rescan => "rescan",
            Action::Screenshots => "screenshots",
            Action::Clips => "clips",
            Action::Home => "home",
            Action::Settings => "settings",
            Action::Favorites => "favorites",
            Action::Streaks => "streaks",
            Action::Calendar => "calendar",
            Action::Clipboard => "clipboard",
            Action::ToggleReplay => "toggle_replay",
            Action::FreezeGame => "freeze_game",
            Action::Freezes => "freezes",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::OpenGameHub => "Open GameHub",
            Action::QuickTools => "Quick Tools",
            Action::Screenshot => "Take a screenshot",
            Action::SaveReplay => "Save the last moments (instant replay)",
            Action::ToggleOverlay => "Show performance while playing",
            Action::Search => "Search games",
            Action::Library => "Go to Library",
            Action::Quests => "Go to Quests",
            Action::Performance => "Go to Performance",
            Action::Rescan => "Scan for new games",
            Action::Screenshots => "Go to Screenshots",
            Action::Clips => "Go to Clips",
            Action::Home => "Go to Home",
            Action::Settings => "Go to Settings",
            Action::Favorites => "Go to Favourites",
            Action::Streaks => "Go to Streaks",
            Action::Calendar => "Go to Calendar",
            Action::Clipboard => "Go to Clipboard",
            Action::ToggleReplay => "Turn Replay on or off",
            Action::FreezeGame => "Freeze the game now / let it continue",
            Action::Freezes => "Go to Freeze points",
        }
    }

    pub fn scope(self) -> Scope {
        match self {
            // These have to work while a game is in front, or they are useless.
            Action::OpenGameHub
            | Action::QuickTools
            | Action::Screenshot
            | Action::SaveReplay
            | Action::ToggleOverlay
            | Action::ToggleReplay
            | Action::FreezeGame => {
                Scope::Global
            }
            _ => Scope::App,
        }
    }

    pub fn default_binding(self) -> &'static str {
        match self {
            Action::OpenGameHub => "Ctrl+Shift+G",
            Action::QuickTools => "Ctrl+Space",
            Action::Screenshot => "F9",
            Action::SaveReplay => "F8",
            Action::ToggleOverlay => "Ctrl+Shift+O",
            Action::Search => "Ctrl+F",
            Action::Library => "Ctrl+1",
            Action::Quests => "Ctrl+2",
            Action::Performance => "Ctrl+3",
            Action::Rescan => "F5",
            Action::Screenshots => "Ctrl+4",
            Action::Clips => "Ctrl+5",
            Action::Home => "Ctrl+0",
            Action::Settings => "Ctrl+,",
            Action::Favorites => "Ctrl+6",
            Action::Streaks => "Ctrl+7",
            Action::Calendar => "Ctrl+8",
            Action::Clipboard => "Ctrl+9",
            Action::ToggleReplay => "Ctrl+Shift+R",
            Action::FreezeGame => "F7",
            Action::Freezes => "Ctrl+Shift+F",
        }
    }

    pub fn all() -> Vec<Action> {
        vec![
            Action::OpenGameHub,
            Action::QuickTools,
            Action::Screenshot,
            Action::SaveReplay,
            Action::ToggleOverlay,
            Action::Search,
            Action::Library,
            Action::Quests,
            Action::Performance,
            Action::Rescan,
            Action::Screenshots,
            Action::Clips,
            Action::Home,
            Action::Settings,
            Action::Favorites,
            Action::Streaks,
            Action::Calendar,
            Action::Clipboard,
            Action::ToggleReplay,
            Action::FreezeGame,
            Action::Freezes,
        ]
    }

    pub fn from_id(id: &str) -> Option<Action> {
        Action::all().into_iter().find(|a| a.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub action: String,
    pub label: String,
    pub scope: Scope,
    /// The accelerator, e.g. "Ctrl+Shift+G". Empty means unbound.
    pub binding: String,
    pub default_binding: String,
}

/// The user's bindings, keyed by action id. Anything absent uses its default.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bindings(pub BTreeMap<String, String>);

impl Bindings {
    pub fn binding_for(&self, action: Action) -> String {
        self.0
            .get(action.id())
            .cloned()
            .unwrap_or_else(|| action.default_binding().to_string())
    }

    /// The full list, for the Shortcut Center.
    pub fn list(&self) -> Vec<Shortcut> {
        Action::all()
            .into_iter()
            .map(|action| Shortcut {
                action: action.id().to_string(),
                label: action.label().to_string(),
                scope: action.scope(),
                binding: self.binding_for(action),
                default_binding: action.default_binding().to_string(),
            })
            .collect()
    }

    /// Just the global ones, for registering with the OS.
    pub fn global(&self) -> Vec<(Action, String)> {
        Action::all()
            .into_iter()
            .filter(|a| a.scope() == Scope::Global)
            .map(|a| (a, self.binding_for(a)))
            .filter(|(_, binding)| !binding.is_empty())
            .collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RebindError {
    /// Already used by another action, which is named so the user can go and change it.
    Conflict { action: String, label: String },
    Invalid(String),
}

impl std::fmt::Display for RebindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Message codes the interface translates (`@key|param`); the
            // conflicting action is named by its own translated label.
            RebindError::Conflict { action, .. } => write!(f, "@shortcut_conflict|@sc.{action}"),
            RebindError::Invalid(why) => write!(f, "{why}"),
        }
    }
}

/// Accepts an accelerator only if it is one Tauri can register and a person
/// could actually press.
pub fn validate_binding(binding: &str) -> Result<String, RebindError> {
    let trimmed = binding.trim();
    if trimmed.is_empty() {
        // Clearing a shortcut is allowed.
        return Ok(String::new());
    }

    let parts: Vec<&str> = trimmed.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return Err(RebindError::Invalid("@shortcut_not_combo".into()));
    }

    let modifiers = ["ctrl", "control", "alt", "shift", "super", "cmd", "command", "meta"];
    let (mods, keys): (Vec<&&str>, Vec<&&str>) = parts
        .iter()
        .partition(|p| modifiers.contains(&p.to_lowercase().as_str()));

    if keys.len() != 1 {
        return Err(RebindError::Invalid("@shortcut_one_key".into()));
    }

    let key = keys[0].to_lowercase();
    let is_function_key = key.starts_with('f')
        && key[1..].parse::<u8>().map(|n| (1..=24).contains(&n)).unwrap_or(false);

    // A bare letter would fire every time the user typed it.
    if mods.is_empty() && !is_function_key {
        return Err(RebindError::Invalid("@shortcut_needs_modifier".into()));
    }

    Ok(trimmed.to_string())
}

/// Rebinds an action, refusing a combination another action already holds.
pub fn rebind(bindings: &mut Bindings, action: Action, binding: &str) -> Result<String, RebindError> {
    let candidate = validate_binding(binding)?;

    if !candidate.is_empty() {
        let normalised = normalise(&candidate);
        for other in Action::all() {
            if other == action {
                continue;
            }
            if normalise(&bindings.binding_for(other)) == normalised {
                return Err(RebindError::Conflict {
                    action: other.id().to_string(),
                    label: other.label().to_string(),
                });
            }
        }
    }

    bindings.0.insert(action.id().to_string(), candidate.clone());
    Ok(candidate)
}

/// Puts one action back to its default, even if that collides — the collision
/// is then reported by `list`, rather than silently refusing a reset.
pub fn reset(bindings: &mut Bindings, action: Action) {
    bindings.0.remove(action.id());
}

/// "ctrl+shift+g" from "Shift + Control+G", so two spellings of one combination
/// are recognised as the same shortcut.
fn normalise(binding: &str) -> String {
    let mut parts: Vec<String> = binding
        .split('+')
        .map(|p| p.trim().to_lowercase())
        .map(|p| match p.as_str() {
            "control" => "ctrl".to_string(),
            "cmd" | "command" | "meta" => "super".to_string(),
            other => other.to_string(),
        })
        .filter(|p| !p.is_empty())
        .collect();
    parts.sort();
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_a_default_and_they_do_not_collide() {
        let bindings = Bindings::default();
        let mut seen: Vec<String> = Vec::new();
        for shortcut in bindings.list() {
            assert!(!shortcut.binding.is_empty(), "{} has no default", shortcut.action);
            let key = normalise(&shortcut.binding);
            assert!(!seen.contains(&key), "two actions ship with {}", shortcut.binding);
            seen.push(key);
        }
    }

    #[test]
    fn the_center_lists_every_shortcut_that_exists() {
        assert_eq!(Bindings::default().list().len(), Action::all().len());
    }

    #[test]
    fn global_and_app_shortcuts_are_told_apart() {
        let bindings = Bindings::default();
        let global: Vec<String> = bindings.global().into_iter().map(|(a, _)| a.id().into()).collect();
        assert!(global.contains(&"screenshot".to_string()));
        assert!(global.contains(&"quick_tools".to_string()));
        assert!(!global.contains(&"search".to_string()), "search only makes sense in the window");
    }

    #[test]
    fn a_combination_another_action_holds_is_refused_and_names_it() {
        let mut bindings = Bindings::default();
        let error = rebind(&mut bindings, Action::Screenshot, "Ctrl+Space").unwrap_err();
        match error {
            RebindError::Conflict { label, .. } => assert_eq!(label, "Quick Tools"),
            other => panic!("expected a conflict, got {other:?}"),
        }
        assert_eq!(bindings.binding_for(Action::Screenshot), "F9", "the old binding survives a refusal");
    }

    #[test]
    fn the_same_combination_spelled_differently_still_conflicts() {
        let mut bindings = Bindings::default();
        assert!(rebind(&mut bindings, Action::Screenshot, "Space + Ctrl").is_err());
        assert!(rebind(&mut bindings, Action::Screenshot, "control+space").is_err());
    }

    #[test]
    fn a_bare_letter_is_refused_but_a_function_key_is_not() {
        assert!(validate_binding("G").is_err());
        assert!(validate_binding("F8").is_ok());
        assert!(validate_binding("Ctrl+G").is_ok());
        assert!(validate_binding("Ctrl+Shift+Alt+K").is_ok());
    }

    #[test]
    fn nonsense_combinations_are_refused() {
        assert!(validate_binding("Ctrl").is_err(), "modifiers alone are not a shortcut");
        assert!(validate_binding("Ctrl+A+B").is_err(), "two keys is not a shortcut");
        assert!(validate_binding("F99").is_err());
    }

    #[test]
    fn clearing_a_shortcut_is_allowed() {
        let mut bindings = Bindings::default();
        assert_eq!(rebind(&mut bindings, Action::Screenshot, "").unwrap(), "");
        assert!(!bindings.global().iter().any(|(a, _)| *a == Action::Screenshot));
    }

    #[test]
    fn rebinding_then_resetting_restores_the_default() {
        let mut bindings = Bindings::default();
        rebind(&mut bindings, Action::Screenshot, "Ctrl+Shift+P").unwrap();
        assert_eq!(bindings.binding_for(Action::Screenshot), "Ctrl+Shift+P");
        reset(&mut bindings, Action::Screenshot);
        assert_eq!(bindings.binding_for(Action::Screenshot), "F9");
    }

    #[test]
    fn an_action_can_be_rebound_to_what_it_already_has() {
        let mut bindings = Bindings::default();
        assert!(rebind(&mut bindings, Action::Screenshot, "F9").is_ok());
    }
}

#[cfg(test)]
mod registry_completeness {
    use super::*;

    /// The registry drives Settings, the OS registration and the in-app
    /// handler. A variant missing from `all()` is invisible to every one of
    /// them, so this checks the list covers the enum rather than trusting that
    /// whoever added a variant remembered.
    #[test]
    fn every_action_is_in_all() {
        // Exhaustive match: adding a variant makes this fail to compile until
        // it is listed here, and the assert then checks `all()` too.
        let expected = [
            Action::OpenGameHub,
            Action::QuickTools,
            Action::Screenshot,
            Action::SaveReplay,
            Action::ToggleOverlay,
            Action::Search,
            Action::Library,
            Action::Quests,
            Action::Performance,
            Action::Rescan,
            Action::Screenshots,
            Action::Clips,
            Action::Home,
            Action::Settings,
            Action::Favorites,
            Action::Streaks,
            Action::Calendar,
            Action::Clipboard,
            Action::ToggleReplay,
            Action::FreezeGame,
            Action::Freezes,
        ];
        for action in expected {
            let _exhaustive = match action {
                Action::OpenGameHub
                | Action::QuickTools
                | Action::Screenshot
                | Action::SaveReplay
                | Action::ToggleOverlay
                | Action::Search
                | Action::Library
                | Action::Quests
                | Action::Performance
                | Action::Rescan
                | Action::Screenshots
                | Action::Clips
                | Action::Home
                | Action::Settings
                | Action::Favorites
                | Action::Streaks
                | Action::Calendar
                | Action::Clipboard
                | Action::ToggleReplay
                | Action::FreezeGame
                | Action::Freezes => (),
            };
            assert!(Action::all().contains(&action), "{} is missing from all()", action.id());
        }
        assert_eq!(Action::all().len(), expected.len());
    }

    #[test]
    fn every_action_round_trips_through_its_id() {
        for action in Action::all() {
            assert_eq!(Action::from_id(action.id()), Some(action), "id {}", action.id());
        }
    }

    #[test]
    fn no_two_actions_share_a_default_binding() {
        // Two actions on one key means one of them silently never fires.
        let mut seen = std::collections::BTreeMap::new();
        for action in Action::all() {
            if let Some(other) = seen.insert(action.default_binding(), action) {
                panic!(
                    "{} and {} both default to {}",
                    other.id(),
                    action.id(),
                    action.default_binding()
                );
            }
        }
    }

    #[test]
    fn every_action_has_a_label_and_a_default() {
        for action in Action::all() {
            assert!(!action.label().is_empty(), "{} has no label", action.id());
            assert!(!action.default_binding().is_empty(), "{} has no default", action.id());
        }
    }

    #[test]
    fn navigation_shortcuts_are_app_scope_and_capture_is_global() {
        // A navigation shortcut registered with Windows would steal the key
        // from every other program; a capture shortcut that is not registered
        // would never work while a game is in front. Both matter.
        assert_eq!(Action::Screenshots.scope(), Scope::App);
        assert_eq!(Action::Clips.scope(), Scope::App);
        assert_eq!(Action::Settings.scope(), Scope::App);
        assert_eq!(Action::Screenshot.scope(), Scope::Global);
        assert_eq!(Action::SaveReplay.scope(), Scope::Global);
    }
}
