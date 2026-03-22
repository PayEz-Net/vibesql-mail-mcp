mod db;
mod models;

use db::DbClient;
use serde_json::{json, Value};
use std::env;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_BODY_SIZE: usize = 65536;
const MAX_SUBJECT_LEN: usize = 256;
const MAX_RECIPIENTS: usize = 10;

struct McpState {
    db: DbClient,
    agent_name: String,
    agent_id: Mutex<Option<i64>>,
    initialized: Mutex<bool>,
}

impl McpState {
    /// Lazy init: run migrations and resolve agent on first tool call.
    /// Returns the agent_id or an error string.
    async fn ensure_ready(&self) -> Result<i64, String> {
        // Fast path: already initialized
        {
            let init = self.initialized.lock().await;
            if *init {
                return self.agent_id.lock().await.ok_or_else(|| {
                    "Agent ID not resolved (this should not happen)".to_string()
                });
            }
        }

        // Slow path: first call — run migrations + resolve agent
        eprintln!(
            "[agent-mail-mcp] Connecting to vibesql-micro at {} ...",
            self.db.micro_url()
        );

        if let Err(e) = self.db.run_migrations().await {
            return Err(format!(
                "Cannot reach vibesql-micro ({}). Is it running? Error: {}",
                self.db.micro_url(),
                e
            ));
        }

        let id = resolve_agent_id(&self.db, &self.agent_name).await.map_err(|e| {
            format!(
                "Connected to vibesql-micro but failed to resolve agent '{}': {}",
                self.agent_name, e
            )
        })?;

        eprintln!(
            "[agent-mail-mcp] Ready — agent '{}' (id: {})",
            self.agent_name, id
        );

        *self.agent_id.lock().await = Some(id);
        *self.initialized.lock().await = true;
        Ok(id)
    }

    /// Get the resolved agent_id (panics if not initialized — always call ensure_ready first)
    async fn get_agent_id(&self) -> i64 {
        self.agent_id.lock().await.expect("ensure_ready not called")
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let (agent_name, micro_url) = parse_args();

    let db = DbClient::new(&micro_url);

    // Do NOT connect here — defer to first tool call
    eprintln!(
        "[agent-mail-mcp] Starting for agent '{}' (vibesql-micro: {})",
        agent_name, micro_url
    );
    eprintln!("[agent-mail-mcp] Database connection deferred until first tool call");

    let state = Arc::new(McpState {
        db,
        agent_name,
        agent_id: Mutex::new(None),
        initialized: Mutex::new(false),
    });

    // Stdio JSON-RPC loop
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = request.get("id").cloned();
        let params = request.get("params").cloned().unwrap_or(json!({}));

        // Notifications (no id) — don't respond
        if id.is_none() {
            continue;
        }

        let result = match method {
            "initialize" => handle_initialize(),
            "tools/list" => handle_tools_list(),
            "tools/call" => handle_tools_call(&state, &params).await,
            "resources/list" => handle_resources_list(),
            "resources/read" => handle_resources_read(&state, &params).await,
            _ => json!({"error": {"code": -32601, "message": format!("Method not found: {}", method)}}),
        };

        let response = if result.get("error").is_some() {
            json!({"jsonrpc": "2.0", "id": id, "error": result["error"]})
        } else {
            json!({"jsonrpc": "2.0", "id": id, "result": result})
        };

        let out = serde_json::to_string(&response).unwrap();
        let _ = writeln!(stdout, "{}", out);
        let _ = stdout.flush();
    }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn parse_args() -> (String, String) {
    let args: Vec<String> = env::args().collect();
    let mut agent_name: Option<String> = None;
    let mut micro_url: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--agent" => {
                i += 1;
                agent_name = args.get(i).cloned();
            }
            "--micro-url" => {
                i += 1;
                micro_url = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }

    let agent = agent_name.or_else(|| env::var("AGENT_NAME").ok()).unwrap_or_else(|| {
        eprintln!("Error: --agent <name> is required");
        eprintln!("Usage: vibesql-mail-mcp --agent BAPert [--micro-url http://localhost:5173]");
        std::process::exit(1);
    });

