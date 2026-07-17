use chrono::{DateTime, Duration, Utc};
use reqwest::StatusCode;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    path::Path,
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
    time::Duration as StdDuration,
};

pub const RELEASES_API_URL: &str =
    "https://api.github.com/repos/Maxdelta/rendernorth-industrial/releases?per_page=20";
pub const RELEASES_PAGE_URL: &str = "https://github.com/Maxdelta/rendernorth-industrial/releases";
const STATE_KEY: &str = "update_check_state";
const AUTO_KEY: &str = "update_check_enabled";
const FREQUENCY_KEY: &str = "update_check_frequency";
const PRERELEASE_KEY: &str = "update_include_prerelease";
const REQUEST_TIMEOUT_SECONDS: u64 = 12;
static CHECK_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Identifier {
    Numeric(u64),
    Text(String),
}

impl SemanticVersion {
    fn parse(tag: &str) -> Result<Self, String> {
        let value = tag.trim().strip_prefix('v').unwrap_or(tag.trim());
        let value = value.split_once('+').map(|(left, _)| left).unwrap_or(value);
        let (core, prerelease) = value.split_once('-').map_or((value, ""), |parts| parts);
        let core = core
            .split('.')
            .map(|part| part.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| format!("malformed semantic version: {tag}"))?;
        if core.len() != 3 {
            return Err(format!("malformed semantic version: {tag}"));
        }
        let prerelease = if prerelease.is_empty() {
            Vec::new()
        } else {
            prerelease
                .split('.')
                .map(|part| {
                    if part.is_empty()
                        || !part
                            .chars()
                            .all(|character| character.is_ascii_alphanumeric() || character == '-')
                    {
                        return Err(format!("malformed semantic version: {tag}"));
                    }
                    if part.chars().all(|character| character.is_ascii_digit()) {
                        Ok(Identifier::Numeric(part.parse().map_err(|_| {
                            format!("malformed semantic version: {tag}")
                        })?))
                    } else {
                        Ok(Identifier::Text(part.to_ascii_lowercase()))
                    }
                })
                .collect::<Result<Vec<_>, String>>()?
        };
        Ok(Self {
            major: core[0],
            minor: core[1],
            patch: core[2],
            prerelease,
        })
    }

