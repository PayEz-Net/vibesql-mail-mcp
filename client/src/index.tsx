import React, { useState, useCallback } from 'react';
import { render, Box, Text, useApp, useStdin } from 'ink';
import { Readable } from 'node:stream';
import { parseCliArgs, getOrCreateConfig, configExists } from './config.js';
import type { AppConfig, CliArgs } from './config.js';
import { StatusBar } from './components/StatusBar.js';
import { Inbox } from './screens/Inbox.js';
import { ReadMessage } from './screens/ReadMessage.js';
import { Compose } from './screens/Compose.js';
import { Sent } from './screens/Sent.js';
import { Agents } from './screens/Agents.js';
import { Thread } from './screens/Thread.js';
import { Help } from './screens/Help.js';
import { SetupWizard } from './screens/SetupWizard.js';
import { useKeyboard } from './hooks/useKeyboard.js';
import type { Screen } from './hooks/useKeyboard.js';
import type { InboxMessage, ApiConfig } from './api.js';

// Parse CLI args and resolve config
const cliArgs: CliArgs = parseCliArgs();
const needsWizard = cliArgs.setup || (!configExists() && !cliArgs.agent);
const initialConfig = needsWizard ? null : getOrCreateConfig(cliArgs.agent);

if (!needsWizard && initialConfig && !initialConfig.agent) {
  console.error('Usage: vibesql-mail --agent <name>');
  console.error('       vibesql-mail --setup');
  process.exit(1);
}

// Detect TTY
const isTTY = !!(process.stdin as any).isTTY && typeof (process.stdin as any).setRawMode === 'function';

interface ComposeState {
  to?: string;
  subject?: string;
  body?: string;
  threadId?: string;
}