    let url = micro_url
        .or_else(|| env::var("VIBESQL_MAIL_MICRO_URL").ok())
        .unwrap_or_else(|| "http://localhost:5173".to_string());

    (agent, url)
}

// ---------------------------------------------------------------------------
// Agent resolution (copied from routes/agents.rs to avoid actix deps)
// ---------------------------------------------------------------------------

async fn resolve_agent_id(db: &DbClient, name: &str) -> Result<i64, String> {
    let result = db
        .query(
            "SELECT id FROM agents WHERE name = $1::text",
            vec![Value::String(name.to_string())],
        )
        .await?;

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
        .await?;

    DbClient::field_i64(&result, 0, "id").ok_or_else(|| "Failed to register agent".to_string())
}

// ---------------------------------------------------------------------------
// MCP handlers
// ---------------------------------------------------------------------------

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "vibesql-mail-mcp",
            "version": "1.0.0"
        }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "check_inbox",
                "description": "Check your inbox for messages. Returns messages with sender, subject, date, and read status.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "read_message",
                "description": "Read a specific message by ID. Returns full message body and marks it as read.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "number", "description": "Message ID to read" }
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "send_mail",
                "description": "Send a message to one or more agents.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "to": { "type": "string", "description": "Comma-separated recipient agent names" },
                        "subject": { "type": "string", "description": "Message subject" },
                        "body": { "type": "string", "description": "Message body" },
                        "cc": { "type": "string", "description": "Comma-separated CC agent names (optional)" },
                        "thread_id": { "type": "string", "description": "Thread ID to continue (optional)" },
                        "importance": { "type": "string", "description": "low, normal, high, or urgent (default: normal)" }
                    },
                    "required": ["to", "subject", "body"]
                }
            },
            {
                "name": "reply",
                "description": "Reply to a message. Pre-fills thread_id and recipient from the original message.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message_id": { "type": "number", "description": "ID of the message to reply to" },
                        "body": { "type": "string", "description": "Reply body" }
                    },
                    "required": ["message_id", "body"]
                }
            },
            {
                "name": "list_agents",
                "description": "List all registered agents with their names, roles, and status.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "search_mail",
                "description": "Search messages by keyword, sender, or date range.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Keyword to search in subject and body" },
                        "from": { "type": "string", "description": "Filter by sender agent name" },
                        "since": { "type": "string", "description": "ISO date — messages after this date" },
                        "limit": { "type": "number", "description": "Max results (default 20)" }
                    },
                    "required": []
                }
            }
        ]
    })
}

fn handle_resources_list() -> Value {
    json!({
        "resources": [
            {
                "uri": "mail://agents",
                "name": "Agent Directory",
                "description": "List of all registered agents",
                "mimeType": "text/plain"
            }
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "mail://inbox/{agent}",
                "name": "Agent Inbox",
                "description": "Inbox for a specific agent",
                "mimeType": "text/plain"
            },
            {
                "uriTemplate": "mail://thread/{thread_id}",
                "name": "Thread",
                "description": "Full conversation thread",
                "mimeType": "text/plain"
            }
        ]
    })
}

