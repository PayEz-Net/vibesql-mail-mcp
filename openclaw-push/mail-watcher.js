#!/usr/bin/env node
/**
 * Agent Mail Watcher — OpenClaw Push Edition
 *
 * Polls the VibeSQL Mail inbox and injects new messages directly into an
 * OpenClaw agent's context via the gateway CLI.
 *
 * Designed to run INSIDE an OpenClaw container where `openclaw.mjs` is available.
 * Also writes to a NOTIFICATIONS.md file as a fallback/audit trail.
 *
 * Usage:
 *   node mail-watcher.js --agent RosaCalder
 *   node mail-watcher.js --agent RosaCalder --interval 60
 *   node mail-watcher.js --agent RosaCalder --mute
 *
 * Options:
 *   --agent <name>         Agent name (required)
 *   --interval <secs>      Poll interval in seconds (default: 30)
 *   --mute                 Start muted (no notifications, still tracks seen IDs)
 *   --output <path>        Notifications file (default: ~/.openclaw/workspace/NOTIFICATIONS.md)
 *   --state <path>         State file (default: ~/.openclaw/workspace/.mail-watcher-state.json)
 *   --openclaw-cmd <cmd>   OpenClaw CLI path (default: node /app/openclaw.mjs)
 *   --no-inject            Write to file only, skip OpenClaw context injection
 *   --openclaw-agent <id>  OpenClaw agent ID to inject into (default: main)
 *
 * Runtime commands (stdin, TTY only):
 *   mute      — Pause notifications (still polls to track seen IDs)
 *   unmute    — Resume notifications
 *   status    — Print current status
 *   quit      — Save state and exit
 */

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const readline = require('readline');
const { execSync } = require('child_process');

// ---------------------------------------------------------------------------
// Parse CLI args
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);

function getArg(flag) {
  const idx = args.indexOf(flag);
  return idx !== -1 && idx + 1 < args.length ? args[idx + 1] : null;
}

const AGENT_NAME = getArg('--agent');
if (!AGENT_NAME) {
  console.error('Error: --agent <name> is required');
  console.error('Usage: node mail-watcher.js --agent RosaCalder');
  process.exit(1);
}

const HOME = process.env.HOME || process.env.USERPROFILE || '/home/node';
const WORKSPACE = path.join(HOME, '.openclaw', 'workspace');
const POLL_INTERVAL_S = parseInt(getArg('--interval') || '30', 10);
const POLL_INTERVAL_MS = POLL_INTERVAL_S * 1000;
const NOTIF_FILE = getArg('--output') || path.join(WORKSPACE, 'NOTIFICATIONS.md');
const STATE_FILE = getArg('--state') || path.join(WORKSPACE, '.mail-watcher-state.json');
const OPENCLAW_CMD = getArg('--openclaw-cmd') || 'node /app/openclaw.mjs';
const OPENCLAW_AGENT = getArg('--openclaw-agent') || 'main';
const INJECT_ENABLED = !args.includes('--no-inject');

let muted = args.includes('--mute');

// ---------------------------------------------------------------------------
// API Config — VibeSQL Mail API with HMAC auth
// ---------------------------------------------------------------------------
// Set these environment variables before running:
//   VIBESQL_MAIL_API_URL   — Base URL for the agent mail API (e.g. https://your-server.com/v1/agentmail)
//   VIBESQL_MAIL_CLIENT_ID — Your HMAC client ID
//   VIBESQL_MAIL_SECRET    — Your HMAC secret key (base64)
const API_CONFIG = {
  apiUrl: process.env.VIBESQL_MAIL_API_URL || 'http://localhost:4100/v1/agentmail',
  clientId: process.env.VIBESQL_MAIL_CLIENT_ID || '',
  secretKey: process.env.VIBESQL_MAIL_SECRET || ''
};

