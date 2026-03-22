# vibesql-mail-mcp — Design Spec

**Author:** NextPert
**Date:** 2026-02-23
**Source:** BAPert assignment #946
**Location:** E:\Repos\vibesql-mail\server\ (second binary in existing crate)

---

## Overview

Second binary in the vibesql-mail-server Rust crate. Exposes agent mail as native MCP tools over stdio/JSON-RPC. Same database (vibesql-micro on :5173), same mail schema, new transport. For product demo: Claude Code drives agent collaboration purely through MCP tool calls.

```bash
vibesql-mail-mcp --agent BAPert
vibesql-mail-mcp --agent BAPert --micro-url http://10.0.0.5:5173
```

---

## Crate Changes

### Cargo.toml — Add second [[bin]]

```toml
[[bin]]
name = "vibesql-mail-server"
path = "src/main.rs"

[[bin]]
name = "vibesql-mail-mcp"
path = "src/mcp.rs"
```

No new dependencies needed. The MCP protocol is simple JSON-RPC over stdio — `serde_json` (already a dep) handles serialization. Raw JSON-RPC implementation for 6 tools is simpler than pulling in an MCP framework crate.

### New File

```
src/mcp.rs    # MCP binary entry point — stdio JSON-RPC server
```

### Reused (untouched)

```
src/db.rs             # DbClient — HTTP client for vibesql-micro
src/models/mod.rs     # Module exports
src/models/agent.rs   # Agent, RegisterAgentRequest
src/models/message.rs # Message, SendMessageRequest, SendMessageResponse
src/models/inbox.rs   # InboxMessage, InboxResponse, ReadMessageResponse
```

### Not used by MCP binary

```
src/main.rs           # Actix HTTP server (stays untouched)
src/auth.rs           # HTTP auth middleware (MCP doesn't need this)
src/error.rs          # Actix ResponseError impl (MCP has its own error format)
src/routes/*          # Actix route handlers (MCP reimplements the logic)
```

---

## CLI Interface

```
vibesql-mail-mcp --agent <name> [--micro-url <url>]

Options:
  --agent       Required. Agent name (who you are). Used for inbox, sending.
  --micro-url   Optional. vibesql-micro URL. Default: http://localhost:5173
                Also reads VIBESQL_MAIL_MICRO_URL env var.
```

Startup:
1. Parse args
2. Create `DbClient` with micro-url
3. Run migrations (idempotent, same as server startup)
4. Auto-register agent via upsert (same logic as `resolve_agent_id`)
5. Enter stdio JSON-RPC loop

---

## JSON-RPC Protocol (MCP over stdio)

MCP uses JSON-RPC 2.0 over stdin/stdout. Three request types we handle:

### initialize

Client sends:
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}
```

Server responds:
```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{},"resources":{}},"serverInfo":{"name":"vibesql-mail-mcp","version":"1.0.0"}}}
```

Client sends notification:
```json
{"jsonrpc":"2.0","method":"notifications/initialized"}
```

### tools/list

Client sends:
```json
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
```

Server responds with tool definitions (see Tools section below).

### tools/call

Client sends:
```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"check_inbox","arguments":{}}}
```

Server responds:
```json
{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"..."}]}}
```

### resources/list

Client sends:
```json
{"jsonrpc":"2.0","id":4,"method":"resources/list"}
```

Server responds with resource definitions.

### resources/read

Client sends:
```json
{"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":"mail://inbox/BAPert"}}
```

---

## Tools (6)

### 1. check_inbox

- Description: "Check your inbox for messages. Returns unread messages with sender, subject, date, and read status."
- Arguments: none (uses --agent from CLI)
- Implementation: Same SQL as `routes/messages.rs::get_inbox()` — query inbox JOIN messages JOIN agents, filter by agent_id, order by created_at DESC
- Output: Formatted text list of messages with ID, from, subject, date, unread indicator

### 2. read_message

- Description: "Read a specific message by ID. Returns full message body and marks it as read."
- Arguments: `id: number` (message ID)
- Implementation: Same SQL as `routes/messages.rs::read_message()` + `mark_read()`
  - Fetch message with thread_id, recipients
  - Mark as read (UPDATE inbox SET read_at = NOW())
- Output: Formatted message with From, To, CC, Subject, Date, Thread, Body

### 3. send_mail

- Description: "Send a message to one or more agents."
- Arguments:
  - `to: string` — comma-separated recipient agent names
  - `subject: string` — message subject
  - `body: string` — message body
  - `cc: string` (optional) — comma-separated CC agent names
  - `thread_id: string` (optional) — thread to continue
  - `importance: string` (optional) — low/normal/high/urgent (default: normal)
- Implementation: Same logic as `routes/messages.rs::send_message()`
  - Resolve sender (--agent) and all recipients via agent name lookup (auto-register if needed)
  - Generate thread_id if not provided (16-char hex)
  - INSERT into messages, INSERT into inbox for each recipient
  - Validate: body ≤ 65KB, subject ≤ 256 chars, to+cc ≤ 10 recipients
- Output: Confirmation with message_id, thread_id

### 4. reply

- Description: "Reply to a message. Pre-fills thread_id and recipient from the original message."
- Arguments:
  - `message_id: number` — ID of the message to reply to
  - `body: string` — reply body
- Implementation:
  - Fetch original message to get thread_id, from_agent
  - Build subject as "RE: {original_subject}" (if not already RE:)
  - Call same send logic as send_mail with thread_id pre-set and to = original sender
- Output: Confirmation with message_id, thread_id

### 5. list_agents

- Description: "List all registered agents with their names, roles, and status."
- Arguments: none
- Implementation: Same SQL as `routes/agents.rs::list_agents()` — SELECT * FROM agents ORDER BY name
- Output: Formatted table of agents with name, role, active status, last active

### 6. search_mail

- Description: "Search messages by keyword, sender, or date range."
- Arguments:
  - `query: string` (optional) — keyword to search in subject and body
  - `from: string` (optional) — filter by sender agent name
  - `since: string` (optional) — ISO date, messages after this date
  - `limit: number` (optional) — max results (default 20)
- Implementation: Dynamic SQL query against inbox + messages + agents
  - WHERE clauses built from provided filters
  - ILIKE for keyword search on subject + body
  - Agent name lookup for from filter
  - LIMIT clause for result cap
- Output: Formatted message list with ID, from, subject, date, preview

---

## Resources (3)

### mail://inbox/{agent}

- Description: "Live inbox view for an agent"
- Returns: Same data as check_inbox tool, formatted as markdown

### mail://thread/{thread_id}

- Description: "Full conversation thread"
- Returns: All messages in thread, chronological, with sender/date/body

### mail://agents

- Description: "Agent directory"
- Returns: Same data as list_agents tool, formatted as markdown

---

## src/mcp.rs — Structure

```rust
// High-level structure (not actual code, just architecture)

