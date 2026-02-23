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

    let mut messages = Vec::new();
    if let Some(ref rows) = result.rows {
        for row in rows {
            let msg_id = row
                .get("message_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            // Get recipients
            let recip_result = db
                .query(
                    "SELECT a.name, i.recipient_type FROM inbox i \
                     JOIN agents a ON a.id = i.agent_id \
                     WHERE i.message_id = $1::int",
                    vec![Value::Number(msg_id.into())],
                )
                .await
                .map_err(|e| AppError::DbError(e))?;

            let mut to = Vec::new();
            let mut cc = Vec::new();
            if let Some(ref rrows) = recip_result.rows {
                for r in rrows {
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
                        "cc" => cc.push(Value::String(name)),
                        _ => to.push(Value::String(name)),
                    }
                }
            }

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