function App() {
  const { exit } = useApp();
  const { isRawModeSupported } = useStdin();
  const [appConfig, setAppConfig] = useState<AppConfig | null>(initialConfig);
  const [showWizard, setShowWizard] = useState(needsWizard);
  const [screen, setScreen] = useState<Screen>(needsWizard ? 'wizard' : 'inbox');
  const [selectedMessage, setSelectedMessage] = useState<InboxMessage | null>(null);
  const [unreadCount, setUnreadCount] = useState(0);
  const [composeState, setComposeState] = useState<ComposeState>({});
  const [threadId, setThreadId] = useState<string | null>(null);

  const apiConfig: ApiConfig | null = appConfig ? {
    server: appConfig.server,
    secret: appConfig.secret,
  } : null;

  const handleWizardComplete = useCallback((config: AppConfig) => {
    setAppConfig(config);
    setShowWizard(false);
    setScreen('inbox');
  }, []);

  const handleNavigate = useCallback((s: Screen) => {
    if (s === 'compose') {
      setComposeState({});
    }
    setScreen(s);
  }, []);

  const handleQuit = useCallback(() => {
    exit();
  }, [exit]);

  const handleReadMessage = useCallback((msg: InboxMessage) => {
    setSelectedMessage(msg);
    setScreen('read');
  }, []);

  const handleThreadLoaded = useCallback((tid: string) => {
    setThreadId(tid);
  }, []);

  const handleReply = useCallback(() => {
    if (!selectedMessage || !apiConfig) return;
    const from = selectedMessage.from_agent_display || selectedMessage.from_agent || '';
    const subj = selectedMessage.subject || '';
    const body = selectedMessage.body || '';
    const quoted = body.split('\n').map(l => `> ${l}`).join('\n');
    setComposeState({
      to: from,
      subject: subj.startsWith('RE: ') ? subj : `RE: ${subj}`,
      body: `\n\n${quoted}`,
      threadId: threadId || selectedMessage.thread_id,
    });
    setScreen('compose');
  }, [selectedMessage, threadId, apiConfig]);

  const handleForward = useCallback(() => {
    if (!selectedMessage || !apiConfig) return;
    const from = selectedMessage.from_agent_display || selectedMessage.from_agent || '';
    const subj = selectedMessage.subject || '';
    const body = selectedMessage.body || '';
    const header = `--- Forwarded from ${from} ---\nSubject: ${subj}\nDate: ${new Date(selectedMessage.created_at).toLocaleString()}\n\n`;
    setComposeState({
      subject: subj.startsWith('FWD: ') ? subj : `FWD: ${subj}`,
      body: header + body,
      threadId: threadId || selectedMessage.thread_id,
    });
    setScreen('compose');
  }, [selectedMessage, threadId, apiConfig]);

  const handleThread = useCallback(() => {
    const tid = threadId || selectedMessage?.thread_id;
    if (!tid) return;
    setThreadId(tid);
    setScreen('thread');
  }, [selectedMessage, threadId]);

  const handleComposeTo = useCallback((agentName: string) => {
    setComposeState({ to: agentName });
    setScreen('compose');
  }, []);

  const handleComposeDone = useCallback(() => {
    setScreen('inbox');
  }, []);

  useKeyboard({
    currentScreen: screen,
    onNavigate: handleNavigate,
    onQuit: handleQuit,
    onReply: handleReply,
    onForward: handleForward,
    onThread: handleThread,
    disabled: screen === 'compose' || screen === 'wizard',
  });

  // --- Wizard ---
  if (showWizard) {
    return <SetupWizard onComplete={handleWizardComplete} />;
  }

  if (!appConfig || !apiConfig) {
    return (
      <Box paddingX={1}>
        <Text color="red">No config. Run: vibesql-mail --setup</Text>
      </Box>
    );
  }

  // --- Main app ---
  return (
    <Box flexDirection="column">
      <Box paddingX={1}>
        <Text bold color="magenta">vibesql-mail</Text>
        <Text color="gray"> {'\u2500'} </Text>
        <Text bold color="cyan">{appConfig.agent}</Text>
        {appConfig.project && <Text color="gray"> ({appConfig.project})</Text>}
        {!isRawModeSupported && <Text color="yellow"> (read-only: no TTY)</Text>}
      </Box>

      {screen === 'inbox' && (
        <Inbox
          config={apiConfig}
          agent={appConfig.agent}
          onReadMessage={handleReadMessage}
          onUnreadCount={setUnreadCount}
        />
      )}

      {screen === 'read' && selectedMessage && (
        <ReadMessage
          config={apiConfig}
          message={selectedMessage}
          onBack={() => setScreen('inbox')}
          onThreadLoaded={handleThreadLoaded}
        />
      )}

      {screen === 'compose' && isRawModeSupported && (
        <Compose
          config={apiConfig}
          agent={appConfig.agent}
          initialTo={composeState.to}
          initialSubject={composeState.subject}
          initialBody={composeState.body}
          initialThreadId={composeState.threadId}
          onDone={handleComposeDone}
          onCancel={() => setScreen('inbox')}
        />
      )}

      {screen === 'compose' && !isRawModeSupported && (
        <Box paddingX={1}>
          <Text color="yellow">Compose requires an interactive terminal.</Text>
        </Box>
      )}

      {screen === 'sent' && (
        <Sent
          config={apiConfig}
          agent={appConfig.agent}
          onReadMessage={handleReadMessage}
        />
      )}

      {screen === 'agents' && (
        <Agents
          config={apiConfig}
          onComposeTo={handleComposeTo}
        />
      )}

      {screen === 'thread' && threadId && (
        <Thread
          config={apiConfig}
          threadId={threadId}
          onBack={() => setScreen('read')}
        />
      )}

      {screen === 'help' && (
        <Help onClose={() => setScreen('inbox')} />
      )}

      <StatusBar
        agent={appConfig.agent}
        server={appConfig.server}
        unreadCount={unreadCount}
      />
    </Box>
  );
}

const stdinOption = isTTY ? process.stdin : new Readable({ read() {} });

render(<App />, {
  stdin: stdinOption as NodeJS.ReadStream,
  patchConsole: false,
});
