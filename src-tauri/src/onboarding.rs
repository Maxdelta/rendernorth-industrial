use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use std::{fs, path::{Path, PathBuf}};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

pub const WEBSITE_URL: &str = "https://rendernorth.com";
pub const GITHUB_URL: &str = "https://github.com/Maxdelta/rendernorth-industrial";
pub const ISSUES_URL: &str = "https://github.com/Maxdelta/rendernorth-industrial/issues";
pub const SUPPORT_URL: &str = "https://buymeacoffee.com/maxdelta";
pub const DISCORD_INVITE_URL: &str = "https://discord.gg/XycCz6ppx";
pub const REQUIRED_SDE_FILES: [&str; 4] = [
    "categories.jsonl",
    "groups.jsonl",
    "types.jsonl",
    "blueprints.jsonl",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdeFileStatus {
    pub name: String,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdeInspection {
    pub path: String,
    pub valid: bool,
    pub source_build: Option<String>,
    pub files: Vec<SdeFileStatus>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    pub application_name: String,
    pub version: String,
    pub build: String,
    pub git_commit: String,
    pub database_version: String,
    pub migration_version: i64,
    pub rust_version: String,
    pub website: String,
    pub github: String,
    pub issues: String,
    pub support: String,
    pub discord_invite: String,
}

fn value_as_label(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn find_build(value: &Value) -> Option<String> {
    const KEYS: [&str; 8] = [
        "buildNumber", "build_number", "build", "version", "release",
        "sdeVersion", "sde_version", "releaseVersion",
    ];
    match value {
        Value::Object(map) => {
            for key in KEYS {
                if let Some(label) = map.get(key).and_then(value_as_label) {
                    return Some(label);
                }
            }
            map.values().find_map(find_build)
        }
        Value::Array(values) => values.iter().find_map(find_build),
        _ => None,
    }
}

fn build_from_directory_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| {
            name.split(|character: char| !character.is_ascii_digit())
                .find(|part| part.len() >= 6)
                .map(str::to_string)
        })
}

pub fn detect_source_build(path: &Path) -> Option<String> {
    let metadata_path = path.join("_sde.jsonl");
    if let Ok(contents) = fs::read_to_string(metadata_path) {
        for line in contents.lines().filter(|line| !line.trim().is_empty()).take(20) {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if let Some(build) = find_build(&value) {
                    return Some(build);
                }
            }
        }
    }
    build_from_directory_name(path)
}

pub fn inspect_sde(path: &str) -> SdeInspection {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return SdeInspection {
            path: String::new(), valid: false, source_build: None, files: Vec::new(),
            error: Some("Select an extracted CCP JSONL SDE directory.".into()),
        };
    }
    let directory = Path::new(trimmed);
    if !directory.is_dir() {
        return SdeInspection {
            path: trimmed.into(), valid: false, source_build: None, files: Vec::new(),
            error: Some("The selected path is not a readable directory.".into()),
        };
    }
    let files: Vec<SdeFileStatus> = REQUIRED_SDE_FILES.iter().map(|name| SdeFileStatus {
        name: (*name).into(), present: directory.join(name).is_file(),
    }).collect();
    let missing: Vec<&str> = files.iter().filter(|file| !file.present).map(|file| file.name.as_str()).collect();
    SdeInspection {
        path: trimmed.into(),
        valid: missing.is_empty(),
        source_build: detect_source_build(directory),
        error: if missing.is_empty() { None } else { Some(format!("Missing required files: {}", missing.join(", "))) },
        files,
    }
}

pub fn pick_sde_directory(app: &AppHandle) -> Option<String> {
    app.dialog().file().blocking_pick_folder()
        .and_then(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().to_string())
}

pub fn about_info(conn: &Connection) -> Result<AboutInfo, String> {
    let migration_version = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |row| row.get(0),
    ).map_err(|error| format!("failed to read migration version: {error}"))?;
    Ok(AboutInfo {
        application_name: "RenderNorth Industrial".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        build: option_env!("RENDERNORTH_BUILD_UNIX").unwrap_or("unknown").into(),
        git_commit: option_env!("RENDERNORTH_GIT_COMMIT").unwrap_or("unknown").into(),
        database_version: format!("SQLite {}", rusqlite::version()),
        migration_version,
        rust_version: option_env!("RENDERNORTH_RUST_VERSION").unwrap_or("unknown").into(),
        website: WEBSITE_URL.into(), github: GITHUB_URL.into(), issues: ISSUES_URL.into(),
        support: SUPPORT_URL.into(), discord_invite: DISCORD_INVITE_URL.into(),
    })
}

