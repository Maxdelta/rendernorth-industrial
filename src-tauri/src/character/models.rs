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
}

pub struct NewCharacter {
    pub character_id: i64,
    pub name: String,
    pub scopes_granted: Vec<String>,
    pub token_expires_at: String,
}
