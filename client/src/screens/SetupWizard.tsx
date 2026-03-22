import React, { useState } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';
import { readdirSync, readFileSync } from 'node:fs';
import { join, basename } from 'node:path';
import { useSafeInput } from '../hooks/useSafeInput.js';
import { saveConfig } from '../config.js';
import { registerAgent } from '../api.js';
import type { AppConfig } from '../config.js';
import type { ApiConfig } from '../api.js';

// --- Default agent profiles ---

interface AgentProfile {
  name: string;
  role: string;
  profile: string;
}

const DEFAULT_AGENTS: AgentProfile[] = [
  {
    name: 'Strategist',
    role: 'Product & Business Strategy',
    profile:
      'Product and business strategy lead. Focuses on market fit, user needs, ' +
      'and the "why" behind decisions. Challenges assumptions, asks hard ' +
      'questions, keeps the team aligned on goals.',
  },
  {
    name: 'Engineer',
    role: 'Architecture & Code',
    profile:
      'Architecture and code specialist. Handles system design, feasibility ' +
      'analysis, and technical implementation. Pragmatic — picks the simplest ' +
      'approach that works, flags complexity early.',
  },
  {
    name: 'Designer',
    role: 'UX & Experience',
    profile:
      'UX and experience advocate. Emphasizes user perspective, simplicity, ' +
      'and usability. Pushes back on features that add confusion. Thinks in ' +
      'flows, not screens.',
  },
];

// --- Wizard steps ---

type Step = 'project' | 'agent-mode' | 'custom-path' | 'confirm-agents' | 'pick-agent' | 'server-check' | 'registering' | 'secret' | 'done';

interface SetupWizardProps {
  onComplete: (config: AppConfig) => void;
}

function loadCustomAgents(dirPath: string): AgentProfile[] {
  try {
    const resolved = dirPath.startsWith('~')
      ? join(process.env['HOME'] || process.env['USERPROFILE'] || '', dirPath.slice(1))
      : dirPath;
    const files = readdirSync(resolved).filter(f => f.endsWith('.md'));
    return files.map(f => {
      const content = readFileSync(join(resolved, f), 'utf8');
      const name = basename(f, '.md');
      const roleMatch = content.match(/##\s*Role\s*\n+(.+)/i);
      const role = roleMatch ? roleMatch[1]!.trim() : 'Agent';
      return { name, role, profile: content };
    });
  } catch {
    return [];
  }
}

function fileSizeLabel(profile: string): string {
  const bytes = Buffer.byteLength(profile, 'utf8');
  if (bytes < 1024) return `${bytes}B`;
  return `${(bytes / 1024).toFixed(1)}KB`;
}

async function checkServer(url: string): Promise<boolean> {
  try {
    const res = await fetch(`${url}/v1/mail/agents`, { method: 'GET', signal: AbortSignal.timeout(3000) });
    return res.ok || res.status === 401; // 401 = server is up, just needs auth
  } catch {
    return false;
  }
}

async function checkDatabase(url: string): Promise<boolean> {
  try {
    const res = await fetch(`${url}/v1/query`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ sql: 'SELECT 1' }),
      signal: AbortSignal.timeout(3000),
    });
    return res.ok;
  } catch {
    return false;
  }
}

