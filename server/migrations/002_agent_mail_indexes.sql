-- =====================================================================================
-- Agent Mail Performance Indexes
-- =====================================================================================
-- Purpose: Add composite indexes for inbox and message lookup performance
-- Date: 2026-03-14
-- Author: Aurum
-- =====================================================================================

-- Index 1: Inbox lookups by agent sorted by recency (inbox list, unread count)
CREATE INDEX IF NOT EXISTS idx_inbox_agent_created
ON inbox (agent_id, created_at DESC);

-- Index 2: Inbox lookup by agent + message_id (mark-as-read, get entry by message)
CREATE INDEX IF NOT EXISTS idx_inbox_agent_message
ON inbox (agent_id, message_id);

-- Index 3: Messages by ID with created_at (join path optimization)
CREATE INDEX IF NOT EXISTS idx_messages_id_created
ON messages (id, created_at DESC);
