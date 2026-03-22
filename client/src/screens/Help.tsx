import React from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';

interface HelpProps {
  onClose: () => void;
}

export function Help({ onClose }: HelpProps) {
  useSafeInput((_input, key) => {
    if (key.escape || _input === '?') {
      onClose();
    }
  });

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      <Text bold color="cyan">vibesql-mail Keyboard Shortcuts</Text>
      <Text>{''}</Text>

      <Text bold underline>Global</Text>
      <Box flexDirection="column" paddingX={2}>
        <Text><Text color="cyan" bold>I</Text>       Inbox</Text>
        <Text><Text color="cyan" bold>C</Text>       Compose new message</Text>
        <Text><Text color="cyan" bold>S</Text>       Sent messages</Text>
        <Text><Text color="cyan" bold>A</Text>       Agent directory</Text>
        <Text><Text color="cyan" bold>Q</Text>       Quit</Text>
        <Text><Text color="cyan" bold>?</Text>       This help screen</Text>
        <Text><Text color="cyan" bold>j/k</Text>     Navigate up/down</Text>
        <Text><Text color="cyan" bold>Enter</Text>   Open / select</Text>
        <Text><Text color="cyan" bold>Esc</Text>     Back / cancel</Text>
      </Box>

      <Text>{''}</Text>
      <Text bold underline>Read Message View</Text>
      <Box flexDirection="column" paddingX={2}>
        <Text><Text color="cyan" bold>R</Text>       Reply</Text>
        <Text><Text color="cyan" bold>F</Text>       Forward</Text>
        <Text><Text color="cyan" bold>T</Text>       Thread view</Text>
        <Text><Text color="cyan" bold>j/k</Text>     Scroll body</Text>
      </Box>

      <Text>{''}</Text>
      <Text bold underline>Compose</Text>
      <Box flexDirection="column" paddingX={2}>
        <Text><Text color="cyan" bold>Ctrl+S</Text>     Send message</Text>
        <Text><Text color="cyan" bold>Tab</Text>        Next field / autocomplete</Text>
        <Text><Text color="cyan" bold>Shift+Tab</Text>  Previous field</Text>
        <Text><Text color="cyan" bold>Esc</Text>        Cancel (confirm if body has content)</Text>
      </Box>

      <Text>{''}</Text>
      <Text color="gray">Press Esc or ? to close</Text>
    </Box>
  );
}