async fn handle_resources_read(state: &McpState, params: &Value) -> Value {
    // Lazy init: connect to vibesql-micro on first resource read
    if let Err(e) = state.ensure_ready().await {
        return mcp_error(&format!("agent-mail-mcp startup error: {}", e));
    }

    let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

    if uri == "mail://agents" {
        return match tool_list_agents(state).await {
            Ok(text) => json!({"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}),
            Err(e) => mcp_error(&e),
        };
    }

    if let Some(agent) = uri.strip_prefix("mail://inbox/") {
        return match tool_check_inbox_for(state, agent).await {
            Ok(text) => json!({"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}),
            Err(e) => mcp_error(&e),
        };
    }

    if let Some(tid) = uri.strip_prefix("mail://thread/") {
        return match tool_thread(state, tid).await {
            Ok(text) => json!({"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}),
            Err(e) => mcp_error(&e),
        };
    }

    mcp_error(&format!("Unknown resource: {}", uri))
}

async fn handle_tools_call(state: &McpState, params: &Value) -> Value {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // Lazy init: connect to vibesql-micro on first tool call
    if let Err(e) = state.ensure_ready().await {
        return json!({
            "content": [{"type": "text", "text": format!("agent-mail-mcp startup error: {}", e)}],
            "isError": true
        });
    }

    let result = match name {
        "check_inbox" => tool_check_inbox(state).await,
        "read_message" => tool_read_message(state, &args).await,
        "send_mail" => tool_send_mail(state, &args).await,
        "reply" => tool_reply(state, &args).await,
        "list_agents" => tool_list_agents(state).await,
        "search_mail" => tool_search_mail(state, &args).await,
        _ => Err(format!("Unknown tool: {}", name)),
    };

    match result {
        Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
        Err(e) => json!({"content": [{"type": "text", "text": format!("Error: {}", e)}], "isError": true}),
    }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

async fn tool_check_inbox(state: &McpState) -> Result<String, String> {
    tool_check_inbox_for(state, &state.agent_name).await
}

async fn tool_check_inbox_for(state: &McpState, agent_name: &str) -> Result<String, String> {
    // Get agent ID
    let agent_result = state
        .db
        .query(
            "SELECT id FROM agents WHERE name = $1::text",
            vec![Value::String(agent_name.to_string())],
        )
        .await?;

    let agent_id = DbClient::field_i64(&agent_result, 0, "id")
        .ok_or_else(|| format!("Agent '{}' not found", agent_name))?;

    // Unread count
    let unread_result = state
        .db
        .query(
            "SELECT COUNT(*) as cnt FROM inbox WHERE agent_id = $1::int AND read_at IS NULL AND archived_at IS NULL",
            vec![Value::Number(agent_id.into())],
        )
        .await?;
    let unread_count = DbClient::field_i64(&unread_result, 0, "cnt").unwrap_or(0);

    // Total count
    let total_result = state
        .db
        .query(
            "SELECT COUNT(*) as cnt FROM inbox WHERE agent_id = $1::int AND archived_at IS NULL",
            vec![Value::Number(agent_id.into())],
        )
        .await?;
    let total_count = DbClient::field_i64(&total_result, 0, "cnt").unwrap_or(0);

    // Recent messages
    let msg_result = state
        .db
        .query(
            "SELECT i.id as inbox_id, m.id as message_id, a.name as from_agent, \
             m.subject, m.created_at::text as created_at, i.read_at::text as read_at \
             FROM inbox i \
             JOIN messages m ON m.id = i.message_id \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE i.agent_id = $1::int AND i.archived_at IS NULL \
             ORDER BY m.created_at DESC LIMIT 20",
            vec![Value::Number(agent_id.into())],
        )
        .await?;

    let mut output = format!(
        "Inbox for {} ({} unread of {} total)\n\n",
        agent_name, unread_count, total_count
    );

    if let Some(ref rows) = msg_result.rows {
        for row in rows {
            let msg_id = row.get("message_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let from = row.get("from_agent").and_then(|v| v.as_str()).unwrap_or("?");
            let subject = row.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
            let created = row.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
            let is_unread = row.get("read_at").map_or(true, |v| v.is_null());
            let marker = if is_unread { "* " } else { "  " };
            output.push_str(&format!(
                "{}[{}] From: {} — \"{}\" — {}\n",
                marker, msg_id, from, subject, created
            ));
        }
        if rows.is_empty() {
            output.push_str("  (no messages)\n");
        }
    }

    output.push_str("\n* = unread");
    Ok(output)
}

async fn tool_read_message(state: &McpState, args: &Value) -> Result<String, String> {
    let message_id = args
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing required argument: id".to_string())?;

    // Fetch message
    let result = state
        .db
        .query(
            "SELECT m.id as message_id, a.name as from_agent, a.display_name as from_agent_display, \
             m.subject, m.body, m.body_format, m.importance, m.thread_id, \
             m.created_at::text as created_at \
             FROM messages m \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE m.id = $1::int",
            vec![Value::Number(message_id.into())],
        )
        .await?;

    let row = result
        .rows
        .as_ref()
        .and_then(|rows| rows.first())
        .ok_or_else(|| format!("Message {} not found", message_id))?;

    // Get recipients
    let recip_result = state
        .db
        .query(
            "SELECT a.name, i.recipient_type FROM inbox i \
             JOIN agents a ON a.id = i.agent_id \
             WHERE i.message_id = $1::int",
            vec![Value::Number(message_id.into())],
        )
        .await?;

    let mut to_list = Vec::new();
    let mut cc_list = Vec::new();
    if let Some(ref rows) = recip_result.rows {
        for r in rows {
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let rtype = r.get("recipient_type").and_then(|v| v.as_str()).unwrap_or("to");
            match rtype {
                "cc" => cc_list.push(name),
                _ => to_list.push(name),
            }
        }
    }

    // Mark as read
    let _ = state
        .db
        .query(
            "UPDATE inbox SET read_at = NOW() \
             WHERE message_id = $1::int AND agent_id = $2::int AND read_at IS NULL",
            vec![
                Value::Number(message_id.into()),
                Value::Number(state.get_agent_id().await.into()),
            ],
        )
        .await;

    let from = row.get("from_agent").and_then(|v| v.as_str()).unwrap_or("?");
    let subject = row.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
    let body = row.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let thread_id = row.get("thread_id").and_then(|v| v.as_str()).unwrap_or("");
    let created = row.get("created_at").and_then(|v| v.as_str()).unwrap_or("");

    let mut output = format!("Message #{}\n", message_id);
    output.push_str(&format!("From: {}\n", from));
    output.push_str(&format!("To: {}\n", to_list.join(", ")));
    if !cc_list.is_empty() {
        output.push_str(&format!("CC: {}\n", cc_list.join(", ")));
    }
    output.push_str(&format!("Date: {}\n", created));
    output.push_str(&format!("Thread: {}\n", thread_id));
    output.push_str(&format!("Subject: {}\n", subject));
    output.push_str(&format!("\n{}", body));

    Ok(output)
}

async fn tool_send_mail(state: &McpState, args: &Value) -> Result<String, String> {
    let to_str = args
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: to".to_string())?;
    let subject = args
        .get("subject")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: subject".to_string())?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: body".to_string())?;

    let to: Vec<String> = to_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let cc: Vec<String> = args
        .get("cc")
        .and_then(|v| v.as_str())
        .map(|s| s.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    let thread_id = args.get("thread_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let importance = args
        .get("importance")
        .and_then(|v| v.as_str())
        .unwrap_or("normal");

    // Validate
    if to.is_empty() {
        return Err("'to' must have at least one recipient".to_string());
    }
    if to.len() + cc.len() > MAX_RECIPIENTS {
        return Err(format!("Max {} recipients", MAX_RECIPIENTS));
    }
    if body.is_empty() {
        return Err("Body cannot be empty".to_string());
    }
    if body.len() > MAX_BODY_SIZE {
        return Err(format!("Body exceeds max size of {} bytes", MAX_BODY_SIZE));
    }
    if subject.len() > MAX_SUBJECT_LEN {
        return Err(format!("Subject exceeds max length of {} chars", MAX_SUBJECT_LEN));
    }
    if !["low", "normal", "high", "urgent"].contains(&importance) {
        return Err("importance must be low, normal, high, or urgent".to_string());
    }

    send_message_impl(state, &to, &cc, subject, body, thread_id.as_deref(), importance).await
}

async fn tool_reply(state: &McpState, args: &Value) -> Result<String, String> {
    let message_id = args
        .get("message_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing required argument: message_id".to_string())?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: body".to_string())?;

    // Fetch original message
    let result = state
        .db
        .query(
            "SELECT a.name as from_agent, m.subject, m.thread_id \
             FROM messages m \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE m.id = $1::int",
            vec![Value::Number(message_id.into())],
        )
        .await?;

    let row = result
        .rows
        .as_ref()
        .and_then(|rows| rows.first())
        .ok_or_else(|| format!("Message {} not found", message_id))?;

    let original_from = row.get("from_agent").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let original_subject = row.get("subject").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let thread_id = row.get("thread_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let subject = if original_subject.starts_with("RE: ") {
        original_subject.clone()
    } else {
        format!("RE: {}", original_subject)
    };

    send_message_impl(
        state,
        &[original_from],
        &[],
        &subject,
        body,
        Some(&thread_id),
        "normal",
    )
    .await
}

async fn send_message_impl(
    state: &McpState,
    to: &[String],
    cc: &[String],
    subject: &str,
    body: &str,
    thread_id: Option<&str>,
    importance: &str,
) -> Result<String, String> {
    let from_id = state.get_agent_id().await;

    // Generate thread_id if not provided
    let tid = match thread_id {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => {
            let bytes: [u8; 8] = rand::random();
            hex::encode(bytes)
        }
    };

    // Insert message
    let result = state
        .db
        .query(
            "INSERT INTO messages (from_agent_id, thread_id, subject, body, body_format, importance) \
             VALUES ($1::int, $2::text, $3::text, $4::text, 'markdown', $5::text) \
             RETURNING id, created_at::text",
            vec![
                Value::Number(from_id.into()),
                Value::String(tid.clone()),
                Value::String(subject.to_string()),
                Value::String(body.to_string()),
                Value::String(importance.to_string()),
            ],
        )
        .await?;

    let message_id = DbClient::field_i64(&result, 0, "id")
        .ok_or_else(|| "Failed to insert message".to_string())?;

    // Create inbox entries
    for recipient in to {
        let agent_id = resolve_agent_id(&state.db, recipient).await?;
        state
            .db
            .query(
                "INSERT INTO inbox (message_id, agent_id, recipient_type) VALUES ($1::int, $2::int, 'to')",
                vec![
                    Value::Number(message_id.into()),
                    Value::Number(agent_id.into()),
                ],
            )
            .await?;
    }
    for recipient in cc {
        let agent_id = resolve_agent_id(&state.db, recipient).await?;
        state
            .db
            .query(
                "INSERT INTO inbox (message_id, agent_id, recipient_type) VALUES ($1::int, $2::int, 'cc')",
                vec![
                    Value::Number(message_id.into()),
                    Value::Number(agent_id.into()),
                ],
            )
            .await?;
    }

    let mut output = format!("Message sent (ID: {})\n", message_id);
    output.push_str(&format!("Thread: {}\n", tid));
    output.push_str(&format!("To: {}\n", to.join(", ")));
    if !cc.is_empty() {
        output.push_str(&format!("CC: {}\n", cc.join(", ")));
    }
    output.push_str(&format!("Subject: {}", subject));

    Ok(output)
}

async fn tool_list_agents(state: &McpState) -> Result<String, String> {
    let result = state
        .db
        .query(
            "SELECT name, role, is_active, last_active_at::text \
             FROM agents ORDER BY name",
            vec![],
        )
        .await?;

    let mut output = String::from("Agent Directory\n\n");

    if let Some(ref rows) = result.rows {
        for row in rows {
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let role = row.get("role").and_then(|v| v.as_str()).unwrap_or("-");
            let active = row.get("is_active").and_then(|v| v.as_bool()).unwrap_or(false);
            let last = row.get("last_active_at").and_then(|v| v.as_str()).unwrap_or("never");
            let status = if active { "active" } else { "inactive" };
            output.push_str(&format!("  {} — {} [{}] (last: {})\n", name, role, status, last));
        }
        if rows.is_empty() {
            output.push_str("  (no agents registered)\n");
        }
    }

    Ok(output)
}

async fn tool_search_mail(state: &McpState, args: &Value) -> Result<String, String> {
    let keyword = args.get("query").and_then(|v| v.as_str());
    let from_agent = args.get("from").and_then(|v| v.as_str());
    let since = args.get("since").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20).min(100);

    let mut conditions = vec![
        "i.agent_id = $1::int".to_string(),
        "i.archived_at IS NULL".to_string(),
    ];
    let mut params: Vec<Value> = vec![Value::Number(state.get_agent_id().await.into())];
    let mut param_idx = 2;

    if let Some(kw) = keyword {
        conditions.push(format!(
            "(m.subject ILIKE ${0}::text OR m.body ILIKE ${0}::text)",
            param_idx
        ));
        params.push(Value::String(format!("%{}%", kw)));
        param_idx += 1;
    }

    if let Some(from) = from_agent {
        conditions.push(format!("a.name = ${}::text", param_idx));
        params.push(Value::String(from.to_string()));
        param_idx += 1;
    }

    if let Some(date) = since {
        conditions.push(format!("m.created_at >= ${}::timestamptz", param_idx));
        params.push(Value::String(date.to_string()));
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");
    let sql = format!(
        "SELECT m.id as message_id, a.name as from_agent, m.subject, m.body, \
         m.created_at::text as created_at, i.read_at::text as read_at \
         FROM inbox i \
         JOIN messages m ON m.id = i.message_id \
         JOIN agents a ON a.id = m.from_agent_id \
         WHERE {} \
         ORDER BY m.created_at DESC LIMIT ${}::int",
        where_clause, param_idx
    );
    params.push(Value::Number(limit.into()));

    let result = state.db.query(&sql, params).await?;

    let mut output = String::from("Search Results\n\n");
    let mut count = 0;

    if let Some(ref rows) = result.rows {
        for row in rows {
            count += 1;
            let msg_id = row.get("message_id").and_then(|v| v.as_i64()).unwrap_or(0);
            let from = row.get("from_agent").and_then(|v| v.as_str()).unwrap_or("?");
            let subject = row.get("subject").and_then(|v| v.as_str()).unwrap_or("(no subject)");
            let body = row.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let created = row.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
            let is_unread = row.get("read_at").map_or(true, |v| v.is_null());
            let marker = if is_unread { "* " } else { "  " };
            let preview = if body.len() > 80 { &body[..77] } else { body };
            output.push_str(&format!(
                "{}[{}] From: {} — \"{}\" — {}\n      {}{}\n\n",
                marker,
                msg_id,
                from,
                subject,
                created,
                preview,
                if body.len() > 80 { "..." } else { "" }
            ));
        }
    }

    if count == 0 {
        output.push_str("  (no messages found)\n");
    } else {
        output.push_str(&format!("{} result(s)", count));
    }

    Ok(output)
}

async fn tool_thread(state: &McpState, thread_id: &str) -> Result<String, String> {
    let result = state
        .db
        .query(
            "SELECT m.id as message_id, a.name as from_agent, m.subject, m.body, \
             m.created_at::text as created_at \
             FROM messages m \
             JOIN agents a ON a.id = m.from_agent_id \
             WHERE m.thread_id = $1::text \
             ORDER BY m.created_at ASC",
            vec![Value::String(thread_id.to_string())],
        )
        .await?;

    let mut output = format!("Thread: {}\n\n", thread_id);
    let mut count = 0;

    if let Some(ref rows) = result.rows {
        for row in rows {
            count += 1;
            let from = row.get("from_agent").and_then(|v| v.as_str()).unwrap_or("?");
            let subject = row.get("subject").and_then(|v| v.as_str()).unwrap_or("");
            let body = row.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let created = row.get("created_at").and_then(|v| v.as_str()).unwrap_or("");

            output.push_str(&format!("--- {} — {} ---\n", from, created));
            if !subject.is_empty() {
                output.push_str(&format!("Subject: {}\n", subject));
            }
            output.push_str(&format!("{}\n\n", body));
        }
    }

    if count == 0 {
        return Err(format!("Thread '{}' not found", thread_id));
    }

    output.push_str(&format!("({} messages)", count));
    Ok(output)
}

fn mcp_error(msg: &str) -> Value {
    json!({"error": {"code": -32000, "message": msg}})
}
