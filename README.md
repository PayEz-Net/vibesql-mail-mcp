# vibesql-mail

Agent-to-agent mail system for AI coding teams. Send, receive, search, and thread messages between agents — powered by VibeSQL.

Three components ship together:

| Component | What | Language |
|-----------|------|----------|
| **MCP Server** | Model Context Protocol server for Claude Code, OpenCode, and MCP-compatible clients | Rust |
| **npm Package** | `npx vibesql-mail-mcp` — zero-install distribution with auto-download | Node.js wrapper |
| **TUI Client** | Interactive terminal mail client with inbox, compose, threads, and search | TypeScript + Ink |

## Why

AI agents working in teams need asynchronous communication that preserves context across sessions. vibesql-mail gives every agent a persistent inbox with threading, importance levels, and full-text search — no external services, no API keys, no accounts.

Messages are stored in a local PostgreSQL database via [vibesql-micro](https://github.com/PayEz-Net/vibesql-micro) (embedded PG, single binary, zero config) or [vibesql-server](https://github.com/PayEz-Net/vibesql-server) for production deployments.

## Quick Start

### MCP Server (recommended)

Add to your `.mcp.json` (Claude Code) or MCP client config:

```json
{
  "mcpServers": {
    "agent-mail": {
      "command": "npx",
      "args": ["-y", "vibesql-mail-mcp", "--agent", "MyAgent"],
      "env": {}
    }
  }
}
```

> **Windows users:** Wrap with `cmd /c`:
> ```json
> "command": "cmd",
> "args": ["/c", "npx", "-y", "vibesql-mail-mcp", "--agent", "MyAgent"]
> ```

Requires [vibesql-micro](https://github.com/PayEz-Net/vibesql-micro) running on `localhost:5173` (default). Override with `--micro-url`.

### From Source (Rust)

```bash
cd server
cargo build --release --bin vibesql-mail-mcp
./target/release/vibesql-mail-mcp --agent BAPert --micro-url http://localhost:5173
```

### TUI Client

```bash
cd client
npm install && npm run build
node bin/vibesql-mail.js --agent BAPert
```

First run? Use `--setup` to configure your server connection:

```bash
node bin/vibesql-mail.js --setup
```

## MCP Server Tools

The MCP server exposes 6 tools that any MCP-compatible AI client can call:

| Tool | Description |
|------|-------------|
| `check_inbox` | Check your inbox — returns messages with sender, subject, date, and read status |
| `read_message` | Read a message by ID — returns full body and marks it as read |
| `send_mail` | Send a message to one or more agents |
| `reply` | Reply to a message — auto-fills thread_id, recipient, and RE: subject |
| `list_agents` | List all registered agents with roles and status |
| `search_mail` | Search messages by keyword, sender, or date range |

### MCP Resources

| Resource | Description |
|----------|-------------|
| `mail://agents` | Agent directory |
| `mail://inbox/{agent}` | Inbox for a specific agent |
| `mail://thread/{thread_id}` | Full conversation thread |

## Message Format

```
send_mail:
  to: "BAPert, DotNetPert"     # comma-separated recipients
  subject: "Deploy ready"
  body: "All tests pass."
  cc: "QAPert"                  # optional
  importance: "high"            # low, normal, high, urgent
  thread_id: "abc123"           # optional — continues a thread
```

### Limits

| Field | Limit |
|-------|-------|
| Body | 65,536 bytes |
| Subject | 256 characters |
| Recipients (to + cc) | 10 |

## TUI Client Screens

| Screen | Key | Description |
|--------|-----|-------------|
| Inbox | `i` | Message list with unread markers |
| Read | `Enter` | Full message display with headers |
| Compose | `c` | New message with interactive fields |
| Reply | `r` | Pre-filled reply with quoted body |
| Forward | `f` | Pre-filled forward |
| Sent | `s` | Sent messages |
| Agents | `a` | Agent directory with compose-to |
| Thread | `t` | Full conversation view |
| Help | `?` | Keyboard shortcuts |
| Quit | `q` | Exit |

## Architecture

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Claude Code /   │     │  TUI Client      │     │  Other MCP       │
│  AI Agent        │     │  (terminal)       │     │  Clients         │
└────────┬─────────┘     └────────┬──────────┘     └────────┬─────────┘
         │ MCP (stdin/stdout)     │ HTTP                     │ MCP
         v                        v                          v
┌──────────────────────────────────────────────────────────────────────┐
│                    vibesql-mail-mcp (Rust)                          │
│                    JSON-RPC 2.0 over stdio                          │
└────────────────────────────┬─────────────────────────────────────────┘
                             │ POST /v1/query
                             v
              ┌──────────────────────────────┐
              │  vibesql-micro               │
              │  (embedded PostgreSQL 16.1)  │
              │  localhost:5173              │
              └──────────────────────────────┘
```

**Local development:** vibesql-micro — single binary, zero config, embedded PostgreSQL.

**Production:** vibesql-server — .NET 9, external PostgreSQL, HMAC auth, horizontal scaling.

Both backends speak the same protocol: `POST /v1/query` with `{"sql": "...", "params": [...]}`.

## Database Schema

Auto-migrated on first connection. Core tables:

| Table | Purpose |
|-------|---------|
| `agents` | Agent profiles — name, role, display name, active status |
| `messages` | Mail messages — sender, thread, subject, body, importance |
| `inbox` | Delivery tracking — per-recipient read/archive status |
| `teams` | Agent teams |
| `team_members` | Team membership |
| `projects` | Project tracking |
| `kanban_boards` | Kanban boards, columns, and cards |

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_NAME` | — | Agent name (alternative to `--agent`) |
| `VIBESQL_MAIL_MICRO_URL` | `http://localhost:5173` | VibeSQL endpoint (alternative to `--micro-url`) |

### CLI Arguments

```
vibesql-mail-mcp --agent <name> [--micro-url <url>]
```

## Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| Windows | x64 | Supported |
| Linux | x64 | Supported |
| macOS | x64 (Intel) | Supported |
| macOS | arm64 (Apple Silicon) | Planned |

## Requirements

- **vibesql-micro** >=1.1.0 (parameterized query support) or **vibesql-server** >=2.0
- **Node.js** >=14 (npm package)
- **Rust** >=1.70 (building from source)

## License

Apache-2.0

## Links

- [VibeSQL Documentation](https://vibesql.online/docs.html)
- [vibesql-micro](https://github.com/PayEz-Net/vibesql-micro) — Embedded PostgreSQL for local development
- [vibesql-server](https://github.com/PayEz-Net/vibesql-server) — Production PostgreSQL server
