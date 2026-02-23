use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::db::{opt_str, DbClient};
use crate::error::AppError;
use crate::models::inbox::{InboxMessage, InboxResponse, Pagination, ReadMessageResponse};
use crate::models::message::{SendMessageRequest, SendMessageResponse};
use crate::routes::agents::resolve_agent_id;
use crate::routes::stream::SseBroadcaster;

const MAX_BODY_SIZE: usize = 65536;
const MAX_SUBJECT_LEN: usize = 256;
const MAX_RECIPIENTS: usize = 10;

/// POST /v1/mail/send
pub async fn send_message(
    db: web::Data<DbClient>,
    broadcaster: web::Data<SseBroadcaster>,
    body: web::Json<SendMessageRequest>,
) -> Result<HttpResponse, AppError> {
    let req = body.into_inner();

    // Validate
    if req.to.is_empty() {
        return Err(AppError::BadRequest(
            "'to' must have at least one recipient".into(),
        ));
    }
    if req.to.len() + req.cc.len() > MAX_RECIPIENTS {
        return Err(AppError::BadRequest(format!(
            "Max {} recipients",
            MAX_RECIPIENTS
        )));
    }
    if req.body.is_empty() {
        return Err(AppError::BadRequest("Body cannot be empty".into()));
    }
    if req.body.len() > MAX_BODY_SIZE {
        return Err(AppError::BadRequest(format!(
            "Body exceeds max size of {} bytes",
            MAX_BODY_SIZE
        )));
    }
    if let Some(ref subj) = req.subject {
        if subj.len() > MAX_SUBJECT_LEN {
            return Err(AppError::BadRequest(format!(
                "Subject exceeds max length of {} chars",
                MAX_SUBJECT_LEN
            )));
        }
    }
    if !["plain", "markdown"].contains(&req.body_format.as_str()) {
        return Err(AppError::BadRequest(
            "body_format must be 'plain' or 'markdown'".into(),
        ));
    }
    if !["low", "normal", "high", "urgent"].contains(&req.importance.as_str()) {
        return Err(AppError::BadRequest(
            "importance must be low, normal, high, or urgent".into(),
        ));
    }

    // Resolve sender
    let from_id = resolve_agent_id(&db, &req.from_agent).await?;

    // Generate thread_id if not provided
    let thread_id = req.thread_id.unwrap_or_else(|| {
        let bytes: [u8; 8] = rand::random();
        hex::encode(bytes)
    });

    // Insert message
    let result = db
        .query(
            "INSERT INTO messages (from_agent_id, thread_id, subject, body, body_format, importance) \
             VALUES ($1::int, $2::text, $3::text, $4::text, $5::text, $6::text) \
             RETURNING id, created_at::text",
            vec![
                Value::Number(from_id.into()),
                Value::String(thread_id.clone()),
                opt_str(req.subject.as_deref()),
                Value::String(req.body.clone()),
                Value::String(req.body_format.clone()),
                Value::String(req.importance.clone()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let message_id = DbClient::field_i64(&result, 0, "id")
        .ok_or_else(|| AppError::Internal("Failed to insert message".into()))?;
    let created_at = DbClient::field_str(&result, 0, "created_at").unwrap_or_default();

    // Create inbox entries for all recipients
    for recipient in &req.to {
        let agent_id = resolve_agent_id(&db, recipient).await?;
        db.query(
            "INSERT INTO inbox (message_id, agent_id, recipient_type) \
             VALUES ($1::int, $2::int, 'to')",
            vec![
                Value::Number(message_id.into()),
                Value::Number(agent_id.into()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;
    }
    for recipient in &req.cc {
        let agent_id = resolve_agent_id(&db, recipient).await?;
        db.query(
            "INSERT INTO inbox (message_id, agent_id, recipient_type) \
             VALUES ($1::int, $2::int, 'cc')",
            vec![
                Value::Number(message_id.into()),
                Value::Number(agent_id.into()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;
    }

    // Broadcast SSE event to all recipients
    let all_recipients: Vec<String> = req.to.iter().chain(req.cc.iter()).cloned().collect();
    for recipient in &all_recipients {
        broadcaster.send(
            recipient,
            "new-mail",
            &json!({
                "message_id": message_id,
                "from": req.from_agent,
                "subject": req.subject
            }),
        );
    }

    let response = SendMessageResponse {
        message_id,
        thread_id,
        from_agent: req.from_agent,
        to: req.to,
        cc: req.cc,
        subject: req.subject,
        importance: req.importance,
        created_at,
    };

    Ok(HttpResponse::Created().json(json!({
        "success": true,
        "data": response
    })))
}

#[derive(Deserialize)]
pub struct InboxQuery {
    pub unread: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Deserialize)]
pub struct MarkReadRequest {
    pub agent: String,
}

/// GET /v1/mail/inbox/{agent}
pub async fn get_inbox(
    db: web::Data<DbClient>,
    agent: web::Path<String>,
    query: web::Query<InboxQuery>,
) -> Result<HttpResponse, AppError> {
    let agent_name = agent.into_inner();
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Get agent ID
    let agent_result = db
        .query(
            "SELECT id FROM agents WHERE name = $1::text",
            vec![Value::String(agent_name.clone())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let agent_id = DbClient::field_i64(&agent_result, 0, "id")
        .ok_or_else(|| AppError::NotFound(format!("Agent '{}' not found", agent_name)))?;

    // Build query with optional unread filter
    let (count_sql, msg_sql) = if query.unread.unwrap_or(false) {
        (
            "SELECT COUNT(*) as cnt FROM inbox i \
             WHERE i.agent_id = $1::int AND i.read_at IS NULL AND i.archived_at IS NULL",
            "SELECT i.id as inbox_id, m.id as message_id, a.name as from_agent, \
             a.display_name as from_agent_display, m.subject, m.body, m.body_format, \
             m.importance, i.recipient_type, m.created_at::text as created_at, \
             i.read_at::text as read_at \
             FROM inbox i \
             JOIN messages m ON m.id = i.message_id \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE i.agent_id = $1::int AND i.read_at IS NULL AND i.archived_at IS NULL \
             ORDER BY m.created_at DESC \
             LIMIT $2::int OFFSET $3::int",
        )
    } else {
        (
            "SELECT COUNT(*) as cnt FROM inbox i \
             WHERE i.agent_id = $1::int AND i.archived_at IS NULL",
            "SELECT i.id as inbox_id, m.id as message_id, a.name as from_agent, \
             a.display_name as from_agent_display, m.subject, m.body, m.body_format, \
             m.importance, i.recipient_type, m.created_at::text as created_at, \
             i.read_at::text as read_at \
             FROM inbox i \
             JOIN messages m ON m.id = i.message_id \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE i.agent_id = $1::int AND i.archived_at IS NULL \
             ORDER BY m.created_at DESC \
             LIMIT $2::int OFFSET $3::int",
        )
    };

    let id_param = vec![Value::Number(agent_id.into())];

    // Get total count
    let count_result = db
        .query(count_sql, id_param.clone())
        .await
        .map_err(|e| AppError::DbError(e))?;
    let total_count = DbClient::field_i64(&count_result, 0, "cnt").unwrap_or(0);

    // Get unread count (always)
    let unread_result = db
        .query(
            "SELECT COUNT(*) as cnt FROM inbox \
             WHERE agent_id = $1::int AND read_at IS NULL AND archived_at IS NULL",
            id_param.clone(),
        )
        .await
        .map_err(|e| AppError::DbError(e))?;
    let unread_count = DbClient::field_i64(&unread_result, 0, "cnt").unwrap_or(0);

    // Get messages
    let msg_result = db
        .query(
            msg_sql,
            vec![
                Value::Number(agent_id.into()),
                Value::Number(page_size.into()),
                Value::Number(offset.into()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let mut messages = Vec::new();
    if let Some(ref rows) = msg_result.rows {
        for row in rows {
            messages.push(InboxMessage {
                inbox_id: row.get("inbox_id").and_then(|v| v.as_i64()).unwrap_or(0),
                message_id: row
                    .get("message_id")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                from_agent: row
                    .get("from_agent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                from_agent_display: row
                    .get("from_agent_display")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                subject: row
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                body: row
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                body_format: row
                    .get("body_format")
                    .and_then(|v| v.as_str())
                    .unwrap_or("markdown")
                    .to_string(),
                importance: row
                    .get("importance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("normal")
                    .to_string(),
                recipient_type: row
                    .get("recipient_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("to")
                    .to_string(),
                created_at: row
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                read_at: row
                    .get("read_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }
    }

    let total_pages = if total_count == 0 {
        0
    } else {
        (total_count + page_size - 1) / page_size
    };

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": InboxResponse {
            agent: agent_name,
            messages,
            unread_count,
        },
        "pagination": Pagination {
            page,
            page_size,
            total_count,
            total_pages,
        }
    })))
}

/// GET /v1/mail/messages/{id} — read message (read-only, use POST mark_read to mark)
pub async fn read_message(
    db: web::Data<DbClient>,
    id: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let message_id = id.into_inner();

    let result = db
        .query(
            "SELECT m.id as message_id, a.name as from_agent, a.display_name as from_agent_display, \
             m.subject, m.body, m.body_format, m.importance, m.thread_id, \
             m.created_at::text as created_at \
             FROM messages m \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE m.id = $1::int",
            vec![Value::Number(message_id.into())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let row = result
        .rows
        .as_ref()
        .and_then(|rows| rows.first())
        .ok_or_else(|| AppError::NotFound(format!("Message {} not found", message_id)))?;

    let thread_id = row
        .get("thread_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Get all recipients
    let recip_result = db
        .query(
            "SELECT a.name, i.recipient_type FROM inbox i \
             JOIN agents a ON a.id = i.agent_id \
             WHERE i.message_id = $1::int",
            vec![Value::Number(message_id.into())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let mut to = Vec::new();
    let mut cc = Vec::new();
    if let Some(ref rows) = recip_result.rows {
        for r in rows {
            let name = r
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rtype = r
                .get("recipient_type")
                .and_then(|v| v.as_str())
                .unwrap_or("to");
            match rtype {
                "cc" => cc.push(name),
                _ => to.push(name),
            }
        }
    }

    let response = ReadMessageResponse {
        message_id,
        from_agent: row
            .get("from_agent")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        from_agent_display: row
            .get("from_agent_display")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        to,
        cc,
        subject: row
            .get("subject")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        body: row
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        body_format: row
            .get("body_format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown")
            .to_string(),
        importance: row
            .get("importance")
            .and_then(|v| v.as_str())
            .unwrap_or("normal")
            .to_string(),
        thread_id,
        created_at: row
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    };

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": response
    })))
}

/// POST /v1/mail/messages/{id}/read — explicitly mark as read for a specific agent
pub async fn mark_read(
    db: web::Data<DbClient>,
    id: web::Path<i64>,
    body: web::Json<MarkReadRequest>,
) -> Result<HttpResponse, AppError> {
    let message_id = id.into_inner();
    let agent_name = &body.agent;

    // Resolve agent
    let agent_result = db
        .query(
            "SELECT id FROM agents WHERE name = $1::text",
            vec![Value::String(agent_name.clone())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let agent_id = DbClient::field_i64(&agent_result, 0, "id")
        .ok_or_else(|| AppError::NotFound(format!("Agent '{}' not found", agent_name)))?;

    let result = db
        .query(
            "UPDATE inbox SET read_at = NOW() \
             WHERE message_id = $1::int AND agent_id = $2::int AND read_at IS NULL \
             RETURNING id",
            vec![
                Value::Number(message_id.into()),
                Value::Number(agent_id.into()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let updated = result.rows.as_ref().map(|r| r.len()).unwrap_or(0);

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "message_id": message_id,
            "agent": agent_name,
            "marked_read": updated
        }
    })))
}

/// GET /v1/mail/sent/{agent}
pub async fn get_sent(
    db: web::Data<DbClient>,
    agent: web::Path<String>,
    query: web::Query<InboxQuery>,
) -> Result<HttpResponse, AppError> {
    let agent_name = agent.into_inner();
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Get agent ID
    let agent_result = db
        .query(
            "SELECT id FROM agents WHERE name = $1::text",
            vec![Value::String(agent_name.clone())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let agent_id = DbClient::field_i64(&agent_result, 0, "id")
        .ok_or_else(|| AppError::NotFound(format!("Agent '{}' not found", agent_name)))?;

    // Count
    let count_result = db
        .query(
            "SELECT COUNT(*) as cnt FROM messages WHERE from_agent_id = $1::int",
            vec![Value::Number(agent_id.into())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;
    let total_count = DbClient::field_i64(&count_result, 0, "cnt").unwrap_or(0);

    // Get sent messages (LIMIT/OFFSET as params)
    let result = db
        .query(
            "SELECT m.id as message_id, m.subject, m.body, m.body_format, m.importance, \
             m.thread_id, m.created_at::text as created_at \
             FROM messages m \
             WHERE m.from_agent_id = $1::int \
             ORDER BY m.created_at DESC \
             LIMIT $2::int OFFSET $3::int",
            vec![
                Value::Number(agent_id.into()),
                Value::Number(page_size.into()),
                Value::Number(offset.into()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    // Fetch all recipients for these messages in one query (avoids N+1)
    let recip_result = db
        .query(
            "SELECT i.message_id, a.name, i.recipient_type FROM inbox i \
             JOIN agents a ON a.id = i.agent_id \
             WHERE i.message_id IN ( \
               SELECT m.id FROM messages m WHERE m.from_agent_id = $1::int \
               ORDER BY m.created_at DESC LIMIT $2::int OFFSET $3::int \
             )",
            vec![
                Value::Number(agent_id.into()),
                Value::Number(page_size.into()),
                Value::Number(offset.into()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    // Build recipient lookup: message_id -> (to, cc)
    let mut recip_map: std::collections::HashMap<i64, (Vec<Value>, Vec<Value>)> =
        std::collections::HashMap::new();
    if let Some(ref rrows) = recip_result.rows {
        for r in rrows {
            let mid = r.get("message_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let name = r
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let rtype = r
                .get("recipient_type")
                .and_then(|v| v.as_str())
                .unwrap_or("to");
            let entry = recip_map.entry(mid).or_insert_with(|| (Vec::new(), Vec::new()));
            match rtype {
                "cc" => entry.1.push(Value::String(name)),
                _ => entry.0.push(Value::String(name)),
            }
        }
    }

    let mut messages = Vec::new();
    if let Some(ref rows) = result.rows {
        for row in rows {
            let msg_id = row
                .get("message_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let (to_list, cc_list) = recip_map
                .get(&msg_id)
                .cloned()
                .unwrap_or_else(|| (Vec::new(), Vec::new()));

            messages.push(json!({
                "message_id": msg_id,
                "to": to_list,
                "cc": cc_list,
                "subject": row.get("subject").and_then(|v| v.as_str()),
                "body": row.get("body").and_then(|v| v.as_str()).unwrap_or(""),
                "body_format": row.get("body_format").and_then(|v| v.as_str()).unwrap_or("markdown"),
                "importance": row.get("importance").and_then(|v| v.as_str()).unwrap_or("normal"),
                "thread_id": row.get("thread_id").and_then(|v| v.as_str()).unwrap_or(""),
                "created_at": row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }
    }

    let total_pages = if total_count == 0 {
        0
    } else {
        (total_count + page_size - 1) / page_size
    };

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "agent": agent_name,
            "messages": messages,
        },
        "pagination": Pagination {
            page,
            page_size,
            total_count,
            total_pages,
        }
    })))
}
