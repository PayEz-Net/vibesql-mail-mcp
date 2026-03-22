import React, { useState, useCallback } from 'react';
import { Box, Text } from 'ink';
import { useSafeInput } from '../hooks/useSafeInput.js';
import TextInput from 'ink-text-input';
import { AgentPicker } from './AgentPicker.js';
import { CharCounter, MAX_CHARS } from './CharCounter.js';
import { sendMessage } from '../api.js';
import type { ApiConfig, Agent } from '../api.js';

type Field = 'to' | 'cc' | 'subject' | 'body';
const FIELDS: Field[] = ['to', 'cc', 'subject', 'body'];

interface ComposeFormProps {
  config: ApiConfig;
  agent: string;
  agents: Agent[];
  initialTo?: string;
  initialSubject?: string;
  initialBody?: string;
  initialThreadId?: string;
  onSent: () => void;
  onCancel: () => void;
}

export function ComposeForm({
  config,
  agent,
  agents,
  initialTo = '',
  initialSubject = '',
  initialBody = '',
  initialThreadId,
  onSent,
  onCancel,
}: ComposeFormProps) {
  const [to, setTo] = useState(initialTo);
  const [cc, setCc] = useState('');
  const [subject, setSubject] = useState(initialSubject);
  const [body, setBody] = useState(initialBody);
  const [focusedField, setFocusedField] = useState<Field>('to');
  const [status, setStatus] = useState('');
  const [sending, setSending] = useState(false);

  const focusIndex = FIELDS.indexOf(focusedField);

  const nextField = useCallback(() => {
    const next = FIELDS[(focusIndex + 1) % FIELDS.length];
    if (next) setFocusedField(next);
  }, [focusIndex]);

  const prevField = useCallback(() => {
    const prev = FIELDS[(focusIndex - 1 + FIELDS.length) % FIELDS.length];
    if (prev) setFocusedField(prev);
  }, [focusIndex]);

  const handleSend = useCallback(async () => {
    if (!to.trim()) { setStatus('To field required'); return; }
    if (!subject.trim()) { setStatus('Subject required'); return; }
    if (!body.trim()) { setStatus('Body required'); return; }
    if (sending) return;

    setSending(true);
    setStatus('Sending...');

    try {
      const toList = to.split(',').map(s => s.trim()).filter(Boolean);
      const ccList = cc ? cc.split(',').map(s => s.trim()).filter(Boolean) : undefined;

      const result = await sendMessage(config, {
        from_agent: agent,
        to: toList,
        cc: ccList,
        subject,
        body,
        thread_id: initialThreadId,
      });

      if (result.success) {
        setStatus(`Sent! (ID: ${result.data?.message_id})`);
        setTimeout(onSent, 800);
      } else {
        setStatus(`Error: ${result.error || 'Send failed'}`);
        setSending(false);
      }
    } catch (err: unknown) {
      setStatus(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`);
      setSending(false);
    }
  }, [to, cc, subject, body, sending, config, agent, initialThreadId, onSent]);

  useSafeInput((input, key) => {
    if (key.escape) {
      if (body.trim()) {
        setStatus('Press Esc again to discard');
        if (status.startsWith('Press Esc')) {
          onCancel();
        }
      } else {
        onCancel();
      }
      return;
    }

    // Clear "press Esc again" status on any other input
    if (status.startsWith('Press Esc')) {
      setStatus('');
    }

    if (key.ctrl && input === 's') {
      handleSend();
      return;
    }

    if (key.tab) {
      if (key.shift) {
        prevField();
      } else {
        nextField();
      }
      return;
    }
  });

  const setBodyCapped = (val: string) => {
    if (val.length <= MAX_CHARS) setBody(val);
  };

  return (
    <Box flexDirection="column" paddingX={1}>
      <Box marginBottom={1}>
        <Text bold color="cyan">Compose</Text>
        {initialThreadId && <Text color="gray"> (Thread: {initialThreadId})</Text>}
      </Box>

      <Box>
        <Text color="gray">From: </Text>
        <Text bold>{agent}</Text>
      </Box>

      <Box>
        <Text color={focusedField === 'to' ? 'cyan' : 'gray'}>To:   </Text>
        <AgentPicker
          agents={agents}
          value={to}
          onChange={setTo}
          onSubmit={nextField}
          isFocused={focusedField === 'to'}
        />
      </Box>

      <Box>
        <Text color={focusedField === 'cc' ? 'cyan' : 'gray'}>Cc:   </Text>
        <AgentPicker
          agents={agents}
          value={cc}
          onChange={setCc}
          onSubmit={nextField}
          isFocused={focusedField === 'cc'}
        />
      </Box>

      <Box>
        <Text color={focusedField === 'subject' ? 'cyan' : 'gray'}>Subj: </Text>
        <TextInput
          value={subject}
          onChange={setSubject}
          onSubmit={nextField}
          placeholder="Subject..."
          focus={focusedField === 'subject'}
        />
      </Box>

      <Box marginTop={1} borderStyle="single" borderTop={true} borderBottom={false} borderLeft={false} borderRight={false}>
        <Text color={focusedField === 'body' ? 'cyan' : 'gray'}>Body: </Text>
        <Box flexDirection="column" flexGrow={1}>
          <TextInput
            value={body}
            onChange={setBodyCapped}
            onSubmit={() => {}}
            placeholder="Type your message..."
            focus={focusedField === 'body'}
          />
        </Box>
        <Box marginLeft={2}>
          <CharCounter count={body.length} />
        </Box>
      </Box>

      <Box marginTop={1} borderStyle="single" borderTop={true} borderBottom={false} borderLeft={false} borderRight={false} paddingTop={0}>
        <Text color="gray">Ctrl+S Send  Tab Next Field  Esc Cancel</Text>
      </Box>

      {status && (
        <Box marginTop={0}>
          <Text color={status.startsWith('Error') ? 'red' : status === 'Sending...' ? 'yellow' : 'green'}>
            {status}
          </Text>
        </Box>
      )}
    </Box>
  );
}