    fn is_prerelease(&self) -> bool {
        !self.prerelease.is_empty()
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let core =
            (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core != Ordering::Equal {
            return core;
        }
        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => {
                for (left, right) in self.prerelease.iter().zip(&other.prerelease) {
                    let order = match (left, right) {
                        (Identifier::Numeric(left), Identifier::Numeric(right)) => left.cmp(right),
                        (Identifier::Numeric(_), Identifier::Text(_)) => Ordering::Less,
                        (Identifier::Text(_), Identifier::Numeric(_)) => Ordering::Greater,
                        (Identifier::Text(left), Identifier::Text(right)) => left.cmp(right),
                    };
                    if order != Ordering::Equal {
                        return order;
                    }
                }
                self.prerelease.len().cmp(&other.prerelease.len())
            }
        }
    }
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatus {
    NotChecked,
    Checking,
    UpToDate,
    UpdateAvailable,
    CheckFailed,
    Offline,
    Skipped,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub installed_version: String,
    pub latest_version: Option<String>,
    pub latest_release_title: Option<String>,
    pub release_date: Option<String>,
    pub release_url: Option<String>,
    pub installer_url: Option<String>,
    pub portable_url: Option<String>,
    pub summary: Option<String>,
    pub last_checked: Option<String>,
    pub status: UpdateStatus,
    pub last_error: Option<String>,
    pub skipped_version: Option<String>,
    pub reminder_until: Option<String>,
    pub release_in_catalog: bool,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            installed_version: env!("CARGO_PKG_VERSION").into(),
            latest_version: None,
            latest_release_title: None,
            release_date: None,
            release_url: None,
            installer_url: None,
            portable_url: None,
            summary: None,
            last_checked: None,
            status: UpdateStatus::NotChecked,
            last_error: None,
            skipped_version: None,
            reminder_until: None,
            release_in_catalog: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckFrequency {
    Daily,
    Weekly,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferences {
    pub automatically_check: bool,
    pub frequency: CheckFrequency,
    pub include_prerelease: bool,
}

impl Default for UpdatePreferences {
    fn default() -> Self {
        let include_prerelease = crate::release::catalog()
            .ok()
            .and_then(|catalog| {
                catalog
                    .releases
                    .into_iter()
                    .find(|release| release.version == catalog.public_version)
                    .map(|release| release.channel != "Stable")
            })
            .unwrap_or(true);
        Self {
            automatically_check: true,
            frequency: CheckFrequency::Daily,
            include_prerelease,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

struct CheckGuard(&'static AtomicBool);

impl CheckGuard {
    fn acquire(flag: &'static AtomicBool) -> Option<Self> {
        flag.compare_exchange(false, true, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
            .ok()
            .map(|_| Self(flag))
    }
}

impl Drop for CheckGuard {
    fn drop(&mut self) {
        self.0.store(false, AtomicOrdering::SeqCst);
    }
}

fn meta(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM app_meta WHERE key=?1", [key], |row| {
        row.get(0)
    })
    .optional()
    .map_err(|error| format!("failed to read update setting: {error}"))
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_meta(key,value) VALUES(?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        (key, value),
    )
    .map_err(|error| format!("failed to save update setting: {error}"))?;
    Ok(())
}

pub fn preferences(conn: &Connection) -> Result<UpdatePreferences, String> {
    let defaults = UpdatePreferences::default();
    Ok(UpdatePreferences {
        automatically_check: meta(conn, AUTO_KEY)?
            .map(|value| value != "false")
            .unwrap_or(defaults.automatically_check),
        frequency: match meta(conn, FREQUENCY_KEY)?.as_deref() {
            Some("weekly") => CheckFrequency::Weekly,
            Some("never") => CheckFrequency::Never,
            _ => CheckFrequency::Daily,
        },
        include_prerelease: meta(conn, PRERELEASE_KEY)?
            .map(|value| value != "false")
            .unwrap_or(defaults.include_prerelease),
    })
}

pub fn save_preferences(
    conn: &Connection,
    preferences: &UpdatePreferences,
) -> Result<UpdatePreferences, String> {
    set_meta(
        conn,
        AUTO_KEY,
        if preferences.automatically_check {
            "true"
        } else {
            "false"
        },
    )?;
    set_meta(
        conn,
        FREQUENCY_KEY,
        match preferences.frequency {
            CheckFrequency::Daily => "daily",
            CheckFrequency::Weekly => "weekly",
            CheckFrequency::Never => "never",
        },
    )?;
    set_meta(
        conn,
        PRERELEASE_KEY,
        if preferences.include_prerelease {
            "true"
        } else {
            "false"
        },
    )?;
    let mut state = state(conn)?;
    if !preferences.automatically_check || preferences.frequency == CheckFrequency::Never {
        state.status = UpdateStatus::Disabled;
        save_state(conn, &state)?;
    } else if state.status == UpdateStatus::Disabled {
        state.status = status_for_cached(&state, Utc::now());
        save_state(conn, &state)?;
    }
    Ok(preferences.clone())
}

fn load_state(conn: &Connection) -> Result<UpdateState, String> {
    match meta(conn, STATE_KEY)? {
        Some(value) => serde_json::from_str(&value)
            .map_err(|error| format!("invalid cached update state: {error}")),
        None => Ok(UpdateState::default()),
    }
}

fn save_state(conn: &Connection, state: &UpdateState) -> Result<(), String> {
    let value = serde_json::to_string(state).map_err(|error| error.to_string())?;
    set_meta(conn, STATE_KEY, &value)
}

pub fn state(conn: &Connection) -> Result<UpdateState, String> {
    let mut state = load_state(conn)?;
    let preferences = preferences(conn)?;
    state.installed_version = env!("CARGO_PKG_VERSION").into();
    state.status =
        if !preferences.automatically_check || preferences.frequency == CheckFrequency::Never {
            UpdateStatus::Disabled
        } else {
            status_for_cached(&state, Utc::now())
        };
    Ok(state)
}

fn status_for_cached(state: &UpdateState, now: DateTime<Utc>) -> UpdateStatus {
    let Some(latest) = state.latest_version.as_deref() else {
        return state.status.clone();
    };
    let Ok(installed) = SemanticVersion::parse(&state.installed_version) else {
        return UpdateStatus::CheckFailed;
    };
    let Ok(latest_semver) = SemanticVersion::parse(latest) else {
        return UpdateStatus::CheckFailed;
    };
    if latest_semver <= installed {
        return UpdateStatus::UpToDate;
    }
    if state.skipped_version.as_deref() == Some(latest) {
        return UpdateStatus::Skipped;
    }
    if state
        .reminder_until
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc) > now)
        .unwrap_or(false)
    {
        return UpdateStatus::Skipped;
    }
    UpdateStatus::UpdateAvailable
}

pub fn should_check(
    state: &UpdateState,
    preferences: &UpdatePreferences,
    now: DateTime<Utc>,
) -> bool {
    if !preferences.automatically_check || preferences.frequency == CheckFrequency::Never {
        return false;
    }
    let interval = match preferences.frequency {
        CheckFrequency::Daily => Duration::hours(24),
        CheckFrequency::Weekly => Duration::days(7),
        CheckFrequency::Never => return false,
    };
    state
        .last_checked
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| now.signed_duration_since(value.with_timezone(&Utc)) >= interval)
        .unwrap_or(true)
}

fn check_due(
    manual: bool,
    state: &UpdateState,
    preferences: &UpdatePreferences,
    now: DateTime<Utc>,
) -> bool {
    manual || should_check(state, preferences, now)
}

fn release_summary(body: Option<&str>) -> Option<String> {
    let paragraph = body?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ");
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.chars().take(320).collect())
    }
}

fn asset_urls(assets: &[GithubAsset]) -> (Option<String>, Option<String>) {
    let installer = assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".exe")
                && name.contains("rendernorth")
                && (name.contains("setup") || name.contains("installer"))
        })
        .map(|asset| asset.browser_download_url.clone());
    let portable = assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".zip") && name.contains("rendernorth") && name.contains("portable")
        })
        .map(|asset| asset.browser_download_url.clone());
    (installer, portable)
}

