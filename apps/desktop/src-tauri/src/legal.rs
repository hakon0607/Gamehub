//! Terms, consent and the data rights the GDPR gives the user.
//!
//! Three things live here. The version of the terms and the privacy policy
//! the app ships with; what the user has accepted and consented to, with the
//! time and the version, so both sides can see what was agreed; and the two
//! rights the user should never have to ask a human for — a copy of
//! everything GameHub holds on their PC, and deletion.
//!
//! Consent for the usage statistics is separate from accepting the terms, on
//! purpose: bundling them would not be a valid consent under the GDPR, and
//! the Norwegian Electronic Communications Act § 3-15 requires a consent of
//! the same standard before an identifier may be stored on and read back
//! from the user's device.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::state::AppState;

/// Bumped whenever the terms or the privacy policy change in a way that
/// matters. The app asks again when the accepted version is not this one.
pub const LEGAL_VERSION: &str = "1.0";
/// The date printed in the documents, shown next to the version.
pub const LEGAL_DATE: &str = "2026-09-12";

/// What the user has agreed to, stored with the rest of the settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct LegalSettings {
    /// The terms version the user accepted, empty until they have.
    pub terms_version: String,
    /// When they accepted it (ISO 8601), for the record.
    pub terms_accepted_at: String,
    /// Consent to the anonymous usage statistics. Off until given.
    pub stats_consent: bool,
    /// Which version of the privacy policy was on screen when they chose.
    pub stats_consent_version: String,
    /// When the choice was last made, either way.
    pub stats_consent_at: String,
}

impl Default for LegalSettings {
    fn default() -> Self {
        Self {
            terms_version: String::new(),
            terms_accepted_at: String::new(),
            stats_consent: false,
            stats_consent_version: String::new(),
            stats_consent_at: String::new(),
        }
    }
}

/// What the front end needs to draw the Terms of Service page and to decide
/// whether the first-run screen has to appear.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalStatus {
    pub version: String,
    pub date: String,
    pub accepted_version: String,
    pub accepted_at: String,
    /// True when the shipped version is newer than the accepted one.
    pub needs_acceptance: bool,
    pub stats_consent: bool,
    pub stats_consent_at: String,
    pub stats_consent_version: String,
    /// Shown so the user can quote it in an access or deletion request.
    /// Empty when statistics are off — no identifier exists then.
    pub install_id: String,
    pub data_folder: String,
}

pub fn status_of(settings: &crate::store::Settings) -> LegalStatus {
    LegalStatus {
        version: LEGAL_VERSION.into(),
        date: LEGAL_DATE.into(),
        accepted_version: settings.legal.terms_version.clone(),
        accepted_at: settings.legal.terms_accepted_at.clone(),
        needs_acceptance: settings.legal.terms_version != LEGAL_VERSION,
        stats_consent: settings.legal.stats_consent,
        stats_consent_at: settings.legal.stats_consent_at.clone(),
        stats_consent_version: settings.legal.stats_consent_version.clone(),
        install_id: if settings.legal.stats_consent { settings.install_id.clone() } else { String::new() },
        data_folder: String::new(),
    }
}

#[tauri::command]
pub fn legal_status(state: State<'_, Arc<AppState>>) -> LegalStatus {
    let mut status = status_of(&state.settings());
    status.data_folder = state.paths.settings().parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    status
}

/// Records that the user accepted the terms shown to them.
#[tauri::command]
pub fn accept_terms(state: State<'_, Arc<AppState>>) -> LegalStatus {
    {
        let mut inner = state.inner.lock();
        inner.settings.legal.terms_version = LEGAL_VERSION.into();
        inner.settings.legal.terms_accepted_at = gamehub_detect::now_iso8601();
    }
    state.persist_settings();
    legal_status(state)
}

/// Turns the usage statistics on or off and records when, and against which
/// version of the privacy policy. Turning it on is what creates the
/// installation id; turning it off never sends anything again.
#[tauri::command]
pub fn set_stats_consent(state: State<'_, Arc<AppState>>, consent: bool) -> LegalStatus {
    {
        let mut inner = state.inner.lock();
        inner.settings.legal.stats_consent = consent;
        inner.settings.legal.stats_consent_version = LEGAL_VERSION.into();
        inner.settings.legal.stats_consent_at = gamehub_detect::now_iso8601();
    }
    state.persist_settings();
    legal_status(state)
}

