use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct DbClient {
    http: Client,
    micro_url: String,
}

#[derive(Serialize)]
struct QueryRequest {
    sql: String,
    params: Vec<Value>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    #[allow(dead_code)]
    pub success: Option<bool>,
    pub rows: Option<Vec<Value>>, // Each Value is a JSON object like {"id": 1, "name": "foo"}
    #[allow(dead_code)]
    pub row_count: Option<i64>,
    #[allow(dead_code)]
    pub execution_time: Option<f64>,
    pub error: Option<Value>, // Can be {"code": "...", "message": "...", "detail": "..."}
}

impl DbClient {
    pub fn new(micro_url: &str) -> Self {
        Self {
            http: Client::new(),
            micro_url: micro_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn query(&self, sql: &str, params: Vec<Value>) -> Result<QueryResponse, String> {
        let url = format!("{}/v1/query", self.micro_url);
        let req = QueryRequest {
            sql: sql.to_string(),
            params,
        };

        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("Failed to reach vibesql-micro: {}", e))?;

        let result: QueryResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse vibesql-micro response: {}", e))?;

        if let Some(ref err) = result.error {
            let msg = err
                .get("detail")
                .or_else(|| err.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(format!("SQL error: {}", msg));
        }

        Ok(result)
    }

    pub async fn run_migrations(&self) -> Result<(), String> {
        info!("Running database migrations...");

        let migration_sql = include_str!("../migrations/001_init.sql");

        // Split on semicolons and execute each statement
        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if stmt.is_empty() {
                continue;
            }
            if let Err(e) = self.query(&format!("{};", stmt), vec![]).await {
                // Ignore "already exists" errors
                if !e.contains("already exists") {
                    error!("Migration error: {}", e);
                    return Err(e);
                }
            }
        }

        info!("Migrations complete.");
        Ok(())
    }

    /// Get a field from a specific row in the response
    pub fn field(response: &QueryResponse, row: usize, field: &str) -> Option<Value> {
        response
            .rows
            .as_ref()
            .and_then(|rows| rows.get(row))
            .and_then(|r| r.get(field))
            .cloned()
    }

    /// Get a string field
    pub fn field_str(response: &QueryResponse, row: usize, field: &str) -> Option<String> {
        Self::field(response, row, field).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// Get an i64 field
    pub fn field_i64(response: &QueryResponse, row: usize, field: &str) -> Option<i64> {
        Self::field(response, row, field).and_then(|v| v.as_i64())
    }
}

/// Helper: wrap an Option<&str> as a JSON Value (String or Null)
pub fn opt_str(val: Option<&str>) -> Value {
    match val {
        Some(s) => Value::String(s.to_string()),
        None => Value::Null,
    }
}
