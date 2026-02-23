use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Agent {
    pub id: i64,
    pub name: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
    pub program: Option<String>,
    pub model: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub last_active_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
    pub program: Option<String>,
    pub model: Option<String>,
}
