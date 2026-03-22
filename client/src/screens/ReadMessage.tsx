import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';
import { fetchMessage, markRead } from '../api.js';
import type { ApiConfig, InboxMessage, MessageData } from '../api.js';

interface ReadMessageProps {
  config: ApiConfig;
  message: InboxMessage;
  onBack: () => void;
  onThreadLoaded?: (threadId: string) => void;
}

export function ReadMessage({ config, message, onBack, onThreadLoaded }: ReadMessageProps) {
  const [fullMessage, setFullMessage] = useState<MessageData | null>(null);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const result = await fetchMessage(message.message_id, config);
        if (result.success && result.data) {
          setFullMessage(result.data);
          if (result.data.thread_id && onThreadLoaded) {
            onThreadLoaded(result.data.thread_id);
          }
        }
        // Mark as read
        markRead(message.message_id, config).catch(() => {});
      } catch (err) {
        // fallback to inbox data — log to stderr so user can debug
        if (err instanceof Error) process.stderr.write(`ReadMessage fetch error: ${err.message}\n`);
      } finally {
        setLoading(false);
      }
    })();
  }, [message.message_id, config, onThreadLoaded]);

  useSafeInput((input, key) => {
    if (key.escape) {
      onBack();
      return;
    }
    if (input === 'j' || key.downArrow) {
      setScrollOffset(o => o + 1);
      return;
    }
    if (input === 'k' || key.upArrow) {
      setScrollOffset(o => Math.max(0, o - 1));
      return;
    }
    if (key.pageDown) {
      setScrollOffset(o => o + 10);
      return;
    }
    if (key.pageUp) {
      setScrollOffset(o => Math.max(0, o - 10));
      return;
    }
  });

  const from = fullMessage?.from_agent_display || fullMessage?.from_agent || message.from_agent_display || message.from_agent || '?';
  const to = fullMessage?.to_agent_display || fullMessage?.to_agent || '?';
  const threadId = fullMessage?.thread_id || message.thread_id;
  const rawBody = fullMessage?.body || message.body || '';
  const body = rawBody;
  const subject = fullMessage?.subject || message.subject || '(no subject)';
  const createdAt = fullMessage?.created_at || message.created_at;
  const bodyLines = body.split('\n');
  const visibleLines = bodyLines.slice(scrollOffset, scrollOffset + 20);

  if (loading) {
    return (
      <Box paddingX={1} paddingY={1}>
        <Text color="yellow">Loading message...</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" paddingX={1}>
      <Box marginBottom={0}>
        <Text bold color="cyan">Message #{message.message_id}</Text>
      </Box>

      <Box borderStyle="single" borderBottom={true} borderTop={false} borderLeft={false} borderRight={false}>
        <Box flexDirection="column">
          <Box>
            <Text color="gray">From: </Text>
            <Text bold>{from}</Text>
            <Text color="gray">        To: </Text>
            <Text>{to}</Text>
          </Box>
          <Box>
            <Text color="gray">Date: </Text>
            <Text>{new Date(createdAt).toLocaleString()}</Text>
            {threadId && (
              <>
                <Text color="gray">    Thread: </Text>
                <Text color="gray">{threadId}</Text>
              </>
            )}
          </Box>
          <Box>
            <Text color="gray">Subject: </Text>
            <Text bold>{subject}</Text>
          </Box>
        </Box>
      </Box>

      <Box flexDirection="column" paddingY={1} minHeight={5}>
        {visibleLines.map((line, i) => (
          <Text key={i}>{line}</Text>
        ))}
        {scrollOffset > 0 && (
          <Text color="gray">(scrolled {scrollOffset} lines down)</Text>
        )}
      </Box>

      <Box borderStyle="single" borderTop={true} borderBottom={false} borderLeft={false} borderRight={false}>
        <Text color="gray">[R]eply  [F]orward  [T]hread  [Esc] Back  [j/k] Scroll</Text>
      </Box>
    </Box>
  );
}
