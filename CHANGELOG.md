# Changelog

## 2026-03-22

### Added
- **MCP Server** (`server/src/mcp.rs`) — Rust stdio JSON-RPC server implementing MCP protocol. 6 tools: `check_inbox`, `read_message`, `send_mail`, `reply`, `list_agents`, `search_mail`. 3 resources: `mail://agents`, `mail://inbox/{agent}`, `mail://thread/{thread_id}`. Lazy initialization defers database connection to first tool call.
- **npm distribution package** (`npm/`) — `npx vibesql-mail-mcp --agent MyAgent` auto-downloads platform-specific Rust binary from GitHub releases. Supports Windows x64, Linux x64, macOS x64.
- **TUI client** (`client/`) — Interactive terminal mail client built with TypeScript + Ink (React for CLI). Screens: Inbox, Read, Compose, Reply, Forward, Sent, Agents, Thread, Help. Setup wizard, keyboard navigation, non-TTY read-only mode.
- **README** — Full documentation covering all three components, architecture diagram, MCP tools/resources, TUI screens, configuration, platform support.
- Second binary target in Cargo.toml: `vibesql-mail-mcp` alongside existing `vibesql-mail-server`

### Fixed
- **Migration runner SQL comment stripping** — `run_migrations()` now filters out `--` comment lines before sending statements to vibesql-micro, which rejects queries starting with comments
- **Inbox index on non-existent column** — `idx_inbox_agent_created` referenced `inbox.created_at` which doesn't exist; replaced with `idx_inbox_agent_message_id(agent_id, message_id DESC)`

## 2026-03-14

### Added
- **OpenClaw push notification integration** — `openclaw-push/mail-watcher.js` polls the agent mail inbox and forwards new messages to OpenClaw gateway as push notifications. Includes README with setup instructions.
- **Agent mail performance indexes** — Composite indexes for inbox lookups by agent (sorted by recency), agent+message_id (mark-as-read), and message join optimization
- Performance indexes also baked into `001_init.sql` for new installs

### Security
- **Removed hardcoded credentials from mail-watcher** — OpenClaw push watcher now reads credentials from environment variables instead of hardcoded values in source
