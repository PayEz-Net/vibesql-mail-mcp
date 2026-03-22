import React, { useState, useMemo } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';
import type { Agent } from '../api.js';

interface AgentPickerProps {
  agents: Agent[];
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isFocused?: boolean;
}

export function AgentPicker({ agents, value, onChange, onSubmit, placeholder, isFocused }: AgentPickerProps) {
  const [showSuggestions, setShowSuggestions] = useState(false);

  const suggestions = useMemo(() => {
    if (!value.trim()) return [];
    const term = value.toLowerCase();
    return agents
      .filter(a => a.name.toLowerCase().startsWith(term))
      .slice(0, 5);
  }, [agents, value]);

  const handleChange = (val: string) => {
    onChange(val);
    setShowSuggestions(val.length > 0);
  };

  return (
    <Box flexDirection="column">
      <Box>
        <TextInput
          value={value}
          onChange={handleChange}
          onSubmit={() => {
            // Tab-complete: if there's exactly one suggestion, use it
            if (suggestions.length === 1) {
              onChange(suggestions[0]!.name);
              setShowSuggestions(false);
            }
            onSubmit();
          }}
          placeholder={placeholder || 'Agent name...'}
          focus={isFocused}
        />
      </Box>
      {showSuggestions && suggestions.length > 0 && isFocused && (
        <Box flexDirection="column" marginLeft={2}>
          {suggestions.map(a => (
            <Text key={a.name} color="gray">
              {a.name}{a.role ? ` (${a.role})` : ''}
            </Text>
          ))}
        </Box>
      )}
    </Box>
  );
}
