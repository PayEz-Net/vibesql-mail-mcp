use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Message {
    pub id: i64,
    pub from_agent_id: i64,
    pub thread_id: String,
    pub subject: Option<String>,
    pub body: String,
    pub body_format: String,
    pub importance: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub from_agent: String,
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    pub subject: Option<String>,
    pub body: String,
    #[serde(default = "default_body_format")]
    pub body_format: String,
    #[serde(default = "default_importance")]
    pub importance: String,
    pub thread_id: Option<String>,
}

fn default_body_format() -> String {
    "markdown".to_string()
}

fn default_importance() -> String {
    "normal".to_string()
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_id: i64,
    pub thread_id: String,
    pub from_agent: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: Option<String>,
    pub importance: String,
    pub created_at: String,
}
