# OpenClaw Push — VibeSQL Mail Notifications

Push VibeSQL Mail messages directly into an OpenClaw agent's context. The agent receives mail without polling or manually checking — messages appear in-session.

## How it works

`mail-watcher.js` runs inside the OpenClaw container and:

1. Polls the VibeSQL Mail API every 30s for unread messages
2. Writes new messages to `NOTIFICATIONS.md` (audit trail)
3. Injects each message into the agent's active session via `openclaw.mjs agent -m`

The agent sees the message as if someone typed it into the chat.

## Setup

### 1. Copy scripts into the container

```bash
# Copy the watcher
docker cp mail-watcher.js <container>:/app/mail-watcher.cjs

# Copy agent-mail.js so the agent can reply
docker cp agent-mail.js <container>:/app/agent-mail.cjs
```

Note: `.cjs` extension is required if the container's `package.json` has `"type": "module"`.

### 2. Fix the gateway token

OpenClaw generates a runtime gateway token that differs from the config file. The watcher's inject uses the gateway, so these must match.

```bash
# Check the runtime token
docker exec <container> printenv OPENCLAW_GATEWAY_TOKEN

# Update the config to match
docker exec -u root <container> sed -i \
  's/OLD_TOKEN/RUNTIME_TOKEN/g' \
  /home/node/.openclaw/openclaw.json
```

Both `gateway.auth.token` and `gateway.remote.token` in the config must equal the `OPENCLAW_GATEWAY_TOKEN` env var.

### 3. Fix workspace permissions

The container runs as `node` (UID 1000) but workspace files may be owned by the host user:

```bash
docker exec -u root <container> chown -R node:node /home/node/.openclaw/workspace/
```

### 4. Seed the state (avoid reprocessing old mail)

Start muted first to mark all existing messages as seen:

```bash
docker exec -d <container> node /app/mail-watcher.cjs \
  --agent <AgentName> --mute
```

Wait ~30s for one poll cycle, then kill and restart unmuted:

```bash
docker exec <container> pkill -f mail-watcher
docker exec -d <container> node /app/mail-watcher.cjs \
  --agent <AgentName>
```

### 5. Verify

Send a test message from another agent and confirm the watcher injects it:

```bash
# From any machine with agent-mail.js
node agent-mail.js --agent Aurum --prod send <AgentName> "test" --body "push test"

# Check the container's notifications file after ~30s
docker exec <container> tail -20 /home/node/.openclaw/workspace/NOTIFICATIONS.md
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--agent <name>` | (required) | Agent inbox to watch |
| `--interval <secs>` | 30 | Poll interval |
| `--mute` | false | Track messages but don't notify |
| `--no-inject` | false | Write to file only, skip gateway inject |
| `--openclaw-cmd <cmd>` | `node /app/openclaw.mjs` | Path to OpenClaw CLI |
| `--openclaw-agent <id>` | `main` | Agent ID to inject into |
| `--output <path>` | `~/.openclaw/workspace/NOTIFICATIONS.md` | Notifications file |
| `--state <path>` | `~/.openclaw/workspace/.mail-watcher-state.json` | State file |

## Known issues

- **Gateway token regenerates on container restart.** Re-run step 2 after restarting the container.
- **Inject is synchronous.** Each message takes 1-3s via the gateway (longer if it falls back to embedded mode). Multiple messages queue sequentially.
- **Agent reply quality varies.** The OpenClaw agent receives the message and can reply, but whether it correctly executes the `agent-mail.cjs send` command depends on the underlying model.

## Files

| File | Purpose |
|------|---------|
| `mail-watcher.js` | The watcher script — copy into container as `.cjs` |
| `README.md` | This file |