fn select_release(
    body: &str,
    include_prerelease: bool,
) -> Result<Option<(GithubRelease, SemanticVersion)>, String> {
    let releases: Vec<GithubRelease> = serde_json::from_str(body)
        .map_err(|error| format!("invalid GitHub release response: {error}"))?;
    let mut candidates = releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter(|release| {
            release
                .name
                .as_deref()
                .map(|name| name.to_ascii_lowercase().contains("rendernorth industrial"))
                .unwrap_or(false)
                || release
                    .assets
                    .iter()
                    .any(|asset| asset.name.to_ascii_lowercase().contains("rendernorth"))
        })
        .filter_map(|release| {
            let version = SemanticVersion::parse(&release.tag_name).ok()?;
            if (release.prerelease || version.is_prerelease()) && !include_prerelease {
                return None;
            }
            Some((release, version))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.1.cmp(&left.1));
    Ok(candidates.into_iter().next())
}

fn apply_success(
    mut cached: UpdateState,
    release: Option<(GithubRelease, SemanticVersion)>,
    now: DateTime<Utc>,
) -> Result<UpdateState, String> {
    cached.installed_version = env!("CARGO_PKG_VERSION").into();
    cached.last_checked = Some(now.to_rfc3339());
    cached.last_error = None;
    cached.reminder_until = cached.reminder_until.filter(|value| {
        DateTime::parse_from_rfc3339(value)
            .map(|time| time > now)
            .unwrap_or(false)
    });
    if let Some((release, version)) = release {
        let version_text = release.tag_name.trim().trim_start_matches('v').to_string();
        let (installer_url, portable_url) = asset_urls(&release.assets);
        cached.latest_version = Some(version_text.clone());
        cached.latest_release_title = release.name.or_else(|| Some(release.tag_name.clone()));
        cached.release_date = release.published_at;
        cached.release_url = Some(release.html_url);
        cached.installer_url = installer_url;
        cached.portable_url = portable_url;
        cached.summary = release_summary(release.body.as_deref());
        cached.release_in_catalog = crate::release::catalog()?
            .releases
            .iter()
            .any(|entry| SemanticVersion::parse(&entry.version).ok().as_ref() == Some(&version));
    } else {
        cached.latest_version = Some(cached.installed_version.clone());
        cached.latest_release_title = None;
        cached.release_date = None;
        cached.release_url = Some(RELEASES_PAGE_URL.into());
        cached.installer_url = None;
        cached.portable_url = None;
        cached.summary = None;
        cached.release_in_catalog = true;
    }
    cached.status = status_for_cached(&cached, now);
    Ok(cached)
}

fn apply_failure(
    mut cached: UpdateState,
    status: UpdateStatus,
    error: String,
    now: DateTime<Utc>,
) -> UpdateState {
    cached.installed_version = env!("CARGO_PKG_VERSION").into();
    cached.last_checked = Some(now.to_rfc3339());
    cached.status = status;
    cached.last_error = Some(error);
    cached
}

async fn fetch_releases() -> Result<String, (UpdateStatus, String)> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .user_agent(format!(
            "RenderNorth-Industrial/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|error| (UpdateStatus::CheckFailed, error.to_string()))?;
    let endpoint = option_env!("RENDERNORTH_RELEASES_API_URL").unwrap_or(RELEASES_API_URL);
    let response = client
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| {
            transport_failure(error.is_connect(), error.is_timeout(), &error.to_string())
        })?;
    let status = response.status();
    classify_http_status(status)?;
    response
        .text()
        .await
        .map_err(|error| (UpdateStatus::CheckFailed, error.to_string()))
}

fn transport_failure(connect: bool, timeout: bool, detail: &str) -> (UpdateStatus, String) {
    if connect {
        (
            UpdateStatus::Offline,
            "GitHub could not be reached; cached update information was preserved.".into(),
        )
    } else if timeout {
        (
            UpdateStatus::CheckFailed,
            "The update check timed out; cached update information was preserved.".into(),
        )
    } else {
        (
            UpdateStatus::CheckFailed,
            format!("Update check failed: {detail}"),
        )
    }
}

fn classify_http_status(status: StatusCode) -> Result<(), (UpdateStatus, String)> {
    if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
        return Err((
            UpdateStatus::CheckFailed,
            "GitHub rate limit reached; try again later. Cached update information was preserved."
                .into(),
        ));
    }
    if !status.is_success() {
        return Err((
            UpdateStatus::CheckFailed,
            format!("GitHub release service returned HTTP {status}. Cached update information was preserved."),
        ));
    }
    Ok(())
}

