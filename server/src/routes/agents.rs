use actix_web::{web, HttpResponse};
use serde_json::{json, Value};

use crate::db::{opt_str, DbClient};
use crate::error::AppError;
use crate::models::agent::{Agent, RegisterAgentRequest};

/// GET /v1/mail/agents — list all agents
pub async fn list_agents(db: web::Data<DbClient>) -> Result<HttpResponse, AppError> {
    let result = db
        .query(
            "SELECT id, name, display_name, role, program, model, is_active, \
             created_at::text, last_active_at::text \
             FROM agents ORDER BY name",
            vec![],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let agents = parse_agents(&result.rows);

    Ok(HttpResponse::Ok().json(json!({
        "success": true,
        "data": agents
    })))
}

/// POST /v1/mail/agents — register a new agent
pub async fn register_agent(
    db: web::Data<DbClient>,
    body: web::Json<RegisterAgentRequest>,
) -> Result<HttpResponse, AppError> {
    let name = body.name.trim();
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::BadRequest(
            "Agent name must be 1-64 characters".into(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::BadRequest(
            "Agent name must be alphanumeric, hyphens, or underscores".into(),
        ));
    }

    let result = db
        .query(
            "INSERT INTO agents (name, display_name, role, program, model) \
             VALUES ($1::text, $2::text, $3::text, $4::text, $5::text) \
             ON CONFLICT (name) DO UPDATE SET \
               display_name = COALESCE(EXCLUDED.display_name, agents.display_name), \
               role = COALESCE(EXCLUDED.role, agents.role), \
               program = COALESCE(EXCLUDED.program, agents.program), \
               model = COALESCE(EXCLUDED.model, agents.model), \
               last_active_at = NOW() \
             RETURNING id, name, display_name, role, program, model, is_active, \
               created_at::text, last_active_at::text",
            vec![
                Value::String(name.to_string()),
                opt_str(body.display_name.as_deref()),
                opt_str(body.role.as_deref()),
                opt_str(body.program.as_deref()),
                opt_str(body.model.as_deref()),
            ],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    let agents = parse_agents(&result.rows);
    let agent = agents
        .first()
        .ok_or_else(|| AppError::Internal("Failed to create agent".into()))?;

    Ok(HttpResponse::Created().json(json!({
        "success": true,
        "data": agent
    })))
}

fn parse_agents(rows: &Option<Vec<Value>>) -> Vec<Agent> {
    let mut agents = Vec::new();
    if let Some(ref rows) = rows {
        for row in rows {
            agents.push(Agent {
                id: row.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
                name: row
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                display_name: row
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                role: row
                    .get("role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                program: row
                    .get("program")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                model: row
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                is_active: row.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true),
                created_at: row
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                last_active_at: row
                    .get("last_active_at")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }
    }
    agents
}

/// Resolve agent name to ID, auto-registering if needed
pub async fn resolve_agent_id(db: &DbClient, name: &str) -> Result<i64, AppError> {
    let result = db
        .query(
            "SELECT id FROM agents WHERE name = $1::text",
            vec![Value::String(name.to_string())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    if let Some(id) = DbClient::field_i64(&result, 0, "id") {
        let _ = db
            .query(
                "UPDATE agents SET last_active_at = NOW() WHERE id = $1::int",
                vec![Value::Number(id.into())],
            )
            .await;
        return Ok(id);
    }

    // Auto-register
    let result = db
        .query(
            "INSERT INTO agents (name) VALUES ($1::text) RETURNING id",
            vec![Value::String(name.to_string())],
        )
        .await
        .map_err(|e| AppError::DbError(e))?;

    DbClient::field_i64(&result, 0, "id")
        .ok_or_else(|| AppError::Internal("Failed to register agent".into()))
}
