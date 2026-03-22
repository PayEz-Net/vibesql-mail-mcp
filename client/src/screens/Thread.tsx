import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';
import { fetchThread } from '../api.js';
import type { ApiConfig, InboxMessage } from '../api.js';

interface ThreadProps {
  config: ApiConfig;
  threadId: string;
  onBack: () => void;
}

export function Thread({ config, threadId, onBack }: ThreadProps) {
  const [messages, setMessages] = useState<InboxMessage[]>([]);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const result = await fetchThread(threadId, config);
        if (result.success && result.data?.messages) {
          setMessages(result.data.messages);
          setError(null);
        } else {
          setError(result.error || 'Failed to load thread');
        }
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : 'Connection error');
      } finally {
        setLoading(false);
      }
    })();
  }, [threadId, config]);

  useSafeInput((input, key) => {
    if (key.escape) { onBack(); return; }
    if (input === 'j' || key.downArrow) { setScrollOffset(o => o + 1); return; }
    if (input === 'k' || key.upArrow) { setScrollOffset(o => Math.max(0, o - 1)); return; }
  });

  if (loading) {
    return <Box paddingX={1}><Text color="yellow">Loading thread...</Text></Box>;
  }

  if (error) {
    return <Box paddingX={1}><Text color="red">Error: {error}</Text></Box>;
  }

  return (
    <Box flexDirection="column" paddingX={1}>
      <Box marginBottom={1}>
        <Text bold color="cyan">Thread: {threadId}</Text>
        <Text color="gray"> ({messages.length} messages)</Text>
      </Box>

      {messages.slice(scrollOffset, scrollOffset + 10).map((msg, i) => {
        const from = msg.from_agent_display || msg.from_agent || '?';
        const body = msg.body || '';
        const preview = body.length > 120 ? body.slice(0, 117) + '...' : body;

        return (
          <Box key={msg.message_id || i} flexDirection="column" marginBottom={1}>
            <Box>
              <Text bold>{from}</Text>
              <Text color="gray"> - {new Date(msg.created_at).toLocaleString()}</Text>
            </Box>
            <Box>
              <Text color="gray">  {msg.subject}</Text>
            </Box>
            <Box>
              <Text>  {preview}</Text>
            </Box>
          </Box>
        );
      })}

      <Box borderStyle="single" borderTop={true} borderBottom={false} borderLeft={false} borderRight={false}>
        <Text color="gray">[Esc] Back  [j/k] Scroll</Text>
      </Box>
    </Box>
  );
}