pub async fn check(db_path: &Path, manual: bool) -> Result<UpdateState, String> {
    let Some(_guard) = CheckGuard::acquire(&CHECK_IN_PROGRESS) else {
        let conn = Connection::open(db_path).map_err(|error| error.to_string())?;
        return state(&conn);
    };
    let conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    let preferences = preferences(&conn)?;
    let cached = load_state(&conn)?;
    if !check_due(manual, &cached, &preferences, Utc::now()) {
        return state(&conn);
    }
    let mut checking = cached.clone();
    checking.status = UpdateStatus::Checking;
    save_state(&conn, &checking)?;
    drop(conn);

    let now = Utc::now();
    let result = match fetch_releases().await {
        Ok(body) => match select_release(&body, preferences.include_prerelease) {
            Ok(release) => apply_success(cached, release, now)?,
            Err(error) => apply_failure(cached, UpdateStatus::CheckFailed, error, now),
        },
        Err((status, error)) => apply_failure(cached, status, error, now),
    };
    let conn = Connection::open(db_path).map_err(|error| error.to_string())?;
    save_state(&conn, &result)?;
    Ok(result)
}

pub fn remind_later(conn: &Connection) -> Result<UpdateState, String> {
    let mut state = load_state(conn)?;
    state.reminder_until = Some((Utc::now() + Duration::hours(24)).to_rfc3339());
    state.status = UpdateStatus::Skipped;
    save_state(conn, &state)?;
    Ok(state)
}

pub fn skip_current(conn: &Connection) -> Result<UpdateState, String> {
    let mut state = load_state(conn)?;
    state.skipped_version = state.latest_version.clone();
    state.reminder_until = None;
    state.status = UpdateStatus::Skipped;
    save_state(conn, &state)?;
    Ok(state)
}

pub fn clear_skipped(conn: &Connection) -> Result<UpdateState, String> {
    let mut state = load_state(conn)?;
    state.skipped_version = None;
    state.reminder_until = None;
    state.status = status_for_cached(&state, Utc::now());
    save_state(conn, &state)?;
    Ok(state)
}

