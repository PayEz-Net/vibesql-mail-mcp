import React from 'react';
import { Text } from 'ink';

const MAX_CHARS = 65000;

interface CharCounterProps {
  count: number;
}

export function CharCounter({ count }: CharCounterProps) {
  const pct = count / MAX_CHARS;
  let color: string;
  if (pct >= 0.8) color = 'red';
  else if (pct >= 0.5) color = 'yellow';
  else color = 'green';

  return (
    <Text color={color}>
      {count.toLocaleString()} / {MAX_CHARS.toLocaleString()}
    </Text>
  );
}

export { MAX_CHARS };
