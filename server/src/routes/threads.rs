use actix_web::{web, HttpResponse};
use serde_json::{json, Value};

use crate::db::DbClient;
use crate::error::AppError;

/// GET /v1/mail/threads/{thread_id}
pub async fn get_thread(
    db: web::Data<DbClient>,
    thread_id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let tid = thread_id.into_inner();

    let result = db
        .query(
            "SELECT m.id as message_id, a.name as from_agent, a.display_name as from_agent_display, \
             m.subject, m.body, m.body_format, m.importance, m.created_at::text as created_at \
             FROM messages m \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE m.thread_id = $1::text \
             ORDER BY m.created_at ASC",
            vec![Value::String(tid.clone())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    // Fetch all recipients for this thread in one query (avoids N+1)
    let recip_result = db
        .query(
            "SELECT i.message_id, a.name, i.recipient_type FROM inbox i \
             JOIN agents a ON a.id = i.agent_id \
             WHERE i.message_id IN ( \
               SELECT m.id FROM messages m WHERE m.thread_id = $1::text \
             )",
            vec![Value::String(tid.clone())],
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

            let (to, cc) = recip_map
                .get(&msg_id)
                .cloned()
                .unwrap_or_else(|| (Vec::new(), Vec::new()));

            messages.push(json!({
                "message_id": msg_id,
                "from_agent": row.get("from_agent").and_then(|v| v.as_str()).unwrap_or(""),
                "from_agent_display": row.get("from_agent_display").and_then(|v| v.as_str()),
                "to": to,
                "cc": cc,
                "subject": row.get("subject").and_then(|v| v.as_str()),
                "body": row.get("body").and_then(|v| v.as_str()).unwrap_or(""),
                "body_format": row.get("body_format").and_then(|v| v.as_str()).unwrap_or("markdown"),
                "importance": row.get("importance").and_then(|v| v.as_str()).unwrap_or("normal"),
                "created_at": row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }
    }

    if messages.is_empty() {
        return Err(AppError::NotFound(format!("Thread '{}' not found", tid)));
    }

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "thread_id": tid,
            "message_count": messages.len(),
            "messages": messages,
        }
    })))
}
