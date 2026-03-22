import React, { useState, useEffect, useCallback } from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';
import { MessageList } from '../components/MessageList.js';
import { fetchSent } from '../api.js';
import type { ApiConfig, InboxMessage } from '../api.js';

interface SentProps {
  config: ApiConfig;
  agent: string;
  onReadMessage: (msg: InboxMessage) => void;
}

export function Sent({ config, agent, onReadMessage }: SentProps) {
  const [messages, setMessages] = useState<InboxMessage[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSent = useCallback(async () => {
    try {
      const result = await fetchSent(agent, config);
      if (result.success && result.data) {
        setMessages(result.data.messages || []);
        setError(null);
      } else {
        setError(result.error || 'Failed to load sent messages');
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Connection error');
    } finally {
      setLoading(false);
    }
  }, [agent, config]);

  useEffect(() => { loadSent(); }, [loadSent]);

  useSafeInput((input, key) => {
    if (input === 'j' || key.downArrow) {
      setSelectedIndex(i => Math.min(i + 1, messages.length - 1));
      return;
    }
    if (input === 'k' || key.upArrow) {
      setSelectedIndex(i => Math.max(i - 1, 0));
      return;
    }
    if (key.return && messages[selectedIndex]) {
      onReadMessage(messages[selectedIndex]!);
      return;
    }
  });

  if (loading) {
    return <Box paddingX={1}><Text color="yellow">Loading sent...</Text></Box>;
  }

  if (error) {
    return <Box paddingX={1}><Text color="red">Error: {error}</Text></Box>;
  }

  return (
    <Box flexDirection="column">
      <Box paddingX={1}>
        <Text color="gray">[I]nbox  [C]ompose  </Text>
        <Text bold color="cyan">[S]ent</Text>
        <Text color="gray">  [A]gents  [Q]uit</Text>
      </Box>
      <MessageList messages={messages} selectedIndex={selectedIndex} mode="sent" />
      <Box paddingX={1}>
        <Text color="gray">{messages.length} sent messages</Text>
      </Box>
    </Box>
  );
}