if (!API_CONFIG.clientId || !API_CONFIG.secretKey) {
  console.error('Error: VIBESQL_MAIL_CLIENT_ID and VIBESQL_MAIL_SECRET environment variables are required');
  console.error('Set them before running: export VIBESQL_MAIL_CLIENT_ID=your_client_id');
  process.exit(1);
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------
let seenIds = new Set();
let newMailCount = 0;

function loadState() {
  try {
    const raw = fs.readFileSync(STATE_FILE, 'utf-8');
    if (!raw.trim()) return;
    const state = JSON.parse(raw);
    seenIds = new Set(state.seenIds || []);
    log(`State loaded: ${seenIds.size} seen messages`);
  } catch {
    log('No previous state, starting fresh');
  }
}

function saveState() {
  const state = {
    seenIds: [...seenIds].slice(-500),
    lastPollTime: new Date().toISOString()
  };
  try {
    const dir = path.dirname(STATE_FILE);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(STATE_FILE, JSON.stringify(state, null, 2));
  } catch (e) {
    log(`Warning: failed to save state: ${e.message}`);
  }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------
function log(msg) {
  const ts = new Date().toISOString().slice(11, 19);
  console.log(`[${ts}] [${AGENT_NAME}] ${msg}`);
}

// ---------------------------------------------------------------------------
// HMAC Auth
// ---------------------------------------------------------------------------
function sign(method, urlPath) {
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const signature = crypto
    .createHmac('sha256', Buffer.from(API_CONFIG.secretKey, 'base64'))
    .update(`${timestamp}|${method}|${urlPath}`)
    .digest('base64');
  return { timestamp, signature };
}

async function apiCall(method, urlPath) {
  const { timestamp, signature } = sign(method, urlPath);
  const endpoint = urlPath.replace('/v1/agentmail', '');

  const response = await fetch(`${API_CONFIG.apiUrl}${endpoint}`, {
    method,
    headers: {
      'X-Vibe-Client-Id': API_CONFIG.clientId,
      'X-Vibe-Timestamp': timestamp,
      'X-Vibe-Signature': signature,
    }
  });
  return response.json();
}

// ---------------------------------------------------------------------------
// Mail Operations
// ---------------------------------------------------------------------------
async function getInbox() {
  return apiCall('GET', `/v1/agentmail/inbox/${AGENT_NAME}`);
}

async function readMessage(id) {
  return apiCall('GET', `/v1/agentmail/messages/${id}`);
}

// ---------------------------------------------------------------------------
// Notification Writer + OpenClaw Inject
// ---------------------------------------------------------------------------
function writeNotification(from, subject, body, messageId) {
  const truncatedBody = body.length > 2000
    ? body.slice(0, 2000) + '\n\n[... truncated — read full with: node agent-mail.cjs --agent ' + AGENT_NAME + ' --prod read ' + messageId + ']'
    : body;

  const entry = [
    '',
    `## NEW MAIL #${messageId} — ${new Date().toISOString().slice(0, 16)}`,
    `**From:** ${from}`,
    `**Subject:** ${subject}`,
    '',
    truncatedBody,
    '',
    `**Reply:** \`node agent-mail.cjs --agent ${AGENT_NAME} --prod send ${from} "Re: ${subject}" --body "your reply"\``,
    '',
    '---',
    ''
  ].join('\n');

  // Write to notifications file
  try {
    const dir = path.dirname(NOTIF_FILE);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

    if (!fs.existsSync(NOTIF_FILE)) {
      fs.writeFileSync(NOTIF_FILE, `# ${AGENT_NAME} — Mail Notifications\n\n---\n`);
    }

    fs.appendFileSync(NOTIF_FILE, entry);
    log(`Notification written: #${messageId} from ${from}`);
  } catch (e) {
    log(`Notification write error: ${e.message}`);
  }

  // Inject into OpenClaw agent context
  if (INJECT_ENABLED) {
    try {
      const injectMsg = [
        `NEW MAIL #${messageId} from ${from}: ${subject}`,
        '---',
        truncatedBody,
        '---',
        `Reply: node /app/agent-mail.cjs --agent ${AGENT_NAME} --prod send ${from} "Re: ${subject}" --body "your reply"`
      ].join('\n');

      const tmpFile = `/tmp/mail-notify-${messageId}.txt`;
      fs.writeFileSync(tmpFile, injectMsg);
      execSync(
        `${OPENCLAW_CMD} agent --agent ${OPENCLAW_AGENT} -m "$(cat ${tmpFile})" --json`,
        { timeout: 120_000, stdio: ['pipe', 'pipe', 'pipe'] }
      );
      log(`OpenClaw inject OK: #${messageId}`);
      try { fs.unlinkSync(tmpFile); } catch {}
    } catch (e) {
      log(`OpenClaw inject failed (non-critical): ${(e.message || '').slice(0, 150)}`);
    }
  }
}

// ---------------------------------------------------------------------------
// Poll Loop
// ---------------------------------------------------------------------------
async function poll() {
  try {
    const result = await getInbox();

    if (!result.success || !result.data?.messages) {
      log(`Inbox check failed: ${JSON.stringify(result.error || 'unknown')}`);
      return;
    }

    const messages = result.data.messages;
    const unread = messages.filter(m => !m.read_at);

    const newMessages = unread.filter(m => {
      const id = String(m.message_id || m.id);
      return !seenIds.has(id);
    });

    if (newMessages.length > 0) {
      log(`${newMessages.length} new message(s)`);

      for (const msg of newMessages) {
        const id = String(msg.message_id || msg.id);
        const from = msg.from_agent_display || msg.from_agent || 'Unknown';
        const subject = msg.subject || '(no subject)';

        // Always mark as seen (even if muted)
        seenIds.add(id);

        if (muted) {
          log(`[MUTED] Skipping notification for #${id} from ${from}`);
          continue;
        }

        // Read full message
        const full = await readMessage(id);
        const body = full.success && full.data?.body
          ? full.data.body
          : '(could not read message body)';

        writeNotification(from, subject, body, id);
        newMailCount++;
      }

      saveState();
    }
  } catch (e) {
    log(`Poll error: ${e.message}`);
  }
}

// ---------------------------------------------------------------------------
// Interactive Commands (stdin)
// ---------------------------------------------------------------------------
function setupCommands() {
  if (!process.stdin.isTTY) return;

  const rl = readline.createInterface({ input: process.stdin });

  rl.on('line', (line) => {
    const cmd = line.trim().toLowerCase();
    switch (cmd) {
      case 'mute':
        muted = true;
        log('MUTED — notifications paused (still tracking seen IDs)');
        break;
      case 'unmute':
        muted = false;
        log('UNMUTED — notifications resumed');
        break;
      case 'status':
        log(`Agent: ${AGENT_NAME} | Muted: ${muted} | Seen: ${seenIds.size} | Delivered: ${newMailCount} | Interval: ${POLL_INTERVAL_S}s`);
        log(`Inject: ${INJECT_ENABLED} | Notifications: ${NOTIF_FILE}`);
        log(`State: ${STATE_FILE}`);
        break;
      case 'quit':
      case 'exit':
        log('Saving state and exiting...');
        saveState();
        process.exit(0);
        break;
      default:
        log('Commands: mute | unmute | status | quit');
    }
  });
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  log('=== Mail Watcher Starting ===');
  log(`Agent: ${AGENT_NAME}`);
  log(`Poll interval: ${POLL_INTERVAL_S}s`);
  log(`Inject: ${INJECT_ENABLED} (${OPENCLAW_CMD})`);
  log(`Notifications: ${NOTIF_FILE}`);
  log(`State: ${STATE_FILE}`);
  log(`Muted: ${muted}`);
  log('');

  loadState();

  // Initial poll
  await poll();

  // Schedule recurring polls
  setInterval(poll, POLL_INTERVAL_MS);

  // Interactive commands
  setupCommands();

  // Graceful shutdown
  process.on('SIGTERM', () => {
    log('SIGTERM — saving state...');
    saveState();
    process.exit(0);
  });

  process.on('SIGINT', () => {
    log('SIGINT — saving state...');
    saveState();
    process.exit(0);
  });
}

main().catch(e => {
  log(`Fatal: ${e.message}`);
  process.exit(1);
});