export function SetupWizard({ onComplete }: SetupWizardProps) {
  const [step, setStep] = useState<Step>('project');
  const [project, setProject] = useState('');
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [customPath, setCustomPath] = useState('./agents/');
  const [selectedAgentIdx, setSelectedAgentIdx] = useState(0);
  const [server, setServer] = useState('http://localhost:5188');
  const [dbUrl, setDbUrl] = useState('http://localhost:5173');
  const [secret, setSecret] = useState('');
  const [error, setError] = useState('');
  const [serverOk, setServerOk] = useState(false);
  const [dbOk, setDbOk] = useState(false);
  const [checking, setChecking] = useState(false);
  const [registerStatus, setRegisterStatus] = useState<string[]>([]);
  const [registerError, setRegisterError] = useState('');

  // --- Step handlers ---

  const submitProject = () => {
    if (!project.trim()) { setError('Project name required'); return; }
    setError('');
    setStep('agent-mode');
  };

  const submitCustomPath = () => {
    const loaded = loadCustomAgents(customPath);
    if (loaded.length === 0) {
      setError(`No .md files found in ${customPath}`);
      return;
    }
    setError('');
    setAgents(loaded);
    setStep('confirm-agents');
  };

  const runServerCheck = async () => {
    setChecking(true);
    setError('');
    const [sOk, dOk] = await Promise.all([
      checkServer(server),
      checkDatabase(dbUrl),
    ]);
    setServerOk(sOk);
    setDbOk(dOk);
    setChecking(false);

    if (!sOk || !dOk) {
      const parts: string[] = [];
      if (!sOk) parts.push(`Mail server not found at ${server}. Start it with: VIBESQL_MAIL_DEV=true npx vibesql-mail-server`);
      if (!dOk) parts.push(`Database not found at ${dbUrl}. Start it with: npx vibesql-micro`);
      setError(parts.join('\n'));
      return;
    }

    // Both OK — register agents
    setStep('registering');
    await registerAgents();
  };

  const registerAgents = async () => {
    const apiConfig: ApiConfig = { server, secret };
    const statuses: string[] = [];
    for (const agent of agents) {
      try {
        const result = await registerAgent(apiConfig, { name: agent.name, role: agent.role, profile: agent.profile });
        if (result.success || result.error?.includes('already exists')) {
          statuses.push(`Registering ${agent.name}... done`);
        } else {
          statuses.push(`Registering ${agent.name}... failed: ${result.error}`);
          setRegisterError(result.error || 'Registration failed');
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : 'unknown error';
        statuses.push(`Registering ${agent.name}... failed: ${msg}`);
        setRegisterError(msg);
      }
      setRegisterStatus([...statuses]);
    }

    if (!statuses.some(s => s.includes('failed'))) {
      setStep('secret');
    }
  };

  const submitSecret = () => {
    setStep('done');
    const chosen = agents[selectedAgentIdx];
    const config: AppConfig = {
      project: project.trim(),
      agent: chosen?.name || 'Agent',
      server: server.trim(),
      secret: secret.trim(),
      refreshInterval: 30,
      theme: 'dark',
    };
    saveConfig(config);
    setTimeout(() => onComplete(config), 500);
  };

  // --- Keyboard for selection steps ---

  useSafeInput((input, key) => {
    if (step === 'agent-mode') {
      if (input === '1') {
        setAgents(DEFAULT_AGENTS);
        setStep('pick-agent');
        return;
      }
      if (input === '2') {
        setStep('custom-path');
        return;
      }
    }

    if (step === 'confirm-agents') {
      const ch = input.toLowerCase();
      if (ch === 'y' || key.return) {
        setStep('pick-agent');
        return;
      }
      if (ch === 'n') {
        setStep('custom-path');
        return;
      }
    }

    if (step === 'pick-agent') {
      if (input === 'j' || key.downArrow) {
        setSelectedAgentIdx(i => Math.min(i + 1, agents.length - 1));
        return;
      }
      if (input === 'k' || key.upArrow) {
        setSelectedAgentIdx(i => Math.max(i - 1, 0));
        return;
      }
      if (key.return) {
        runServerCheck();
        return;
      }
    }

    if (step === 'server-check' && error) {
      if (input === 'r') {
        runServerCheck();
        return;
      }
    }
  }, { isActive: step !== 'project' && step !== 'custom-path' && step !== 'secret' });

  // --- Render ---

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      <Text bold color="magenta">vibesql-mail Setup</Text>
      <Text>{''}</Text>

      {/* Step 1: Project name */}
      {step === 'project' && (
        <Box flexDirection="column">
          <Text>Welcome to vibesql-mail!</Text>
          <Text>{''}</Text>
          <Box>
            <Text color="cyan">Project name: </Text>
            <TextInput
              value={project}
              onChange={setProject}
              onSubmit={submitProject}
              placeholder="My Project"
              focus={true}
            />
          </Box>
        </Box>
      )}

      {/* Step 2: Agent mode */}
      {step === 'agent-mode' && (
        <Box flexDirection="column">
          <Text>How would you like to set up your agents?</Text>
          <Text>{''}</Text>
          <Text>  <Text color="cyan" bold>1</Text>  Use default team (Strategist, Engineer, Designer)</Text>
          <Text>  <Text color="cyan" bold>2</Text>  Provide your own agent profile markdown files</Text>
          <Text>{''}</Text>
          <Text color="gray">Press 1 or 2</Text>
        </Box>
      )}

      {/* Step 2b: Custom path */}
      {step === 'custom-path' && (
        <Box flexDirection="column">
          <Text>Path to agent profiles directory:</Text>
          <Text color="gray">Each .md file = one agent. Filename = agent name.</Text>
          <Text>{''}</Text>
          <Box>
            <Text color="cyan">Path: </Text>
            <TextInput
              value={customPath}
              onChange={setCustomPath}
              onSubmit={submitCustomPath}
              focus={true}
            />
          </Box>
        </Box>
      )}

      {/* Step 2c: Confirm custom agents */}
      {step === 'confirm-agents' && (
        <Box flexDirection="column">
          <Text>Found {agents.length} agent profiles:</Text>
          <Text>{''}</Text>
          {agents.map(a => (
            <Text key={a.name}>
              {'  '}<Text bold>{a.name}</Text> — {a.name}.md ({fileSizeLabel(a.profile)})
            </Text>
          ))}
          <Text>{''}</Text>
          <Text>Register these agents? <Text color="cyan" bold>[Y/n]</Text></Text>
        </Box>
      )}

      {/* Step 3: Pick agent */}
      {step === 'pick-agent' && (
        <Box flexDirection="column">
          <Text>Which agent are you?</Text>
          <Text>{''}</Text>
          {agents.map((a, i) => (
            <Text key={a.name} color={i === selectedAgentIdx ? 'cyan' : undefined}>
              {i === selectedAgentIdx ? ' \u25B8 ' : '   '}
              <Text bold={i === selectedAgentIdx}>{a.name}</Text>
              <Text color="gray"> — {a.role}</Text>
            </Text>
          ))}
          <Text>{''}</Text>
          <Text color="gray">j/k to navigate, Enter to select</Text>
        </Box>
      )}

      {/* Step 4: Server check */}
      {(step === 'server-check' || (step === 'pick-agent' && checking)) && (
        <Box flexDirection="column">
          <Text>{''}</Text>
          {checking ? (
            <>
              <Text color="yellow">Checking mail server at {server}...</Text>
              <Text color="yellow">Checking database at {dbUrl}...</Text>
            </>
          ) : (
            <>
              <Text color={serverOk ? 'green' : 'red'}>
                Mail server at {server}... {serverOk ? 'OK' : 'FAILED'}
              </Text>
              <Text color={dbOk ? 'green' : 'red'}>
                Database at {dbUrl}... {dbOk ? 'OK' : 'FAILED'}
              </Text>
            </>
          )}
        </Box>
      )}

      {/* Step 5: Registering agents */}
      {step === 'registering' && (
        <Box flexDirection="column">
          <Text>{''}</Text>
          {registerStatus.map((s, i) => (
            <Text key={i} color={s.includes('failed') ? 'red' : 'green'}>  {s}</Text>
          ))}
          {registerError && (
            <Box marginTop={1}>
              <Text color="red">Registration failed. Check the server and try again.</Text>
            </Box>
          )}
        </Box>
      )}

      {/* Step 6: Secret */}
      {step === 'secret' && (
        <Box flexDirection="column">
          <Text>Shared secret (leave blank for no auth):</Text>
          <Text>{''}</Text>
          <Box>
            <Text color="cyan">Secret: </Text>
            <TextInput
              value={secret}
              onChange={setSecret}
              onSubmit={submitSecret}
              focus={true}
            />
          </Box>
        </Box>
      )}

      {/* Step 7: Done */}
      {step === 'done' && (
        <Box flexDirection="column">
          <Text bold color="green">Setup complete!</Text>
          <Text>{''}</Text>
          <Text>  Project: <Text bold>{project}</Text></Text>
          <Text>  Agents:  <Text bold>{agents.length} configured</Text></Text>
          <Text>  You are: <Text bold color="cyan">{agents[selectedAgentIdx]?.name}</Text></Text>
          <Text>  Server:  <Text bold>{server}</Text></Text>
          {secret && <Text>  Auth:    <Text bold>shared secret</Text></Text>}
          {!secret && <Text>  Auth:    <Text color="gray">none (dev mode)</Text></Text>}
          <Text>{''}</Text>
          <Text color="gray">Config saved to ~/.vibesql-mail/config.json</Text>
        </Box>
      )}

      {/* Error */}
      {error && (
        <Box marginTop={1} flexDirection="column">
          {error.split('\n').map((line, i) => (
            <Text key={i} color="red">{line}</Text>
          ))}
          {(step === 'server-check' || step === 'pick-agent') && !checking && (
            <Text color="gray" italic>Press r to retry</Text>
          )}
        </Box>
      )}

      {/* Progress indicator */}
      {step !== 'done' && step !== 'registering' && (
        <Box marginTop={1}>
          <Text color="gray">
            Step {
              step === 'project' ? '1' :
              step === 'agent-mode' || step === 'custom-path' || step === 'confirm-agents' ? '2' :
              step === 'pick-agent' ? '3' :
              step === 'server-check' ? '4' : '5'
            } of 5
          </Text>
        </Box>
      )}
    </Box>
  );
}
