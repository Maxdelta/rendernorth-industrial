use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub character_id: i64,
    pub name: String,
    pub enabled: bool,
    /// "authorized" | "expired" | "revoked"
    pub authorization_status: String,
    pub last_login_at: Option<String>,
    pub asset_scope_granted: bool,
    pub sync_status: String,
    pub last_sync_at: Option<String>,
    pub asset_count: i64,
    pub page_count: i64,
    pub sync_error: Option<String>,
}

pub struct NewCharacter {
    pub character_id: i64,
    pub name: String,
    pub scopes_granted: Vec<String>,
    pub token_expires_at: String,
}