pub fn safe_diagnostics(conn: &Connection) -> Result<serde_json::Value, String> {
    let state = state(conn)?;
    let preferences = preferences(conn)?;
    Ok(serde_json::json!({
        "enabled": preferences.automatically_check,
        "frequency": preferences.frequency,
        "includePrerelease": preferences.include_prerelease,
        "lastChecked": state.last_checked,
        "latestKnownVersion": state.latest_version,
        "status": state.status,
        "skippedVersion": state.skipped_version,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE app_meta(key TEXT PRIMARY KEY,value TEXT)", [])
            .unwrap();
        conn
    }

    fn release(tag: &str, draft: bool, prerelease: bool, assets: &[(&str, &str)]) -> String {
        serde_json::json!([{
            "tag_name": tag,
            "name": format!("RenderNorth Industrial {tag}"),
            "body": "A concise public release summary.",
            "draft": draft,
            "prerelease": prerelease,
            "html_url": format!("https://github.com/Maxdelta/rendernorth-industrial/releases/tag/{tag}"),
            "published_at": "2026-07-16T12:00:00Z",
            "assets": assets.iter().map(|(name,url)| serde_json::json!({
                "name": name, "browser_download_url": url
            })).collect::<Vec<_>>()
        }])
        .to_string()
    }

    #[test]
    fn semantic_versions_handle_prefix_prerelease_and_ordering() {
        assert_eq!(
            SemanticVersion::parse("v0.1.0").unwrap(),
            SemanticVersion::parse("0.1.0").unwrap()
        );
        assert!(
            SemanticVersion::parse("v0.2.0-beta.1").unwrap()
                > SemanticVersion::parse("0.1.9").unwrap()
        );
        assert!(
            SemanticVersion::parse("0.2.0").unwrap()
                > SemanticVersion::parse("0.2.0-beta.1").unwrap()
        );
        assert!(SemanticVersion::parse("not-a-version").is_err());
    }

    #[test]
    fn equal_newer_and_installed_newer_have_deterministic_states() {
        let now = Utc::now();
        for (latest, expected) in [
            ("0.1.3", UpdateStatus::UpToDate),
            ("0.1.4", UpdateStatus::UpdateAvailable),
            ("0.1.2", UpdateStatus::UpToDate),
        ] {
            let mut state = UpdateState::default();
            state.latest_version = Some(latest.into());
            assert_eq!(status_for_cached(&state, now), expected);
        }
        assert!(
            SemanticVersion::parse("v0.1.1").unwrap()
                > SemanticVersion::parse("0.1.0").unwrap()
        );
    }

    #[test]
    fn drafts_malformed_tags_and_excluded_prereleases_are_ignored() {
        assert!(select_release(&release("v0.2.0", true, false, &[]), true)
            .unwrap()
            .is_none());
        assert!(select_release(&release("latest", false, false, &[]), true)
            .unwrap()
            .is_none());
        assert!(
            select_release(&release("v0.2.0-beta.1", false, true, &[]), false)
                .unwrap()
                .is_none()
        );
        assert!(
            select_release(&release("v0.2.0-beta.1", false, true, &[]), true)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn installer_is_preferred_and_portable_is_available_as_fallback() {
        let assets = [
            (
                "RenderNorth-Industrial-0.2.0-Windows-Portable.zip",
                "https://example/portable",
            ),
            (
                "RenderNorth.Industrial_0.2.0_x64-setup.exe",
                "https://example/installer",
            ),
        ];
        let (installer, portable) = asset_urls(
            &serde_json::from_value::<Vec<GithubAsset>>(serde_json::json!(assets
                .iter()
                .map(|(name, url)| serde_json::json!({"name":name,"browser_download_url":url}))
                .collect::<Vec<_>>()))
            .unwrap(),
        );
        assert_eq!(installer.as_deref(), Some("https://example/installer"));
        assert_eq!(portable.as_deref(), Some("https://example/portable"));
        let only_portable = [GithubAsset {
            name: assets[0].0.into(),
            browser_download_url: assets[0].1.into(),
        }];
        assert_eq!(asset_urls(&only_portable).0, None);
        assert_eq!(
            asset_urls(&only_portable).1.as_deref(),
            Some("https://example/portable")
        );
    }

    #[test]
    fn release_without_installer_uses_release_page_without_fabricating_asset() {
        let selected = select_release(&release("v0.2.0", false, false, &[]), true).unwrap();
        let state = apply_success(UpdateState::default(), selected, Utc::now()).unwrap();
        assert_eq!(state.status, UpdateStatus::UpdateAvailable);
        assert!(state.installer_url.is_none());
        assert!(state.release_url.is_some());
    }

    #[test]
    fn cached_result_is_preserved_after_offline_timeout_rate_limit_and_invalid_json() {
        let mut cached = UpdateState::default();
        cached.latest_version = Some("0.2.0".into());
        cached.release_url = Some("https://example/release".into());
        for (status, message) in [
            (UpdateStatus::Offline, "offline"),
            (UpdateStatus::CheckFailed, "timeout"),
            (UpdateStatus::CheckFailed, "rate limit"),
        ] {
            let failed = apply_failure(cached.clone(), status, message.into(), Utc::now());
            assert_eq!(failed.latest_version.as_deref(), Some("0.2.0"));
            assert_eq!(
                failed.release_url.as_deref(),
                Some("https://example/release")
            );
        }
        assert!(select_release("{invalid", true).is_err());
        assert_eq!(
            transport_failure(true, false, "connect").0,
            UpdateStatus::Offline
        );
        assert!(transport_failure(false, true, "timeout")
            .1
            .contains("timed out"));
        assert!(classify_http_status(StatusCode::FORBIDDEN)
            .unwrap_err()
            .1
            .contains("rate limit"));
        assert!(classify_http_status(StatusCode::INTERNAL_SERVER_ERROR).is_err());
    }

    #[test]
    fn automatic_interval_manual_override_and_disabled_behavior_are_deterministic() {
        let now = Utc::now();
        let preferences = UpdatePreferences::default();
        assert!(should_check(&UpdateState::default(), &preferences, now));
        let mut recent = UpdateState::default();
        recent.last_checked = Some((now - Duration::hours(2)).to_rfc3339());
        assert!(!should_check(&recent, &preferences, now));
        recent.last_checked = Some((now - Duration::hours(25)).to_rfc3339());
        assert!(should_check(&recent, &preferences, now));
        let disabled = UpdatePreferences {
            automatically_check: false,
            ..preferences
        };
        assert!(!should_check(&recent, &disabled, now));
        assert!(check_due(true, &recent, &disabled, now));
        let conn = connection();
        save_preferences(&conn, &disabled).unwrap();
        assert_eq!(state(&conn).unwrap().status, UpdateStatus::Disabled);
    }

    #[test]
    fn concurrent_check_guard_deduplicates_requests() {
        static FLAG: AtomicBool = AtomicBool::new(false);
        let first = CheckGuard::acquire(&FLAG);
        assert!(first.is_some());
        assert!(CheckGuard::acquire(&FLAG).is_none());
        drop(first);
        assert!(CheckGuard::acquire(&FLAG).is_some());
    }

    #[test]
    fn remind_skip_newer_override_and_clear_are_persistent() {
        let conn = connection();
        let mut cached = UpdateState::default();
        cached.latest_version = Some("0.1.4".into());
        cached.status = UpdateStatus::UpdateAvailable;
        save_state(&conn, &cached).unwrap();
        assert_eq!(remind_later(&conn).unwrap().status, UpdateStatus::Skipped);
        let skipped = skip_current(&conn).unwrap();
        assert_eq!(skipped.skipped_version.as_deref(), Some("0.1.4"));
        let mut newer = skipped;
        newer.latest_version = Some("0.1.5".into());
        newer.reminder_until = None;
        save_state(&conn, &newer).unwrap();
        assert_eq!(state(&conn).unwrap().status, UpdateStatus::UpdateAvailable);
        assert!(clear_skipped(&conn).unwrap().skipped_version.is_none());
    }

    #[test]
    fn request_metadata_and_diagnostics_are_secret_free() {
        assert!(RELEASES_API_URL
            .starts_with("https://api.github.com/repos/Maxdelta/rendernorth-industrial/"));
        let conn = connection();
        let diagnostics = safe_diagnostics(&conn)
            .unwrap()
            .to_string()
            .to_ascii_lowercase();
        for forbidden in [
            "character",
            "inventory",
            "access_token",
            "refresh_token",
            "client_id",
            "machine",
        ] {
            assert!(!diagnostics.contains(forbidden));
        }
    }
}
