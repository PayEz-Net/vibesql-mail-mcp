import { useSafeInput } from './useSafeInput.js';

export type Screen = 'inbox' | 'read' | 'compose' | 'sent' | 'agents' | 'thread' | 'help' | 'wizard';

interface UseKeyboardOptions {
  currentScreen: Screen;
  onNavigate: (screen: Screen) => void;
  onQuit: () => void;
  onReply?: () => void;
  onForward?: () => void;
  onThread?: () => void;
  disabled?: boolean;
}

export function useKeyboard({
  currentScreen,
  onNavigate,
  onQuit,
  onReply,
  onForward,
  onThread,
  disabled,
}: UseKeyboardOptions) {
  useSafeInput((input, key) => {
    if (disabled) return;

    // Compose screen handles its own input
    if (currentScreen === 'compose' || currentScreen === 'wizard') return;

    if (key.escape) {
      if (currentScreen === 'help') {
        onNavigate('inbox');
        return;
      }
      if (currentScreen !== 'inbox') {
        onNavigate('inbox');
        return;
      }
    }

    const ch = input.toLowerCase();

    if (ch === 'q') {
      onQuit();
      return;
    }

    if (ch === 'i') { onNavigate('inbox'); return; }
    if (ch === 'c') { onNavigate('compose'); return; }
    if (ch === 's') { onNavigate('sent'); return; }
    if (ch === 'a') { onNavigate('agents'); return; }
    if (ch === '?') { onNavigate('help'); return; }

    // Context-dependent keys (only from read view)
    if (currentScreen === 'read') {
      if (ch === 'r' && onReply) { onReply(); return; }
      if (ch === 'f' && onForward) { onForward(); return; }
      if (ch === 't' && onThread) { onThread(); return; }
    }
  });
}
