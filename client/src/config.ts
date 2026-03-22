import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

export interface AppConfig {
  project?: string;
  agent: string;
  server: string;
  secret: string;
  clientId?: string;
  refreshInterval: number;
  theme: 'dark' | 'light';
}

const CONFIG_DIR = join(homedir(), '.vibesql-mail');
const CONFIG_FILE = join(CONFIG_DIR, 'config.json');

export function configExists(): boolean {
  return existsSync(CONFIG_FILE);
}

export function loadConfig(): AppConfig | null {
  try {
    if (!existsSync(CONFIG_FILE)) return null;
    const raw = readFileSync(CONFIG_FILE, 'utf8');
    return JSON.parse(raw) as AppConfig;
  } catch {
    return null;
  }
}

export function saveConfig(config: AppConfig): void {
  mkdirSync(CONFIG_DIR, { recursive: true });
  writeFileSync(CONFIG_FILE, JSON.stringify(config, null, 2), 'utf8');
}

export function getOrCreateConfig(agentOverride?: string): AppConfig | null {
  const existing = loadConfig();

  // Env var overrides
  const envServer = process.env['VIBESQL_MAIL_SERVER'];
  const envSecret = process.env['VIBESQL_MAIL_SECRET'];
  const envAgent = process.env['AGENT_NAME'];

  if (existing) {
    return {
      ...existing,
      agent: agentOverride || envAgent || existing.agent,
      server: envServer || existing.server,
      secret: envSecret || existing.secret,
    };
  }

  // No config — wizard needed (unless --agent was passed for quick start)
  if (agentOverride || envAgent) {
    const config: AppConfig = {
      agent: agentOverride || envAgent || '',
      server: envServer || 'http://localhost:5188',
      secret: envSecret || '',
      refreshInterval: 30,
      theme: 'dark',
    };
    saveConfig(config);
    return config;
  }

  return null;
}

export interface CliArgs {
  agent?: string;
  setup?: boolean;
}

export function parseCliArgs(): CliArgs {
  const args = process.argv.slice(2);
  const result: CliArgs = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--agent' && args[i + 1]) {
      result.agent = args[++i];
    }
    if (args[i] === '--setup') {
      result.setup = true;
    }
  }
  return result;
}
