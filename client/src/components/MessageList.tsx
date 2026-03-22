import React from 'react';
import { Box, Text } from 'ink';
import type { InboxMessage } from '../api.js';

interface MessageListProps {
  messages: InboxMessage[];
  selectedIndex: number;
  mode: 'inbox' | 'sent';
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  const now = new Date();
  const isToday = d.toDateString() === now.toDateString();

  if (isToday) {
    return d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true })
      .replace(' ', '')
      .toLowerCase();
  }

  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (d.toDateString() === yesterday.toDateString()) {
    return 'Yesterday';
  }

  return d.toLocaleDateString('en-US', { month: 'numeric', day: 'numeric' });
}

function truncate(str: string, max: number): string {
  if (str.length <= max) return str;
  return str.slice(0, max - 1) + '\u2026';
}

export function MessageList({ messages, selectedIndex, mode }: MessageListProps) {
  if (messages.length === 0) {
    return (
      <Box paddingX={1} paddingY={1}>
        <Text color="gray">{mode === 'inbox' ? 'No messages in inbox.' : 'No sent messages.'}</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Header */}
      <Box>
        <Text color="gray">{'  '}</Text>
        <Text color="gray">{'ID'.padEnd(6)}</Text>
        <Text color="gray">{(mode === 'inbox' ? 'From' : 'To').padEnd(14)}</Text>
        <Text color="gray">{'Subject'.padEnd(36)}</Text>
        <Text color="gray">{'Date'.padStart(10)}</Text>
      </Box>
      {/* Messages */}
      {messages.map((msg, i) => {
        const isSelected = i === selectedIndex;
        const isUnread = mode === 'inbox' && !msg.read_at;
        const id = String(msg.message_id || msg.inbox_id || '?');
        const agent = mode === 'inbox'
          ? (msg.from_agent_display || msg.from_agent || '?')
          : (msg.to_agent_display || msg.to_agent || '?');
        const subject = msg.subject || '(no subject)';
        const date = formatDate(msg.created_at);

        return (
          <Box key={msg.message_id || msg.inbox_id || i}>
            <Text color={isSelected ? 'cyan' : undefined}>{isSelected ? '\u25B8 ' : '  '}</Text>
            <Text bold={isUnread} color={isSelected ? 'cyan' : undefined}>
              {id.padEnd(6)}
            </Text>
            <Text bold={isUnread} color={isSelected ? 'cyan' : undefined}>
              {truncate(agent, 13).padEnd(14)}
            </Text>
            <Text bold={isUnread} color={isSelected ? 'cyan' : undefined}>
              {truncate(subject, 35).padEnd(36)}
            </Text>
            <Text color={isSelected ? 'cyan' : 'gray'}>
              {date.padStart(10)}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
