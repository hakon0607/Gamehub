//! Cover art.
//!
//! Steam publishes the artwork for every app on its own CDN, keyed by app id
//! and needing no account, no key and no API call — which is why the library
//! can show real covers out of the box rather than coloured initials.
//!
//! Everything is cached on disk under the app data folder and fetched once.
//! Games from other launchers have no equivalent public source, so they keep
//! their initials until the user adds an IGDB or SteamGridDB key; pretending
//! otherwise would mean guessing at matches and getting them wrong.

use std::path::{Path, PathBuf};

use gamehub_detect::model::{Game, GameMetadata, GameSource};

/// Portrait cover, the shape the library grid uses.
fn cover_url(appid: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg")
}

/// Wide banner, used behind the game detail page. Also the fallback cover for
/// older titles that predate the portrait format.
fn hero_url(appid: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/header.jpg")
}

/// Steam's public app list: every app id and name, no key required. It is
/// several megabytes, so it is fetched once a week and cached.
const APP_LIST_URL: &str = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";

/// Reduces a title to something two spellings of the same game agree on:
/// "Marvel's Spider-Man: Remastered" and "Marvel s Spider Man Remastered" both
/// become "marvelsspidermanremastered".
pub fn match_key(name: &str) -> String {
    let lowered = name.to_lowercase();
    let stripped = lowered
        .replace('&', " and ")
        .replace('+', " plus ");
    let mut key: String = stripped.chars().filter(|c| c.is_alphanumeric()).collect();

    // Edition and platform suffixes that differ between stores but not games.
    for noise in [
        "gameoftheyearedition", "goty", "definitiveedition", "remastered", "enhancededition",
        "completeedition", "deluxeedition", "ultimateedition", "standardedition", "windowsedition",
        "forwindows", "pcedition", "edition", "trial", "demo",
    ] {
        if key.len() > noise.len() + 3 && key.ends_with(noise) {
            key.truncate(key.len() - noise.len());
        }
    }
    key
}

/// The cached Steam app list, as match key to app id.
pub struct AppIndex {
    by_key: std::collections::HashMap<String, String>,
}

impl AppIndex {
    /// Builds the index from Steam's JSON. Lower app ids win a tie, because a
    /// game's own entry predates its demos, soundtracks and test builds.
    pub fn from_json(json: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(json).ok()?;
        let apps = value.get("applist")?.get("apps")?.as_array()?;
        let mut by_key: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for app in apps {
            let (Some(id), Some(name)) = (app.get("appid").and_then(|v| v.as_u64()), app.get("name").and_then(|v| v.as_str()))
            else {
                continue;
            };
            let key = match_key(name);
            if key.len() < 3 {
                continue;
            }
            by_key.entry(key).and_modify(|existing| *existing = (*existing).min(id)).or_insert(id);
        }

        Some(Self {
            by_key: by_key.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
        })
    }

    pub fn lookup(&self, name: &str) -> Option<&str> {
        self.by_key.get(&match_key(name)).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }
}

/// Fetches the app list, reusing a cached copy for a week.
fn load_app_index(client: &reqwest::blocking::Client, cache: &Path) -> Option<AppIndex> {
    let path = cache.join("steam-apps.json");
    let fresh = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .map(|t| t.elapsed().map(|age| age.as_secs() < 7 * 24 * 3600).unwrap_or(false))
        .unwrap_or(false);

    if fresh {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Some(index) = AppIndex::from_json(&text) {
                return Some(index);
            }
        }
    }

    let text = client.get(APP_LIST_URL).send().ok()?.text().ok()?;
    let index = AppIndex::from_json(&text)?;
    let _ = std::fs::write(&path, &text);
    Some(index)
}

/// Result of one game's fetch, so the caller can report progress.
pub struct Fetched {
    pub game_id: String,
    pub metadata: GameMetadata,
}

