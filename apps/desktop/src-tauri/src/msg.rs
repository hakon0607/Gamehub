//! Messages for the user, without the words.
//!
//! The Rust side never knows which language the interface is in, so anything
//! it wants to tell the user is sent as a code: `@key|param|param`. The web
//! view looks `key` up in its dictionary (`be.<key>` in `src/i18n/en.json`)
//! and fills the parameters in as `{0}`, `{1}`… A parameter may itself be a
//! code, which is how a launch error can wrap the reason it was given.
//!
//! Keeping this in one place means a test can check that every code used
//! here has a translation on the other side.

/// A user-facing message code with parameters.
pub fn code(key: &str, params: &[&str]) -> String {
    let mut out = String::with_capacity(1 + key.len() + params.iter().map(|p| p.len() + 1).sum::<usize>());
    out.push('@');
    out.push_str(key);
    for param in params {
        out.push('|');
        // The separator must not appear inside a parameter, or the web view
        // would split it in the wrong place.
        out.push_str(&param.replace('|', "¦"));
    }
    out
}

/// A user-facing message code without parameters.
pub fn plain(key: &str) -> String {
    code(key, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_key_then_params() {
        assert_eq!(plain("no_game_running"), "@no_game_running");
        assert_eq!(code("process_open", &["1234", "5"]), "@process_open|1234|5");
    }

    #[test]
    fn separators_inside_parameters_are_escaped() {
        assert_eq!(code("clip_save", &["a|b"]), "@clip_save|a¦b");
    }
}