fn diagnostics_value(conn: &Connection) -> Result<Value, String> {
    let about = about_info(conn)?;
    let characters = {
        let mut statement = conn.prepare(
            "SELECT c.character_id,c.name,c.enabled,c.authorization_status,c.scopes_granted,
                    a.last_success_at,a.status,b.last_success_at,b.status
             FROM characters c
             LEFT JOIN character_asset_sync_state a ON a.character_id=c.character_id
             LEFT JOIN character_blueprint_sync_state b ON b.character_id=c.character_id
             WHERE c.is_demo=0 ORDER BY c.name"
        ).map_err(|error| format!("failed to prepare diagnostics character query: {error}"))?;
        let rows = statement.query_map([], |row| Ok(json!({
            "characterId": row.get::<_, i64>(0)?, "name": row.get::<_, String>(1)?,
            "enabled": row.get::<_, i64>(2)? != 0, "authorizationStatus": row.get::<_, String>(3)?,
            "enabledScopes": row.get::<_, String>(4)?.split_whitespace().collect::<Vec<_>>(),
            "assetLastSync": row.get::<_, Option<String>>(5)?, "assetStatus": row.get::<_, Option<String>>(6)?,
            "blueprintLastSync": row.get::<_, Option<String>>(7)?, "blueprintStatus": row.get::<_, Option<String>>(8)?,
        }))).map_err(|error| format!("failed to read diagnostics characters: {error}"))?
            .collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())?;
        rows
    };
    let market = conn.query_row(
        "SELECT p.display_name,s.status,s.fetched_at,s.expires_at,s.order_count,s.page_count,s.last_error
         FROM market_profiles p LEFT JOIN market_refresh_state s ON s.profile_id=p.profile_id
         WHERE p.is_selected=1 LIMIT 1", [], |row| Ok(json!({
            "market": row.get::<_, String>(0)?, "status": row.get::<_, Option<String>>(1)?,
            "lastRefresh": row.get::<_, Option<String>>(2)?, "expiresAt": row.get::<_, Option<String>>(3)?,
            "orderCount": row.get::<_, Option<i64>>(4)?.unwrap_or(0), "pageCount": row.get::<_, Option<i64>>(5)?.unwrap_or(0),
            "lastError": row.get::<_, Option<String>>(6)?,
        }))).optional().map_err(|error| format!("failed to read diagnostics market state: {error}"))?;
    let latest_sde = conn.query_row(
        "SELECT source_build,imported_at,status,type_count,blueprint_count FROM sde_imports ORDER BY id DESC LIMIT 1",
        [], |row| Ok(json!({
            "sourceBuild": row.get::<_, Option<String>>(0)?, "importedAt": row.get::<_, String>(1)?,
            "status": row.get::<_, String>(2)?, "typeCount": row.get::<_, i64>(3)?, "blueprintCount": row.get::<_, i64>(4)?,
        })),
    ).optional().map_err(|error| format!("failed to read diagnostics SDE state: {error}"))?;
    Ok(json!({
        "privacy": "No access tokens, refresh tokens, Client IDs, credential material, or app secrets are included.",
        "os": { "family": std::env::consts::OS, "architecture": std::env::consts::ARCH },
        "application": about,
        "staticData": latest_sde,
        "connectedCharacters": characters,
        "market": market,
    }))
}

pub fn diagnostics_json(conn: &Connection) -> Result<String, String> {
    serde_json::to_string_pretty(&diagnostics_value(conn)?).map_err(|error| error.to_string())
}

pub fn export_diagnostics(app: &AppHandle, conn: &Connection) -> Result<Option<String>, String> {
    let suggested = format!("rendernorth-industrial-diagnostics-{}.json", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let Some(target) = app.dialog().file().set_file_name(&suggested).add_filter("JSON", &["json"]).blocking_save_file() else {
        return Ok(None);
    };
    let path: PathBuf = target.into_path().map_err(|error| format!("invalid diagnostics path: {error}"))?;
    fs::write(&path, diagnostics_json(conn)?).map_err(|error| format!("failed to write diagnostics: {error}"))?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("rni-onboarding-{label}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn sde_validation_lists_every_missing_required_file() {
        let path = temp_dir("missing");
        fs::write(path.join("types.jsonl"), "{}\n").unwrap();
        let result = inspect_sde(path.to_str().unwrap());
        assert!(!result.valid);
        assert_eq!(result.files.len(), 4);
        assert!(result.error.unwrap().contains("categories.jsonl"));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn sde_validation_detects_build_and_complete_bundle() {
        let path = temp_dir("eve-online-static-data-3430261-jsonl");
        for file in REQUIRED_SDE_FILES { fs::write(path.join(file), "{}\n").unwrap(); }
        fs::write(path.join("_sde.jsonl"), "{\"buildNumber\":3430261}\n").unwrap();
        let result = inspect_sde(path.to_str().unwrap());
        assert!(result.valid);
        assert_eq!(result.source_build.as_deref(), Some("3430261"));
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn diagnostics_are_explicitly_secret_free() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations(version INTEGER); INSERT INTO schema_migrations VALUES(18);
             CREATE TABLE characters(character_id INTEGER,name TEXT,enabled INTEGER,authorization_status TEXT,scopes_granted TEXT,is_demo INTEGER);
             CREATE TABLE character_asset_sync_state(character_id INTEGER,last_success_at TEXT,status TEXT);
             CREATE TABLE character_blueprint_sync_state(character_id INTEGER,last_success_at TEXT,status TEXT);
             CREATE TABLE market_profiles(profile_id INTEGER,display_name TEXT,is_selected INTEGER);
             CREATE TABLE market_refresh_state(profile_id INTEGER,status TEXT,fetched_at TEXT,expires_at TEXT,order_count INTEGER,page_count INTEGER,last_error TEXT);
             CREATE TABLE sde_imports(id INTEGER,source_build TEXT,imported_at TEXT,status TEXT,type_count INTEGER,blueprint_count INTEGER);
             INSERT INTO characters VALUES(1,'Pilot',1,'authorized','esi-assets.read_assets.v1',0);"
        ).unwrap();
        let report = diagnostics_json(&conn).unwrap();
        assert!(report.contains("\"applicationName\": \"RenderNorth Industrial\""));
        assert!(report.contains(SUPPORT_URL));
        assert!(report.contains(DISCORD_INVITE_URL));
        assert!(report.contains("esi-assets.read_assets.v1"));
        assert!(!report.contains("refresh_token"));
        assert!(!report.contains("access_token"));
        assert!(!report.contains("client_id"));
        assert!(!report.contains("client_secret"));
    }
}
