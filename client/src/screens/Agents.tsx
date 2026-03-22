import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';
import { fetchAgents } from '../api.js';
import type { ApiConfig, Agent } from '../api.js';

interface AgentsProps {
  config: ApiConfig;
  onComposeTo: (agentName: string) => void;
}

export function Agents({ config, onComposeTo }: AgentsProps) {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const result = await fetchAgents(config);
        if (result.success && result.data?.agents) {
          setAgents(result.data.agents);
          setError(null);
        } else {
          setError(result.error || 'Failed to load agents');
        }
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : 'Connection error');
      } finally {
        setLoading(false);
      }
    })();
  }, [config]);

  useSafeInput((input, key) => {
    if (input === 'j' || key.downArrow) {
      setSelectedIndex(i => Math.min(i + 1, agents.length - 1));
      return;
    }
    if (input === 'k' || key.upArrow) {
      setSelectedIndex(i => Math.max(i - 1, 0));
      return;
    }
    if (key.return && agents[selectedIndex]) {
      onComposeTo(agents[selectedIndex]!.name);
      return;
    }
  });

  if (loading) {
    return <Box paddingX={1}><Text color="yellow">Loading agents...</Text></Box>;
  }

  if (error) {
    return <Box paddingX={1}><Text color="red">Error: {error}</Text></Box>;
  }

  return (
    <Box flexDirection="column">
      <Box paddingX={1}>
        <Text color="gray">[I]nbox  [C]ompose  [S]ent  </Text>
        <Text bold color="cyan">[A]gents</Text>
        <Text color="gray">  [Q]uit</Text>
      </Box>
      <Box flexDirection="column" paddingX={1} paddingY={1}>
        <Box>
          <Text color="gray">{'  '}{'Name'.padEnd(16)}{'Role'.padEnd(20)}{'Program'}</Text>
        </Box>
        {agents.map((a, i) => {
          const isSelected = i === selectedIndex;
          return (
            <Box key={a.name}>
              <Text color={isSelected ? 'cyan' : undefined}>
                {isSelected ? '\u25B8 ' : '  '}
                {a.name.padEnd(16)}
                {(a.role || '-').padEnd(20)}
                {a.program || '-'}
              </Text>
            </Box>
          );
        })}
      </Box>
      <Box paddingX={1}>
        <Text color="gray">{agents.length} agents | [Enter] Compose to agent</Text>
      </Box>
    </Box>
  );
}