/// Downloads whatever artwork is missing, newest-looking games first.
///
/// Network failures are not errors here: a PC with no internet simply keeps the
/// initials, and the next scan tries again. Nothing about the library depends
/// on this succeeding.
pub fn fetch_missing(games: &[Game], cache: &Path, limit: usize) -> Vec<Fetched> {
    let _ = std::fs::create_dir_all(cache);

    let client = match reqwest::blocking::Client::builder()
        .user_agent("GameHub/0.1 (+https://github.com/OWNER/gamehub)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(client) => client,
        Err(_) => return Vec::new(),
    };

    // Games from other launchers are looked up by name in Steam's public app
    // list — most PC games are on Steam whether or not you bought them there.
    // The match can be wrong, so it is recorded as a guess and the UI offers to
    // replace it.
    let needs_lookup = games
        .iter()
        .any(|g| g.source != GameSource::Steam && !has_artwork(g));
    let index = needs_lookup.then(|| load_app_index(&client, cache)).flatten();

    let mut out = Vec::new();
    for game in games {
        if out.len() >= limit {
            break;
        }
        if has_artwork(game) {
            continue;
        }

        let (appid, guessed) = if game.source == GameSource::Steam {
            if !game.source_id.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            (game.source_id.clone(), false)
        } else {
            match index.as_ref().and_then(|index| index.lookup(&game.name)) {
                Some(found) => (found.to_string(), true),
                None => continue,
            }
        };

        let cover = download(&client, &cover_url(&appid), cache, &format!("{appid}_cover.jpg"));
        let hero = download(&client, &hero_url(&appid), cache, &format!("{appid}_hero.jpg"));

        // A game with neither is left alone so the next run retries it.
        if cover.is_none() && hero.is_none() {
            continue;
        }

        let mut metadata = game.metadata.clone().unwrap_or_default();
        // Some older titles have no portrait capsule; the banner reads better
        // than a letter, so it stands in.
        metadata.cover_path = cover.clone().or_else(|| hero.clone()).map(path_string);
        metadata.hero_path = hero.map(path_string);
        // "steam-guess" travels through to the UI, which marks the cover as a
        // guess so a wrong match is obvious rather than puzzling.
        metadata.provider = Some(if guessed { "steam-guess" } else { "steam" }.into());
        metadata.fetched_at = Some(gamehub_detect::now_iso8601());

        out.push(Fetched { game_id: game.id.clone(), metadata });
    }
    out
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn has_artwork(game: &Game) -> bool {
    let Some(metadata) = game.metadata.as_ref() else { return false };
    // A cover the user chose themselves is never replaced by a fetched one.
    if metadata.provider.as_deref() == Some("manual") {
        return true;
    }
    metadata
        .cover_path
        .as_ref()
        .map(|p| Path::new(p).is_file())
        .unwrap_or(false)
}

/* -------------------------------------------------------------------------- */
/* Covers the user supplies                                                   */
/* -------------------------------------------------------------------------- */

/// Largest source image accepted for cropping. Big enough for a 4K screenshot,
/// small enough that a mistaken pick of a video file is refused rather than
/// loaded into memory.
pub const MAX_SOURCE_BYTES: u64 = 24 * 1024 * 1024;

/// Largest cropped PNG written back. The crop is 600x900, so this is generous.
pub const MAX_COVER_BYTES: usize = 8 * 1024 * 1024;

/// The image formats a cover may come from, by their file signature rather
/// than by extension — a `.png` that is really something else is refused.
fn sniff_image(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(&[0x42, 0x4d]) {
        Some("image/bmp")
    } else {
        None
    }
}

/// Reads an image the user picked so the crop tool can display it.
///
/// The path comes from the OS file picker, which the user drove themselves, and
/// it is still checked: it must be a regular file, within the size cap, and
/// actually an image by signature. The web view never gets filesystem access —
/// it gets one data URL for one file it asked for.
pub fn read_for_crop(path: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("That file could not be opened: {e}"))?;
    if !meta.is_file() {
        return Err("That is not a file.".into());
    }
    if meta.len() > MAX_SOURCE_BYTES {
        return Err("That image is larger than 24 MB. Pick a smaller one.".into());
    }

    let bytes = std::fs::read(path).map_err(|e| format!("That file could not be read: {e}"))?;
    let mime = sniff_image(&bytes).ok_or("That file is not an image GameHub can read.")?;
    Ok(format!("data:{mime};base64,{}", base64_encode(&bytes)))
}

