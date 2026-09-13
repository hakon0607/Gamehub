//! Clipboard history.
//!
//! The last hundred things copied, kept on this PC and nowhere else. Clipboard
//! contents are among the most sensitive data a program can hold — passwords,
//! tokens, private messages pass through it — so three rules hold here:
//!
//! * Nothing is ever sent anywhere. There is no network code in this module.
//! * Anything that looks like a credential is skipped rather than stored.
//! * The whole thing can be switched off and wiped from Settings.

use serde::{Deserialize, Serialize};

/// The spec's number: keep 100, drop the oldest when the 101st arrives.
pub const MAX_ITEMS: usize = 100;

/// Text longer than this is stored truncated — a copied logfile should not sit
/// in memory forever.
pub const MAX_TEXT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    pub text: String,
    /// Characters in the original, which may exceed what is stored.
    pub length: usize,
    pub truncated: bool,
    pub copied_at: String,
    pub pinned: bool,
    /// True when the text parses as a single URL, so the UI can mark it.
    pub is_url: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipboardHistory {
    pub items: Vec<ClipItem>,
    pub enabled: bool,
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        Self { items: Vec::new(), enabled: true }
    }
}

/// Patterns that mark text as a secret worth not recording. This cannot be
/// perfect, and it is not presented as if it were — it is a courtesy that
/// catches the obvious cases.
fn looks_like_a_secret(text: &str) -> bool {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();

    // Well-known token shapes.
    let prefixes = [
        "sk-", "sk_live", "sk_test", "pk_live", "ghp_", "gho_", "ghu_", "ghs_", "github_pat_",
        "xoxb-", "xoxp-", "xapp-", "aws_secret", "akia", "-----begin",
    ];
    if prefixes.iter().any(|p| lower.starts_with(p)) {
        return true;
    }

    // "password: hunter2" and friends.
    let labels = ["password", "passord", "secret", "api key", "api_key", "apikey", "token:", "private key"];
    if trimmed.len() < 200 && labels.iter().any(|l| lower.contains(l)) {
        return true;
    }

    // A long unbroken run of random-looking characters with no spaces is far
    // more likely to be a key than something a person meant to keep.
    if trimmed.len() >= 32
        && !trimmed.contains(char::is_whitespace)
        && trimmed.chars().any(|c| c.is_ascii_digit())
        && trimmed.chars().any(|c| c.is_ascii_uppercase())
        && trimmed.chars().any(|c| c.is_ascii_lowercase())
        && !trimmed.starts_with("http")
    {
        return true;
    }

    false
}

fn is_url(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.contains(char::is_whitespace)
        && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
}

/// Records something newly copied.
///
/// Returns whether it was stored, so the caller can tell "nothing new" from
/// "deliberately skipped".
pub fn record(history: &mut ClipboardHistory, text: &str, now: &str) -> bool {
    if !history.enabled {
        return false;
    }
    let trimmed = text.trim_end_matches(['\n', '\r']);
    if trimmed.trim().is_empty() {
        return false;
    }
    if looks_like_a_secret(trimmed) {
        return false;
    }

    // The same text copied twice moves to the top rather than appearing twice.
    if let Some(position) = history.items.iter().position(|item| item.text == truncate(trimmed).0) {
        let mut existing = history.items.remove(position);
        existing.copied_at = now.to_string();
        history.items.insert(0, existing);
        return true;
    }

    let (stored, truncated) = truncate(trimmed);
    history.items.insert(
        0,
        ClipItem {
            id: format!("{now}-{}", history.items.len()),
            length: trimmed.chars().count(),
            is_url: is_url(trimmed),
            text: stored,
            truncated,
            copied_at: now.to_string(),
            pinned: false,
        },
    );

    // Pinned items are never pushed out by the cap.
    if history.items.len() > MAX_ITEMS {
        if let Some(position) = history.items.iter().rposition(|item| !item.pinned) {
            history.items.remove(position);
        }
    }
    true
}

fn truncate(text: &str) -> (String, bool) {
    if text.len() <= MAX_TEXT_BYTES {
        return (text.to_string(), false);
    }
    let mut end = MAX_TEXT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

pub fn search<'a>(history: &'a ClipboardHistory, query: &str) -> Vec<&'a ClipItem> {
    let query = query.trim().to_lowercase();
    let mut items: Vec<&ClipItem> = if query.is_empty() {
        history.items.iter().collect()
    } else {
        history.items.iter().filter(|i| i.text.to_lowercase().contains(&query)).collect()
    };
    // Pinned first, then newest.
    items.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.copied_at.cmp(&a.copied_at)));
    items
}

pub fn set_pinned(history: &mut ClipboardHistory, id: &str, pinned: bool) -> bool {
    match history.items.iter_mut().find(|i| i.id == id) {
        Some(item) => {
            item.pinned = pinned;
            true
        }
        None => false,
    }
}