/// Asks the server to delete everything stored under this installation id,
/// then forgets the id. Consent is switched off first, so nothing is sent
/// again while the request is on its way.
#[tauri::command]
pub async fn forget_statistics(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let state = state.inner().clone();
    let id = {
        let mut inner = state.inner.lock();
        inner.settings.legal.stats_consent = false;
        inner.settings.legal.stats_consent_version = LEGAL_VERSION.into();
        inner.settings.legal.stats_consent_at = gamehub_detect::now_iso8601();
        std::mem::take(&mut inner.settings.install_id)
    };
    state.persist_settings();
    if id.is_empty() {
        return Ok(true);
    }
    let deleted = tauri::async_runtime::spawn_blocking(move || crate::telemetry::forget(&id))
        .await
        .unwrap_or(false);
    Ok(deleted)
}

/// Everything GameHub holds about the user on this PC, as one JSON file.
/// This is the portability right, answered by the app instead of by email.
#[tauri::command]
pub async fn export_my_data(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || write_export(&app, &state))
        .await
        .map_err(|e| e.to_string())?
}

fn write_export(app: &AppHandle, state: &Arc<AppState>) -> Result<String, String> {
    let (settings, library, activity, clips, screenshots, freezes, clipboard, bindings) = {
        let inner = state.inner.lock();
        (
            inner.settings.clone(),
            inner.library.clone(),
            inner.activity.clone(),
            inner.clips.clone(),
            inner.screenshots.clone(),
            inner.freezes.clone(),
            inner.clipboard.clone(),
            inner.bindings.clone(),
        )
    };
    // The export is for the user, but it must not hand out secrets that were
    // typed into the app: API keys stay behind.
    let mut settings = settings;
    settings.metadata = crate::store::MetadataKeys::default();
    settings.ai.api_key = String::new();

    let body = serde_json::json!({
        "exportedAt": gamehub_detect::now_iso8601(),
        "appVersion": app.package_info().version.to_string(),
        "legalVersion": LEGAL_VERSION,
        // No prose here: the app speaks ten languages and this file is read
        // by the user, so what it contains is explained in the app and in
        // the privacy policy instead.
        "about": "https://github.com/hakon0607/Gamehub",
        "settings": settings,
        "library": library,
        "activity": activity,
        "clips": clips,
        "screenshots": screenshots,
        "freezePoints": freezes,
        "clipboardHistory": clipboard,
        "shortcuts": bindings,
    });

    let dir = export_dir(app);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = gamehub_detect::now_iso8601().replace(':', "-");
    let path = dir.join(format!("gamehub-my-data-{stamp}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn export_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .document_dir()
        .map(|docs| docs.join("GameHub"))
        .or_else(|_| app.path().app_data_dir().map(|d| d.join("export")))
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Deletes the local data files: library, activity, clips index, screenshots
/// index, freeze points, clipboard and shortcuts. Settings survive so the
/// app still knows the language and that the terms were accepted; the media
/// files themselves are left alone, because the user may want the videos.
#[tauri::command]
pub fn delete_local_data(state: State<'_, Arc<AppState>>) -> Result<u32, String> {
    let paths = &state.paths;
    let files = [
        paths.library(),
        paths.activity(),
        paths.clips_index(),
        paths.screenshots_index(),
        paths.freezes_index(),
        paths.clipboard(),
        paths.shortcuts(),
    ];
    let mut removed = 0;
    for file in files {
        if file.exists() && std::fs::remove_file(&file).is_ok() {
            removed += 1;
        }
    }
    {
        let mut inner = state.inner.lock();
        inner.library.clear();
        inner.running.clear();
        inner.activity = Default::default();
        inner.clips = Default::default();
        inner.screenshots = Default::default();
        inner.freezes = Default::default();
        inner.clipboard = Default::default();
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Settings;

    #[test]
    fn a_fresh_install_has_accepted_nothing_and_consented_to_nothing() {
        let settings = Settings::default();
        let status = status_of(&settings);
        assert!(status.needs_acceptance, "the terms have to be shown once");
        assert!(!status.stats_consent, "statistics are off until the user says yes");
        assert_eq!(status.install_id, "", "no identifier is shown while consent is off");
    }

    #[test]
    fn accepting_one_version_does_not_accept_the_next() {
        let mut settings = Settings::default();
        settings.legal.terms_version = "0.9".into();
        assert!(status_of(&settings).needs_acceptance);
        settings.legal.terms_version = LEGAL_VERSION.into();
        assert!(!status_of(&settings).needs_acceptance);
    }

    #[test]
    fn the_install_id_is_only_shown_once_consent_exists() {
        let mut settings = Settings::default();
        settings.install_id = "abc".into();
        assert_eq!(status_of(&settings).install_id, "");
        settings.legal.stats_consent = true;
        assert_eq!(status_of(&settings).install_id, "abc");
    }
}