/// Stores a cropped cover for one game and returns the path it was written to.
pub fn save_custom_cover(cache: &Path, game_id: &str, png: &[u8]) -> Result<PathBuf, String> {
    if png.len() > MAX_COVER_BYTES {
        return Err("That cover is too large to store.".into());
    }
    if sniff_image(png) != Some("image/png") {
        return Err("A cover must be a PNG.".into());
    }

    std::fs::create_dir_all(cache).map_err(|e| format!("The artwork folder could not be created: {e}"))?;

    // A game id looks like "steam:440", and a colon is not legal in a Windows
    // file name, so it is reduced to something that always is.
    let file_name = format!("{}_custom.png", safe_stem(game_id));
    let target = gamehub_detect::safepath::join_within(cache, &file_name)
        .map_err(|e| e.to_string())?;
    std::fs::write(&target, png).map_err(|e| format!("The cover could not be saved: {e}"))?;
    Ok(target)
}

/// Deletes a custom cover, so the game falls back to a fetched one.
pub fn clear_custom_cover(cache: &Path, game_id: &str) {
    let file_name = format!("{}_custom.png", safe_stem(game_id));
    if let Ok(path) = gamehub_detect::safepath::join_within(cache, &file_name) {
        let _ = std::fs::remove_file(path);
    }
}

fn safe_stem(game_id: &str) -> String {
    game_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .take(80)
        .collect()
}

/// Base64 without pulling in a crate for forty lines of table lookup.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[n as usize & 63] as char } else { '=' });
    }
    out
}