pub fn remove(history: &mut ClipboardHistory, id: &str) {
    history.items.retain(|i| i.id != id);
}

/// Wipes everything, pinned included — the user asked.
pub fn clear(history: &mut ClipboardHistory) {
    history.items.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(n: usize) -> String {
        format!("2026-08-30T12:{:02}:00Z", n % 60)
    }

    #[test]
    fn the_hundred_and_first_item_pushes_out_the_oldest() {
        let mut history = ClipboardHistory::default();
        for i in 0..MAX_ITEMS {
            record(&mut history, &format!("item {i}"), &at(i));
        }
        assert_eq!(history.items.len(), MAX_ITEMS);

        record(&mut history, "the newest", &at(200));
        assert_eq!(history.items.len(), MAX_ITEMS);
        assert_eq!(history.items[0].text, "the newest");
        assert!(!history.items.iter().any(|i| i.text == "item 0"));
    }

    #[test]
    fn a_pinned_item_is_never_pushed_out() {
        let mut history = ClipboardHistory::default();
        record(&mut history, "keep me", &at(0));
        let id = history.items[0].id.clone();
        set_pinned(&mut history, &id, true);

        for i in 1..=MAX_ITEMS + 20 {
            record(&mut history, &format!("filler {i}"), &at(i));
        }
        assert!(history.items.iter().any(|i| i.text == "keep me"));
        assert_eq!(history.items.len(), MAX_ITEMS);
    }

    #[test]
    fn copying_the_same_thing_twice_moves_it_up_rather_than_duplicating() {
        let mut history = ClipboardHistory::default();
        record(&mut history, "hello", &at(1));
        record(&mut history, "world", &at(2));
        record(&mut history, "hello", &at(3));
        assert_eq!(history.items.len(), 2);
        assert_eq!(history.items[0].text, "hello");
    }

    #[test]
    fn obvious_credentials_are_not_recorded() {
        let mut history = ClipboardHistory::default();
        for secret in [
            "sk-abcdefghijklmnopqrstuvwxyz123456",
            "ghp_16CharactersOfTokenHere1234567890",
            "password: hunter2",
            "Passord: veldig hemmelig",
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "aB3xK9mQ7zP2wR5tY8uI1oP4aS6dF0gH",
        ] {
            assert!(!record(&mut history, secret, &at(1)), "stored a secret: {secret}");
        }
        assert!(history.items.is_empty());
    }

    #[test]
    fn ordinary_text_and_urls_are_recorded() {
        let mut history = ClipboardHistory::default();
        assert!(record(&mut history, "https://github.com/hakon0607/gamehub", &at(1)));
        assert!(record(&mut history, "Husk å kjøpe melk", &at(2)));
        assert!(history.items.iter().any(|i| i.is_url));
    }

    #[test]
    fn nothing_is_recorded_while_history_is_switched_off() {
        let mut history = ClipboardHistory { enabled: false, ..Default::default() };
        assert!(!record(&mut history, "anything", &at(1)));
        assert!(history.items.is_empty());
    }

    #[test]
    fn very_long_text_is_stored_truncated_and_says_so() {
        let mut history = ClipboardHistory::default();
        let huge = "x".repeat(MAX_TEXT_BYTES + 5000);
        record(&mut history, &huge, &at(1));
        let item = &history.items[0];
        assert!(item.truncated);
        assert!(item.text.len() <= MAX_TEXT_BYTES);
        assert_eq!(item.length, MAX_TEXT_BYTES + 5000, "the real length is still reported");
    }

    #[test]
    fn search_matches_content_and_puts_pinned_first() {
        let mut history = ClipboardHistory::default();
        record(&mut history, "minecraft install notes", &at(1));
        record(&mut history, "something else", &at(2));
        record(&mut history, "minecraft server ip", &at(3));

        assert_eq!(search(&history, "minecraft").len(), 2);
        assert_eq!(search(&history, "").len(), 3);

        let id = history.items.iter().find(|i| i.text.contains("else")).unwrap().id.clone();
        set_pinned(&mut history, &id, true);
        assert!(search(&history, "").first().unwrap().pinned);
    }

    #[test]
    fn empty_and_whitespace_only_copies_are_ignored() {
        let mut history = ClipboardHistory::default();
        assert!(!record(&mut history, "   \n ", &at(1)));
        assert!(history.items.is_empty());
    }

    #[test]
    fn clearing_removes_everything_including_pinned() {
        let mut history = ClipboardHistory::default();
        record(&mut history, "one", &at(1));
        let id = history.items[0].id.clone();
        set_pinned(&mut history, &id, true);
        clear(&mut history);
        assert!(history.items.is_empty());
    }
}
