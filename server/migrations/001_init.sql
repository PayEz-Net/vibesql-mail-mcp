-- vibesql-mail schema v2
-- Matches production vibe_agents collection (relational translation)

-- Settings (key/value config)
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Agent profiles (full identity — matches prod agent_profiles)
CREATE TABLE IF NOT EXISTS agents (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  display_name TEXT,
  role TEXT,
  role_md TEXT,
  identity_md TEXT,
  philosophy_md TEXT,
  communication_md TEXT,
  response_pattern_md TEXT,
  expertise_json JSONB,
  profile TEXT,
  program TEXT,
  model TEXT,
  is_template BOOLEAN DEFAULT false,
  owner_user_id INTEGER,
  is_active BOOLEAN DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  last_active_at TIMESTAMPTZ
);

-- Agent teams (matches prod agent_teams)
CREATE TABLE IF NOT EXISTS teams (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS team_members (
  id SERIAL PRIMARY KEY,
  team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  joined_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(team_id, agent_id)
);

-- Projects (matches prod vibe_projects)
CREATE TABLE IF NOT EXISTS projects (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT DEFAULT 'active',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Kanban boards (matches prod agent_kanban_boards)
CREATE TABLE IF NOT EXISTS kanban_boards (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS kanban_columns (
  id SERIAL PRIMARY KEY,
  board_id INTEGER NOT NULL REFERENCES kanban_boards(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS kanban_cards (
  id SERIAL PRIMARY KEY,
  column_id INTEGER NOT NULL REFERENCES kanban_columns(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  description TEXT,
  assigned_agent_id INTEGER REFERENCES agents(id) ON DELETE SET NULL,
  position INTEGER NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Messages (matches prod agent_mail_messages)
CREATE TABLE IF NOT EXISTS messages (
  id SERIAL PRIMARY KEY,
  from_agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  thread_id TEXT NOT NULL,
  subject TEXT,
  body TEXT NOT NULL,
  body_format TEXT DEFAULT 'markdown',
  importance TEXT DEFAULT 'normal',
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Inbox (delivery tracking — relational version of prod to_agents[])
CREATE TABLE IF NOT EXISTS inbox (
  id SERIAL PRIMARY KEY,
  message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  agent_id INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  recipient_type TEXT DEFAULT 'to',
  read_at TIMESTAMPTZ,
  archived_at TIMESTAMPTZ
);

-- Indexes: inbox
CREATE INDEX IF NOT EXISTS idx_inbox_agent ON inbox(agent_id);
CREATE INDEX IF NOT EXISTS idx_inbox_message ON inbox(message_id);
CREATE INDEX IF NOT EXISTS idx_inbox_unread ON inbox(agent_id) WHERE read_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_inbox_agent_created ON inbox(agent_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_inbox_agent_message ON inbox(agent_id, message_id);

-- Indexes: messages
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_agent_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_id_created ON messages(id, created_at DESC);

-- Indexes: teams
CREATE INDEX IF NOT EXISTS idx_team_members_agent ON team_members(agent_id);
CREATE INDEX IF NOT EXISTS idx_team_members_team ON team_members(team_id);

-- Indexes: kanban
CREATE INDEX IF NOT EXISTS idx_kanban_boards_project ON kanban_boards(project_id);
CREATE INDEX IF NOT EXISTS idx_kanban_columns_board ON kanban_columns(board_id);
CREATE INDEX IF NOT EXISTS idx_kanban_cards_column ON kanban_cards(column_id);
CREATE INDEX IF NOT EXISTS idx_kanban_cards_agent ON kanban_cards(assigned_agent_id);

-- Indexes: agents
CREATE INDEX IF NOT EXISTS idx_agents_role ON agents(role);
CREATE INDEX IF NOT EXISTS idx_agents_active ON agents(id) WHERE is_active = true;