/// The inverse, for the cropped PNG coming back from the web view.
pub fn base64_decode(value: &str) -> Result<Vec<u8>, String> {
    let data = value.rsplit(',').next().unwrap_or(value);
    let mut out = Vec::with_capacity(data.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in data.bytes() {
        let index = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' => continue,
            _ => return Err("The image data was malformed.".into()),
        } as u32;
        buffer = (buffer << 6) | index;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Ok(out)
}

/// Fetches one image into the cache. A file already there is reused, so a
/// rescan costs nothing.
fn download(
    client: &reqwest::blocking::Client,
    url: &str,
    cache: &Path,
    file_name: &str,
) -> Option<PathBuf> {
    // The name is built from a digits-only app id, so it cannot escape the
    // cache directory — but it is checked anyway, because this writes to disk.
    let target = gamehub_detect::safepath::join_within(cache, file_name).ok()?;
    if target.is_file() {
        return Some(target);
    }

    let response = client.get(url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().ok()?;
    // Steam serves a small placeholder for apps with no art; anything this
    // short is not a real cover.
    if bytes.len() < 2048 {
        return None;
    }
    std::fs::write(&target, &bytes).ok()?;
    Some(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_built_from_the_app_id() {
        assert_eq!(
            cover_url("440"),
            "https://cdn.cloudflare.steamstatic.com/steam/apps/440/library_600x900.jpg"
        );
        assert!(hero_url("440").ends_with("/440/header.jpg"));
    }

    #[test]
    fn titles_that_differ_only_in_punctuation_match_each_other() {
        assert_eq!(match_key("Marvel's Spider-Man: Remastered"), match_key("Marvel s Spider Man  Remastered"));
        assert_eq!(match_key("Half-Life 2"), match_key("Half Life 2"));
        assert_eq!(match_key("Rock & Roll"), match_key("Rock and Roll"));
    }

    #[test]
    fn edition_suffixes_do_not_stop_a_match() {
        assert_eq!(match_key("Skyrim Special Edition"), match_key("Skyrim Special"));
        assert_eq!(match_key("Cyberpunk 2077"), "cyberpunk2077", "a year is part of the name, not noise");
    }

    #[test]
    fn the_app_index_prefers_the_game_over_its_demo_entries() {
        let json = r#"{"applist":{"apps":[
            {"appid":220,"name":"Half-Life 2"},
            {"appid":999999,"name":"Half-Life 2 Demo"},
            {"appid":221,"name":"Half Life 2"}
        ]}}"#;
        let index = AppIndex::from_json(json).unwrap();
        assert_eq!(index.lookup("Half-Life 2"), Some("220"), "the lower app id is the real game");
        assert!(index.lookup("Nothing Like This").is_none());
    }

    #[test]
    fn a_malformed_app_list_yields_no_index_rather_than_a_panic() {
        assert!(AppIndex::from_json("{ not json").is_none());
        assert!(AppIndex::from_json(r#"{"applist":{}}"#).is_none());
    }

    #[test]
    fn a_non_steam_game_with_no_cached_index_touches_no_network_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut game = Game::new(GameSource::Epic, "Fortnite", "Fortnite");
        game.metadata = Some(GameMetadata {
            cover_path: Some("x".into()),
            provider: Some("manual".into()),
            ..Default::default()
        });
        // Already has a chosen cover, so nothing is looked up at all.
        assert!(fetch_missing(&[game], tmp.path(), 10).is_empty());
    }

    #[test]
    fn a_game_that_already_has_a_cached_cover_is_not_refetched() {
        let tmp = tempfile::tempdir().unwrap();
        let cover = tmp.path().join("440_cover.jpg");
        std::fs::write(&cover, vec![0u8; 4096]).unwrap();

        let mut game = Game::new(GameSource::Steam, "440", "Team Fortress 2");
        game.metadata = Some(GameMetadata {
            cover_path: Some(cover.to_string_lossy().into_owned()),
            ..Default::default()
        });
        assert!(has_artwork(&game));
        assert!(fetch_missing(&[game], tmp.path(), 10).is_empty());
    }

    #[test]
    fn base64_round_trips() {
        for sample in [&b""[..], b"a", b"ab", b"abc", b"abcd", &[0x89, b'P', b'N', b'G', 0, 255, 12][..]] {
            let encoded = base64_encode(sample);
            assert_eq!(base64_decode(&encoded).unwrap(), sample, "failed for {sample:?}");
        }
    }

    #[test]
    fn images_are_recognised_by_signature_not_extension() {
        assert_eq!(sniff_image(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0]), Some("image/png"));
        assert_eq!(sniff_image(&[0xff, 0xd8, 0xff, 0xe0]), Some("image/jpeg"));
        assert_eq!(sniff_image(b"MZ this is an executable"), None);
        assert_eq!(sniff_image(b""), None);
    }

    #[test]
    fn a_file_that_is_not_an_image_is_refused_however_it_is_named() {
        let tmp = tempfile::tempdir().unwrap();
        let fake = tmp.path().join("cover.png");
        std::fs::write(&fake, b"MZ not really a png").unwrap();
        assert!(read_for_crop(&fake).is_err());
    }

    #[test]
    fn a_custom_cover_is_written_under_a_file_name_windows_accepts() {
        let tmp = tempfile::tempdir().unwrap();
        let png = [&[0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a][..], &[0u8; 100][..]].concat();
        let path = save_custom_cover(tmp.path(), "steam:440", &png).unwrap();
        assert!(path.is_file());
        assert!(!path.to_string_lossy().contains(':') || cfg!(windows));
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("steam_440"));
    }

    #[test]
    fn a_custom_cover_survives_the_artwork_fetcher() {
        let tmp = tempfile::tempdir().unwrap();
        let mut game = Game::new(GameSource::Steam, "440", "Team Fortress 2");
        game.metadata = Some(GameMetadata {
            cover_path: Some("chosen.png".into()),
            provider: Some("manual".into()),
            ..Default::default()
        });
        assert!(has_artwork(&game), "a chosen cover is never replaced by a fetched one");
        assert!(fetch_missing(&[game], tmp.path(), 10).is_empty());
    }

    #[test]
    fn something_that_is_not_a_png_is_not_stored_as_a_cover() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(save_custom_cover(tmp.path(), "steam:1", b"\xff\xd8\xff jpeg").is_err());
    }

    #[test]
    fn an_app_id_that_is_not_digits_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let game = Game::new(GameSource::Steam, "../../etc/passwd", "Evil");
        assert!(fetch_missing(&[game], tmp.path(), 10).is_empty());
    }
}
