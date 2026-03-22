import React from 'react';
import { Box, Text } from 'ink';

interface StatusBarProps {
  agent: string;
  server: string;
  unreadCount: number;
}

export function StatusBar({ agent, server, unreadCount }: StatusBarProps) {
  const serverDisplay = server.replace('https://', '').replace('http://', '');

  return (
    <Box borderStyle="single" borderTop={true} borderBottom={false} borderLeft={false} borderRight={false} paddingX={1}>
      <Text bold color="cyan">{agent}</Text>
      <Text color="gray"> | </Text>
      <Text color="gray">{serverDisplay}</Text>
      <Text color="gray"> | </Text>
      <Text color={unreadCount > 0 ? 'yellow' : 'green'}>
        {unreadCount > 0 ? `${unreadCount} unread` : 'all read'}
      </Text>
    </Box>
  );
}
