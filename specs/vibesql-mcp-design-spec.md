# @vibesql/mcp — Design Spec

**Author:** NextPert
**Date:** 2026-02-23
**Source:** BAPert assignment #928
**Location:** E:\Repos\vibesql-skills\claude-mcp\

---

## Overview

Standalone MCP server npm package. One command connects any AI coding tool to a local VibeSQL (PostgreSQL 16.1) database. Extracted from vibesql-admin's 26-tool MCP server — keeps 9 database + help tools, drops all ecosystem dependencies (vault, backup, sync, cryptaply).

```bash
npx @vibesql/mcp                          # auto-discovers localhost:5173
npx @vibesql/mcp --url http://10.0.0.5:5173  # custom URL
```

---

## File Structure

```
vibesql-skills/claude-mcp/
  package.json            # @vibesql/mcp, bin: vibesql-mcp
  tsconfig.json           # ESNext, NodeNext, outDir ./dist
  bin/vibesql-mcp.js      # #!/usr/bin/env node, imports ../dist/index.js
  src/
    index.ts              # MCP server setup, tool registration, startup health check
    client.ts             # HTTP client for vibesql-micro
    help.ts               # Embedded help content as string constants
```

---

## Dependencies

```json
{
  "dependencies": {
    "@modelcontextprotocol/sdk": "latest",
    "zod": "^3.23"
  },
  "devDependencies": {
    "typescript": "^5.7",
    "@types/node": "^22.0"
  }
}
```

Nothing else. No Vue, no Express, no vibesql-admin imports.

---

## src/client.ts — VibeSQL HTTP Client

Extracted from `vibesql-admin/src/services/MicroService.ts` with **critical fixes**:

### Bug Fix: PostgreSQL, Not SQLite

The existing MicroService.ts uses SQLite queries:
- `listTables()` → `SELECT name FROM sqlite_master WHERE type='table'`
- `describeTable()` → `PRAGMA table_info('...')`

**VibeSQL is PostgreSQL 16.1. SQLite does not exist in this ecosystem.**

Fixed queries:
- `listTables()` → `SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name`
- `describeTable()` → `SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position` (parameterized)

### Interface

```typescript
export interface QueryResult {
  columns: string[];
  rows: unknown[][];
  time: number;
}

export class VibeClient {
  constructor(baseUrl: string);

  health(): Promise<{ status: string }>;
  query(sql: string, params?: unknown[]): Promise<QueryResult>;
  listTables(): Promise<QueryResult>;
  describeTable(table: string): Promise<QueryResult>;
}
```

### Protocol

- Endpoint: `POST {baseUrl}/v1/query`
- Body: `{ "sql": "...", "params": [...] }`
- Response: `{ columns: [...], rows: [...], time: 0.003 }`
- Health: `GET {baseUrl}/v1/health`

### Table Name Validation

Keep existing regex validation `^[A-Za-z0-9_]+$` for table names in `describeTable`, `table_data`, `create_table`, `insert_row` to prevent SQL injection.

---

## src/index.ts — MCP Server

### Startup

1. Parse `--url` flag from process.argv (default: `http://localhost:5173`)
2. Also check `VIBESQL_URL` env var (flag takes precedence)
3. Health check: `GET {url}/v1/health`
   - If OK: start MCP server
   - If fail: print to stderr:
     ```
     Could not connect to VibeSQL at http://localhost:5173

     Start it with:  npx vibesql-micro
     Or specify URL:  npx @vibesql/mcp --url http://your-host:5173
     ```
     Then exit(1).
4. Create `McpServer({ name: '@vibesql/mcp', version: '1.0.0' })`
5. Register 9 tools (see below)
6. Connect via `StdioServerTransport`

### Tools (9 total)

#### Database Tools (6)

**1. query**
- Description: "Execute a SQL query against the VibeSQL database"
- Params: `sql: string`, `params?: string` (JSON array)
- Format output as TSV (header + rows) matching vibesql-admin pattern
- Extracted from: vibesql-admin index.ts lines 27-51

**2. list_tables**
- Description: "List all tables in the database"
- No params
- Uses fixed `information_schema.tables` query (NOT sqlite_master)
- Returns JSON array of table names

**3. describe_table**
- Description: "Get column schema for a table"
- Params: `table: string`
- Uses `information_schema.columns` query (NOT PRAGMA)
- Validates table name against regex
- Returns JSON with column_name, data_type, is_nullable, column_default

**4. table_data**
- Description: "Browse rows from a table with pagination"
- Params: `table: string`, `limit?: number` (default 50), `offset?: number` (default 0)
- Builds `SELECT * FROM "{table}" LIMIT {limit} OFFSET {offset}`
- Validates table name against regex
- Extracted from: vibesql-admin index.ts lines 92-116

