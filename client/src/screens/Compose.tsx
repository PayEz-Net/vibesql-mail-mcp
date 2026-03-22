import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { ComposeForm } from '../components/ComposeForm.js';
import { fetchAgents } from '../api.js';
import type { ApiConfig, Agent } from '../api.js';

interface ComposeProps {
  config: ApiConfig;
  agent: string;
  initialTo?: string;
  initialSubject?: string;
  initialBody?: string;
  initialThreadId?: string;
  onDone: () => void;
  onCancel: () => void;
}

export function Compose({
  config,
  agent,
  initialTo,
  initialSubject,
  initialBody,
  initialThreadId,
  onDone,
  onCancel,
}: ComposeProps) {
  const [agents, setAgents] = useState<Agent[]>([]);

  useEffect(() => {
    (async () => {
      try {
        const result = await fetchAgents(config);
        if (result.success && result.data?.agents) {
          setAgents(result.data.agents);
        }
      } catch {
        // Agent list unavailable — compose still works
      }
    })();
  }, [config]);

  return (
    <ComposeForm
      config={config}
      agent={agent}
      agents={agents}
      initialTo={initialTo}
      initialSubject={initialSubject}
      initialBody={initialBody}
      initialThreadId={initialThreadId}
      onSent={onDone}
      onCancel={onCancel}
    />
  );
}
