use actix_web::{web, HttpResponse};
use serde_json::json;

use crate::db::DbClient;
use crate::error::AppError;

/// GET /v1/mail/health
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
        }
    }))
}

/// POST /v1/mail/admin/init — run migrations
pub async fn init_db(db: web::Data<DbClient>) -> Result<HttpResponse, AppError> {
    db.run_migrations()
        .await
        .map_err(|e| AppError::DbError(e))?;

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "message": "Database initialized"
        }
    })))
}

/// GET /v1/mail/admin/status — database status
pub async fn status(db: web::Data<DbClient>) -> Result<HttpResponse, AppError> {
    let agents_result = db
        .query("SELECT COUNT(*) as cnt FROM agents", vec![])
        .await
        .map_err(|e| AppError::DbError(e))?;

    let messages_result = db
        .query("SELECT COUNT(*) as cnt FROM messages", vec![])
        .await
        .map_err(|e| AppError::DbError(e))?;

    let inbox_result = db
        .query(
            "SELECT COUNT(*) as cnt FROM inbox WHERE read_at IS NULL",
            vec![],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let agent_count = DbClient::field_i64(&agents_result, 0, "cnt").unwrap_or(0);
    let message_count = DbClient::field_i64(&messages_result, 0, "cnt").unwrap_or(0);
    let unread_count = DbClient::field_i64(&inbox_result, 0, "cnt").unwrap_or(0);

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "agents": agent_count,
            "messages": message_count,
            "unread_inbox": unread_count,
        }
    })))
}
