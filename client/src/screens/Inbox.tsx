import React, { useState, useEffect, useCallback } from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';
import { MessageList } from '../components/MessageList.js';
import { fetchInbox } from '../api.js';
import type { ApiConfig, InboxMessage } from '../api.js';

interface InboxProps {
  config: ApiConfig;
  agent: string;
  onReadMessage: (msg: InboxMessage) => void;
  onUnreadCount: (count: number) => void;
}

export function Inbox({ config, agent, onReadMessage, onUnreadCount }: InboxProps) {
  const [messages, setMessages] = useState<InboxMessage[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [searching, setSearching] = useState(false);

  const loadInbox = useCallback(async () => {
    try {
      const result = await fetchInbox(agent, config);
      if (result.success && result.data) {
        setMessages(result.data.messages || []);
        onUnreadCount(result.data.unread_count || 0);
        setError(null);
      } else {
        setError(result.error || 'Failed to load inbox');
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Connection error');
    } finally {
      setLoading(false);
    }
  }, [agent, config, onUnreadCount]);

  useEffect(() => {
    loadInbox();
    const timer = setInterval(loadInbox, (config as any).refreshInterval ? (config as any).refreshInterval * 1000 : 30000);
    return () => clearInterval(timer);
  }, [loadInbox]);

  const filteredMessages = searchTerm
    ? messages.filter(m =>
        (m.subject || '').toLowerCase().includes(searchTerm.toLowerCase()) ||
        (m.from_agent_display || m.from_agent || '').toLowerCase().includes(searchTerm.toLowerCase())
      )
    : messages;

  useSafeInput((input, key) => {
    if (searching) return; // handled by search mode

    if (input === 'j' || key.downArrow) {
      setSelectedIndex(i => Math.min(i + 1, filteredMessages.length - 1));
      return;
    }
    if (input === 'k' || key.upArrow) {
      setSelectedIndex(i => Math.max(i - 1, 0));
      return;
    }
    if (key.return && filteredMessages[selectedIndex]) {
      onReadMessage(filteredMessages[selectedIndex]!);
      return;
    }
  });

  if (loading) {
    return (
      <Box paddingX={1} paddingY={1}>
        <Text color="yellow">Loading inbox...</Text>
      </Box>
    );
  }

  if (error) {
    return (
      <Box paddingX={1} paddingY={1} flexDirection="column">
        <Text color="red">Error: {error}</Text>
        <Text color="gray">Press I to retry</Text>
      </Box>
    );
  }

  const unread = messages.filter(m => !m.read_at).length;

  return (
    <Box flexDirection="column">
      <Box paddingX={1}>
        <Text bold color="cyan">[I]nbox</Text>
        <Text color="gray">  </Text>
        <Text color="gray">[C]ompose  [S]ent  [A]gents  [Q]uit</Text>
      </Box>
      <MessageList messages={filteredMessages} selectedIndex={selectedIndex} mode="inbox" />
      <Box paddingX={1}>
        <Text color="gray">
          {unread > 0 ? `${unread} unread of ` : ''}{messages.length} total
        </Text>
      </Box>
      <Box paddingX={1}>
        <Text color="gray">[Enter] Read  [R]eply  [F]orward  [C]ompose  [Q]uit</Text>
      </Box>
    </Box>
  );
}
