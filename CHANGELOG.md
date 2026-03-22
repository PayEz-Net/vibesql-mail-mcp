# Changelog

## 2026-03-14

### Added
- **OpenClaw push notification integration** — `openclaw-push/mail-watcher.js` polls the agent mail inbox and forwards new messages to OpenClaw gateway as push notifications. Includes README with setup instructions.
- **Agent mail performance indexes** — Composite indexes for inbox lookups by agent (sorted by recency), agent+message_id (mark-as-read), and message join optimization
- Performance indexes also baked into `001_init.sql` for new installs

### Security
- **Removed hardcoded credentials from mail-watcher** — OpenClaw push watcher now reads credentials from environment variables instead of hardcoded values in source