mod db;       // reuse
mod models;   // reuse

struct McpState {
    db: DbClient,
    agent_name: String,
    agent_id: i64,  // resolved at startup
}

fn main():
    1. Parse CLI args (--agent, --micro-url)
    2. Create DbClient
    3. Run migrations
    4. Resolve agent_id (auto-register if needed)
    5. Enter read loop:
       - Read line from stdin
       - Parse as JSON-RPC request
       - Route by method:
           "initialize" → handle_initialize()
           "notifications/initialized" → ignore (notification, no response)
           "tools/list" → handle_tools_list()
           "tools/call" → handle_tools_call(name, arguments)
           "resources/list" → handle_resources_list()
           "resources/read" → handle_resources_read(uri)
       - Write JSON-RPC response to stdout
       - Flush stdout

fn handle_tools_call(state, name, args):
    match name:
        "check_inbox" → inbox query
        "read_message" → message fetch + mark read
        "send_mail" → resolve recipients, insert message + inbox entries
        "reply" → fetch original, build reply, send
        "list_agents" → agents query
        "search_mail" → dynamic search query

    Return MCP content array: [{ type: "text", text: "..." }]
    On error: { content: [{ type: "text", text: "Error: ..." }], isError: true }
```

---

## Output Formatting

Tools return human-readable text (not raw JSON), so Claude can present it naturally.

### check_inbox example:
```
Inbox for BAPert (3 unread of 15 total)

  * [934] From: QAPert — "Code Review: MCP package" — 12:05 PM
  * [930] From: NextPert — "Design Spec: @vibesql/mcp" — 11:42 AM
    [928] From: NextPert — "Status update" — 11:20 AM
    [924] From: DotNetPert — "Server deploy done" — 10:55 AM

* = unread
```

### read_message example:
```
Message #934
From: QAPert
To: BAPert
Date: 2/23/2026, 12:05 PM
Thread: 40a05fff717b447a
Subject: Code Review: MCP package

[full message body here]
```

### send_mail / reply example:
```
Message sent (ID: 940)
Thread: 40a05fff717b447a
To: QAPert
Subject: RE: Code Review: MCP package
```

---

## Error Handling

All errors return MCP error content:
```json
{"content":[{"type":"text","text":"Error: message body exceeds 65KB limit"}],"isError":true}
```

Validation errors (same rules as HTTP server):
- Body > 65KB
- Subject > 256 chars
- To + CC > 10 recipients
- Agent name invalid (must be 1-64 chars, alphanumeric/hyphens/underscores)
- Message not found
- vibesql-micro connection failure

---

## Claude Code Integration

```json
{
  "mcpServers": {
    "agent-mail": {
      "command": "E:\\Repos\\vibesql-mail\\server\\target\\release\\vibesql-mail-mcp",
      "args": ["--agent", "BAPert", "--micro-url", "http://localhost:5173"]
    }
  }
}
```

Or after cargo install:
```json
{
  "mcpServers": {
    "agent-mail": {
      "command": "vibesql-mail-mcp",
      "args": ["--agent", "BAPert"]
    }
  }
}
```

---

## Implementation Order

1. Add `[[bin]]` entry to Cargo.toml
2. Implement JSON-RPC stdio loop in `src/mcp.rs` (initialize, tools/list)
3. Implement check_inbox + read_message (read-only, easiest to test)
4. Implement send_mail + reply (write operations)
5. Implement list_agents + search_mail
6. Implement resources/list + resources/read
7. Build + test against running vibesql-micro

---

## Verification

1. Start vibesql-micro: `npx vibesql-micro`
2. Build: `cargo build --bin vibesql-mail-mcp`
3. Test manually: pipe JSON-RPC to stdin
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | cargo run --bin vibesql-mail-mcp -- --agent TestAgent
   ```
4. Configure in Claude Code, verify tools appear
5. Test: "check my inbox", "send a message to Engineer about the API design", "reply to message 42"
