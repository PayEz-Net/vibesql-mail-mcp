import { createHmac } from 'node:crypto';

export interface ApiConfig {
  server: string;
  secret: string;
  clientId?: string;
}

export interface ApiResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
  pagination?: {
    page: number;
    page_size: number;
    total_count: number;
    total_pages: number;
  };
}

export interface InboxMessage {
  inbox_id: number;
  message_id: number;
  from_agent: string;
  from_agent_display: string;
  subject: string;
  body: string;
  body_format: string;
  importance: string;
  recipient_type: string;
  created_at: string;
  read_at: string | null;
  thread_id?: string;
  to_agent?: string;
  to_agent_display?: string;
}

export interface InboxData {
  agent: string;
  messages: InboxMessage[];
  unread_count: number;
}

export interface SentData {
  agent: string;
  messages: InboxMessage[];
}

export interface MessageData {
  id: number;
  from_agent_id: number;
  from_agent: string;
  from_agent_display: string;
  from_user_id?: number;
  thread_id: string;
  subject: string;
  body: string;
  body_format: string;
  importance: string;
  created_at: string;
  to_agent?: string;
  to_agent_display?: string;
  cc?: string[];
}

export interface Agent {
  name: string;
  display_name?: string;
  role?: string;
  program?: string;
  model?: string;
  is_active?: boolean;
}

export interface AgentsData {
  agents: Agent[];
}

export interface SendData {
  message_id: number;
  thread_id: string;
  from_agent: string;
  to: string[];
  cc?: string[];
  subject: string;
  importance: string;
  created_at: string;
}

export interface ThreadData {
  thread_id: string;
  messages: InboxMessage[];
}

// --- URL + Auth ---

/** Production API uses /v1/agentmail, local vibesql-mail-server uses /v1/mail */
function isLegacyApi(config: ApiConfig): boolean {
  return !!config.clientId;
}

function routePrefix(config: ApiConfig): string {
  return isLegacyApi(config) ? '/v1/agentmail' : '/v1/mail';
}

function buildUrl(route: string, config: ApiConfig): string {
  return `${config.server}${routePrefix(config)}${route}`;
}

function hmacSign(method: string, urlPath: string, secret: string): { timestamp: string; signature: string } {
  const timestamp = Math.floor(Date.now() / 1000).toString();
  const signature = createHmac('sha256', Buffer.from(secret, 'base64'))
    .update(`${timestamp}|${method}|${urlPath}`)
    .digest('base64');
  return { timestamp, signature };
}

// --- HTTP ---

export async function apiCall<T = unknown>(
  method: string,
  route: string,
  config: ApiConfig,
  body: unknown = null
): Promise<ApiResponse<T>> {
  const headers: Record<string, string> = {};

  if (isLegacyApi(config)) {
    // Production HMAC auth
    const urlPath = `${routePrefix(config)}${route}`;
    const { timestamp, signature } = hmacSign(method, urlPath, config.secret);
    headers['X-Vibe-Client-Id'] = config.clientId!;
    headers['X-Vibe-Timestamp'] = timestamp;
    headers['X-Vibe-Signature'] = signature;
    if (body) headers['X-Vibe-User-Id'] = '1';
  } else if (config.secret) {
    // Local simple secret auth
    headers['X-Mail-Secret'] = config.secret;
  }

  if (body) {
    headers['Content-Type'] = 'application/json';
  }

  const url = buildUrl(route, config);
  const response = await fetch(url, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });

  const text = await response.text();
  if (!text) {
    return { success: false, error: `Empty response (${response.status})` };
  }
  try {
    const parsed = JSON.parse(text);
    // Normalize error to string — production API returns {code, message} objects
    if (parsed.error && typeof parsed.error !== 'string') {
      parsed.error = parsed.error.message || parsed.error.code || JSON.stringify(parsed.error);
    }
    return parsed as ApiResponse<T>;
  } catch {
    return { success: false, error: `Invalid JSON response (${response.status})` };
  }
}

// --- Convenience API methods ---

export function fetchInbox(agent: string, config: ApiConfig) {
  return apiCall<InboxData>('GET', `/inbox/${agent}`, config);
}

export function fetchMessage(id: number, config: ApiConfig) {
  return apiCall<MessageData>('GET', `/messages/${id}`, config);
}

export function markRead(id: number, config: ApiConfig) {
  return apiCall('POST', `/messages/${id}/read`, config);
}

export function fetchSent(agent: string, config: ApiConfig) {
  return apiCall<SentData>('GET', `/sent/${agent}`, config);
}

export function fetchAgents(config: ApiConfig) {
  return apiCall<AgentsData>('GET', '/agents', config);
}

export function fetchThread(threadId: string, config: ApiConfig) {
  return apiCall<ThreadData>('GET', `/threads/${threadId}`, config);
}

export function registerAgent(config: ApiConfig, agent: { name: string; role: string; profile: string }) {
  return apiCall('POST', '/agents', config, agent);
}

export function sendMessage(
  config: ApiConfig,
  payload: {
    from_agent: string;
    to: string[];
    cc?: string[];
    subject: string;
    body: string;
    thread_id?: string;
    importance?: string;
  }
) {
  return apiCall<SendData>('POST', '/send', config, payload);
}
