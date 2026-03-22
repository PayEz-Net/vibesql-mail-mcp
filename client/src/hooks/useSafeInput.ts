import { useInput, useStdin } from 'ink';
import type { Key } from 'ink';

/**
 * Wrapper around Ink's useInput that gracefully skips when raw mode
 * is not supported (non-TTY environments, CI, piped stdin).
 */
export function useSafeInput(
  handler: (input: string, key: Key) => void,
  options?: { isActive?: boolean }
) {
  const { isRawModeSupported } = useStdin();
  const isActive = isRawModeSupported === true && options?.isActive !== false;
  useInput(handler, { isActive: isActive ? true : false });
}
