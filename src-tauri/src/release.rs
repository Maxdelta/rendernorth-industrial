use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const RELEASES_JSON: &str = include_str!("../../src/data/releases.json");
#[cfg(test)]
const CHANGELOG: &str = include_str!("../../CHANGELOG.md");
#[cfg(test)]
const PACKAGE_JSON: &str = include_str!("../../package.json");
#[cfg(test)]
const TAURI_CONFIG: &str = include_str!("../tauri.conf.json");
#[cfg(test)]
const APP_SOURCE: &str = include_str!("../../src/App.tsx");
#[cfg(test)]
const WHATS_NEW_SOURCE: &str = include_str!("../../src/pages/WhatsNew.tsx");
#[cfg(test)]
const RELEASE_HISTORY_SOURCE: &str = include_str!("../../src/lib/releaseHistory.ts");
const LAST_VIEWED_KEY: &str = "release_notes_last_viewed";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseCatalog {
    pub public_version: String,
    pub releases: Vec<ReleaseEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ReleaseEntry {
    pub version: String,
    pub name: String,
    pub date: String,
    pub channel: String,
    pub summary: String,
    #[serde(default)]
    pub added: Vec<String>,
    #[serde(default)]
    pub changed: Vec<String>,
    #[serde(default)]
    pub fixed: Vec<String>,
    #[serde(default)]
    pub known_issues: Vec<String>,
    #[serde(default)]
    pub security: Vec<String>,
    #[serde(default)]
    pub deprecated: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
    #[serde(default)]
    pub internal_checkpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseViewState {
    pub current_version: String,
    pub last_viewed_version: Option<String>,
    pub unseen: bool,
}

pub fn catalog() -> Result<ReleaseCatalog, String> {
    let catalog: ReleaseCatalog = serde_json::from_str(RELEASES_JSON)
        .map_err(|error| format!("invalid release catalog: {error}"))?;
    if !catalog.releases.iter().any(|release| release.version == catalog.public_version) {
        return Err("release catalog does not contain its public version".into());
    }
    for pair in catalog.releases.windows(2) {
        if compare_versions(&pair[0].version, &pair[1].version)?.is_lt() {
            return Err("release catalog must be sorted newest first".into());
        }
    }
    Ok(catalog)
}

pub fn compare_versions(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    fn parse(value: &str) -> Result<[u64; 3], String> {
        let parts = value.split('.').map(str::parse::<u64>).collect::<Result<Vec<_>, _>>()
            .map_err(|_| format!("invalid semantic version: {value}"))?;
        if parts.len() != 3 {
            return Err(format!("invalid semantic version: {value}"));
        }
        Ok([parts[0], parts[1], parts[2]])
    }
    Ok(parse(left)?.cmp(&parse(right)?))
}

pub fn view_state(conn: &Connection) -> Result<ReleaseViewState, String> {
    let catalog = catalog()?;
    let last_viewed_version = conn.query_row(
        "SELECT value FROM app_meta WHERE key=?1", [LAST_VIEWED_KEY], |row| row.get(0),
    ).optional().map_err(|error| format!("failed to read release viewed state: {error}"))?;
    let unseen = match last_viewed_version.as_deref() {
        Some(version) => compare_versions(&catalog.public_version, version)?.is_gt(),
        None => true,
    };
    Ok(ReleaseViewState { current_version: catalog.public_version, last_viewed_version, unseen })
}

pub fn mark_current_viewed(conn: &Connection) -> Result<ReleaseViewState, String> {
    let current = catalog()?.public_version;
    conn.execute(
        "INSERT INTO app_meta(key,value) VALUES(?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        (LAST_VIEWED_KEY, current),
    ).map_err(|error| format!("failed to save release viewed state: {error}"))?;
    view_state(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE app_meta(key TEXT PRIMARY KEY,value TEXT)", []).unwrap();
        conn
    }

    #[test]
    fn semantic_version_parsing_and_comparison_is_deterministic() {
        assert!(compare_versions("0.1.1", "0.1.0").unwrap().is_gt());
        assert!(compare_versions("0.2.0", "0.1.9").unwrap().is_gt());
        assert!(compare_versions("1.0.0", "0.99.99").unwrap().is_gt());
        assert!(compare_versions("0.1", "0.1.0").is_err());
    }

    #[test]
    fn releases_are_newest_first_and_old_entries_remain_accessible() {
        let catalog = catalog().unwrap();
        assert!(!catalog.releases.is_empty());
        for pair in catalog.releases.windows(2) {
            assert!(!compare_versions(&pair[0].version, &pair[1].version).unwrap().is_lt());
        }
        assert!(catalog.releases.iter().any(|release| release.version == catalog.public_version));
    }

    #[test]
    fn current_version_matches_all_public_package_metadata_and_changelog() {
        let catalog = catalog().unwrap();
        assert_eq!(catalog.public_version, env!("CARGO_PKG_VERSION"));
        let package: Value = serde_json::from_str(PACKAGE_JSON).unwrap();
        let tauri: Value = serde_json::from_str(TAURI_CONFIG).unwrap();
        assert_eq!(package["version"], catalog.public_version);
        assert_eq!(tauri["version"], catalog.public_version);
        assert!(CHANGELOG.contains(&format!("## [{}]", catalog.public_version)));
    }

    #[test]
    fn release_content_is_user_facing_categorized_and_secret_free() {
        let serialized = RELEASES_JSON.to_ascii_lowercase();
        let current = &catalog().unwrap().releases[0];
        assert!(!current.added.is_empty());
        assert!(!current.changed.is_empty());
        assert!(!current.fixed.is_empty());
        assert!(!current.known_issues.is_empty());
        assert!(!current.name.contains("RNI-"));
        assert!(!current.summary.contains("Sprint"));
        for forbidden in ["access_token", "refresh_token", "client_secret", "credential contents"] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    fn category_visibility_is_driven_only_by_populated_content() {
        let current = &catalog().unwrap().releases[0];
        let visible = [
            ("Added", !current.added.is_empty()),
            ("Changed", !current.changed.is_empty()),
            ("Fixed", !current.fixed.is_empty()),
            ("Known Issues", !current.known_issues.is_empty()),
            ("Security & Privacy", !current.security.is_empty()),
            ("Deprecated", !current.deprecated.is_empty()),
            ("Removed", !current.removed.is_empty()),
        ].into_iter().filter(|(_, populated)| *populated).map(|(name, _)| name).collect::<Vec<_>>();
        assert!(visible.contains(&"Known Issues"));
        assert!(!visible.contains(&"Deprecated"));
        assert!(!visible.contains(&"Removed"));
        assert!(RELEASE_HISTORY_SOURCE.contains(r#"{ key: "security", label: "Security & Privacy" }"#));
        assert!(!RELEASE_HISTORY_SOURCE.contains(r#"{ key: "security", label: "Security" }"#));
    }

    #[test]
    fn unseen_and_viewed_release_state_persists_without_blocking_startup() {
        let conn = connection();
        assert!(view_state(&conn).unwrap().unseen);
        assert!(!mark_current_viewed(&conn).unwrap().unseen);
        assert!(!view_state(&conn).unwrap().unseen);
        assert!(!APP_SOURCE.contains("<dialog"));
        assert!(!APP_SOURCE.contains("release modal"));
    }

    #[test]
    fn whats_new_page_keeps_internal_checkpoint_hidden_and_history_expandable() {
        assert!(WHATS_NEW_SOURCE.contains("details className=\"release-previous\""));
        assert!(WHATS_NEW_SOURCE.contains("markCurrentReleaseViewed"));
        assert!(!WHATS_NEW_SOURCE.contains("internalCheckpoint"));
        assert!(!WHATS_NEW_SOURCE.contains("Sprint"));
    }
}
