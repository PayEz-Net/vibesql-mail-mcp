import { useState, useCallback } from 'react';
import type { ApiConfig, ApiResponse } from '../api.js';

interface UseApiState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

export function useApi<T>() {
  const [state, setState] = useState<UseApiState<T>>({
    data: null,
    loading: false,
    error: null,
  });

  const call = useCallback(async (fn: () => Promise<ApiResponse<T>>) => {
    setState({ data: null, loading: true, error: null });
    try {
      const result = await fn();
      if (result.success && result.data) {
        setState({ data: result.data, loading: false, error: null });
        return result;
      } else {
        setState({ data: null, loading: false, error: result.error || 'Request failed' });
        return result;
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Unknown error';
      setState({ data: null, loading: false, error: msg });
      return { success: false, error: msg } as ApiResponse<T>;
    }
  }, []);

  return { ...state, call };
}
