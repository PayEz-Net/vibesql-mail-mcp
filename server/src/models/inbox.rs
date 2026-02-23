use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct InboxMessage {
    pub inbox_id: i64,
    pub message_id: i64,
    pub from_agent: String,
    pub from_agent_display: Option<String>,
    pub subject: Option<String>,
    pub body: String,
    pub body_format: String,
    pub importance: String,
    pub recipient_type: String,
    pub created_at: String,
    pub read_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InboxResponse {
    pub agent: String,
    pub messages: Vec<InboxMessage>,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub page: i64,
    pub page_size: i64,
    pub total_count: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct ReadMessageResponse {
    pub message_id: i64,
    pub from_agent: String,
    pub from_agent_display: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: Option<String>,
    pub body: String,
    pub body_format: String,
    pub importance: String,
    pub thread_id: String,
    pub created_at: String,
}
