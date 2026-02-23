CREATE TABLE IF NOT EXISTS agents (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  display_name TEXT,
  role TEXT,
  program TEXT,
  model TEXT,
  is_active BOOLEAN DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_active_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS messages (
  id SERIAL PRIMARY KEY,
  from_agent_id INTEGER NOT NULL REFERENCES agents(id),
  thread_id TEXT NOT NULL,
  subject TEXT,
  body TEXT NOT NULL,
  body_format TEXT DEFAULT 'markdown',
  importance TEXT DEFAULT 'normal',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS inbox (
  id SERIAL PRIMARY KEY,
  message_id INTEGER NOT NULL REFERENCES messages(id),
  agent_id INTEGER NOT NULL REFERENCES agents(id),
  recipient_type TEXT DEFAULT 'to',
  read_at TIMESTAMPTZ,
  archived_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_inbox_agent ON inbox(agent_id);
CREATE INDEX IF NOT EXISTS idx_inbox_message ON inbox(message_id);
CREATE INDEX IF NOT EXISTS idx_inbox_unread ON inbox(agent_id) WHERE read_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_agent_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at DESC);
