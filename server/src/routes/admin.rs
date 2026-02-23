use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::{json, Value};

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

#[derive(Deserialize)]
pub struct InitRequest {
    pub project_name: Option<String>,
}

/// POST /v1/mail/admin/init — run migrations, optionally set project name
pub async fn init_db(
    db: web::Data<DbClient>,
    body: Option<web::Json<InitRequest>>,
) -> Result<HttpResponse, AppError> {
    db.run_migrations()
        .await
        .map_err(|e| AppError::DbError(e))?;

    // Store project name if provided
    if let Some(ref req) = body {
        if let Some(ref name) = req.project_name {
            db.query(
                "INSERT INTO settings (key, value, updated_at) \
                 VALUES ('project_name', $1::text, NOW()) \
                 ON CONFLICT (key) DO UPDATE SET value = $1::text, updated_at = NOW()",
                vec![Value::String(name.clone())],
            )
            .await
            .map_err(|e| AppError::DbError(e))?;
        }
    }

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

    let project_result = db
        .query(
            "SELECT value FROM settings WHERE key = 'project_name'",
            vec![],
        )
        .await
        .ok();

    let agent_count = DbClient::field_i64(&agents_result, 0, "cnt").unwrap_or(0);
    let message_count = DbClient::field_i64(&messages_result, 0, "cnt").unwrap_or(0);
    let unread_count = DbClient::field_i64(&inbox_result, 0, "cnt").unwrap_or(0);
    let project_name = project_result
        .and_then(|r| DbClient::field_str(&r, 0, "value"));

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "project_name": project_name,
            "agents": agent_count,
            "messages": message_count,
            "unread_inbox": unread_count,
        }
    })))
}

/// POST /v1/mail/admin/settings — upsert a setting
pub async fn set_setting(
    db: web::Data<DbClient>,
    body: web::Json<SettingRequest>,
) -> Result<HttpResponse, AppError> {
    db.query(
        "INSERT INTO settings (key, value, updated_at) \
         VALUES ($1::text, $2::text, NOW()) \
         ON CONFLICT (key) DO UPDATE SET value = $2::text, updated_at = NOW()",
        vec![
            Value::String(body.key.clone()),
            Value::String(body.value.clone()),
        ],
    )
    .await
    .map_err(|e| AppError::DbError(e))?;

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": {
            "key": body.key,
            "value": body.value
        }
    })))
}

/// GET /v1/mail/admin/settings — get all settings
pub async fn get_settings(db: web::Data<DbClient>) -> Result<HttpResponse, AppError> {
    let result = db
        .query(
            "SELECT key, value, updated_at::text FROM settings ORDER BY key",
            vec![],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let mut settings = json!({});
    if let Some(ref rows) = result.rows {
        for row in rows {
            if let (Some(key), Some(value)) = (
                row.get("key").and_then(|v| v.as_str()),
                row.get("value").and_then(|v| v.as_str()),
            ) {
                settings[key] = Value::String(value.to_string());
            }
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": settings
    })))
}

#[derive(Deserialize)]
pub struct SettingRequest {
    pub key: String,
    pub value: String,
}