**5. create_table**
- Description: "Create a new table with the given SQL DDL"
- Params: `sql: string`
- Validates that sql starts with `CREATE TABLE` (case insensitive)
- Runs via `client.query(sql)`
- Returns confirmation with table name

**6. insert_row**
- Description: "Insert a row into a table"
- Params: `table: string`, `data: string` (JSON object of column→value pairs)
- Validates table name
- Builds parameterized INSERT: `INSERT INTO "{table}" (col1, col2) VALUES ($1, $2) RETURNING *`
- Column names validated against regex individually
- Returns the inserted row

#### Help Tools (3)

**7. help**
- Description: "Get help on a VibeSQL topic"
- Params: `topic: string`
- Looks up topic in embedded help map from help.ts
- Available topics: "architecture", "products", "glossary"
- If unknown topic: returns list of available topics

**8. help_products**
- Description: "VibeSQL product family overview"
- No params
- Returns embedded products.md content

**9. help_architecture**
- Description: "VibeSQL architecture patterns and design"
- No params
- Returns embedded architecture.md content

---

## src/help.ts — Embedded Help Content

Three files from `vibesql-admin/src/help/content/` embedded as string constants:

```typescript
export const HELP_TOPICS: Record<string, string> = {
  architecture: `...`,  // from architecture.md
  products: `...`,      // from products.md
  glossary: `...`,      // from glossary.md
};

export function getHelp(topic: string): string {
  return HELP_TOPICS[topic] || `Unknown topic "${topic}". Available: ${Object.keys(HELP_TOPICS).join(', ')}`;
}
```

Only these 3 topics. Vault, backup, sync, cryptaply help excluded — not relevant to a database-only tool.

---

## Claude Code Integration

User adds to `.claude/settings.json` or project-level `.claude/settings.local.json`:

```json
{
  "mcpServers": {
    "vibesql": {
      "command": "npx",
      "args": ["@vibesql/mcp"]
    }
  }
}
```

With custom URL:
```json
{
  "mcpServers": {
    "vibesql": {
      "command": "npx",
      "args": ["@vibesql/mcp", "--url", "http://10.0.0.5:5173"]
    }
  }
}
```

---

## package.json

```json
{
  "name": "@vibesql/mcp",
  "version": "1.0.0",
  "description": "MCP server for VibeSQL — connect your AI coding tool to a PostgreSQL database",
  "type": "module",
  "bin": {
    "vibesql-mcp": "./bin/vibesql-mcp.js"
  },
  "scripts": {
    "build": "tsc",
    "dev": "tsc --watch"
  },
  "keywords": ["vibesql", "mcp", "postgresql", "database", "ai", "claude"],
  "author": "PayEz <opensource@payez.net>",
  "license": "Apache-2.0",
  "dependencies": {
    "@modelcontextprotocol/sdk": "latest",
    "zod": "^3.23"
  },
  "devDependencies": {
    "typescript": "^5.7",
    "@types/node": "^22.0"
  }
}
```

---

## Existing vibesql-skills Content — Untouched

These directories stay as-is:
- `claude/vibe-sql/SKILL.md` — slash command skill (different mechanism)
- `claude/vibe-mail/SKILL.md` — mail slash command
- `opencode/` — OpenCode skills
- `codex/` — Codex CLI skills
- `README.md` — update to mention @vibesql/mcp package

---

## Critical Differences from vibesql-admin MCP

| Aspect | vibesql-admin | @vibesql/mcp |
|--------|---------------|--------------|
| Tools | 26 (db + vault + backup + sync + help) | 9 (db + help only) |
| Dependencies | Vue, Express, config service, 4 service classes | SDK + zod only |
| listTables query | `sqlite_master` (WRONG) | `information_schema.tables` (correct) |
| describeTable query | `PRAGMA table_info` (WRONG) | `information_schema.columns` (correct) |
| Config | loadConfig() from file | CLI flag or env var |
| Transport | Express HTTP | Stdio (standard MCP) |
| Help content | File reads at runtime | Embedded string constants |
| New tools | — | create_table, insert_row |

---

## Implementation Order

1. Scaffold: package.json, tsconfig.json, bin/vibesql-mcp.js
2. src/client.ts — HTTP client with corrected PostgreSQL queries
3. src/help.ts — Embed 3 help files as constants
4. src/index.ts — MCP server, 9 tools, health check, CLI args
5. Build + test: `npx @vibesql/mcp` against running vibesql-micro
6. Update vibesql-skills README.md to mention the MCP package

---

## Verification

1. Start vibesql-micro: `npx vibesql-micro` (port 5173)
2. Run: `cd vibesql-skills/claude-mcp && node bin/vibesql-mcp.js`
3. Health check passes, MCP server starts on stdio
4. Configure in Claude Code settings, verify tools appear
5. Test: "list all tables", "describe the users table", "insert a row into projects"
6. Test without vibesql-micro running — should print clear error and exit
