#!/usr/bin/env node
// gently-cc MCP server — CCP Watchdog surface
// Transport: stdio JSON-RPC 2.0
// Tools: codec_pin_list, codec_pin_define, codec_pin_conflicts, barf_query

import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { spawnSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = join(__dirname, '..');

function bin(name) {
  const local = join(PKG_ROOT, 'target/release', name);
  return existsSync(local) ? local : name;
}

function callBin(binary, args, stdinData) {
  const r = spawnSync(bin(binary), args, {
    input: stdinData !== undefined ? JSON.stringify(stdinData) : undefined,
    encoding: 'utf8',
    timeout: 5000,
  });
  return { stdout: r.stdout?.trim() || '', stderr: r.stderr?.trim() || '', status: r.status ?? -1 };
}

// ── Tool definitions ──────────────────────────────────────────────────────────

const TOOLS = [
  {
    name: 'codec_pin_list',
    description: 'List all active pins in the current project scope. Returns pin names, labels, and scope levels.',
    inputSchema: {
      type: 'object',
      properties: {
        scope: {
          type: 'string',
          enum: ['chat', 'user', 'project', 'global', 'all'],
          description: 'Which scope level to list. Default: all.',
        },
      },
    },
  },
  {
    name: 'codec_pin_define',
    description: 'Define a new pin in the current project scope. Pins compress repeated concepts across the conversation.',
    inputSchema: {
      type: 'object',
      properties: {
        name: { type: 'string', description: 'Pin name (short identifier, alphanumeric + underscore)' },
        label: { type: 'string', description: 'Full label text this pin expands to' },
      },
      required: ['name', 'label'],
    },
  },
  {
    name: 'codec_pin_conflicts',
    description: 'Show pins defined at multiple scope levels with different labels.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'barf_query',
    description: 'Query the BARF foam for semantically relevant context labels. Returns top-N tori by weight-driven score.',
    inputSchema: {
      type: 'object',
      properties: {
        text: { type: 'string', description: 'Query text to find related context' },
        max: { type: 'number', description: 'Max results to return (default 5)', default: 5 },
      },
      required: ['text'],
    },
  },
];

// ── Tool handlers ─────────────────────────────────────────────────────────────

function handleCodecPinList(args) {
  const scope = args?.scope || 'all';
  const cmdArgs = scope === 'all' ? ['list'] : ['list', '--scope', scope];
  const { stdout, status } = callBin('codec', cmdArgs);
  if (status !== 0) return { error: 'codec list failed — binary may not be built' };
  return { pins: stdout };
}

function handleCodecPinDefine(args) {
  if (!args?.name || !args?.label) return { error: 'name and label are required' };
  const name = args.name.replace(/[^a-zA-Z0-9_]/g, '');
  if (!name) return { error: 'pin name must be alphanumeric/underscore only' };
  const { stdout, status } = callBin('codec', ['define', name, args.label]);
  if (status !== 0) return { error: 'codec define failed', detail: stdout };
  return { success: true, name, label: args.label, output: stdout };
}

function handleCodecPinConflicts() {
  const { stdout, status } = callBin('codec', ['conflicts']);
  if (status !== 0) return { conflicts: [], note: 'codec conflicts not available — rebuild binary' };
  return { conflicts: stdout || '(none)' };
}

function handleBarfQuery(args) {
  if (!args?.text) return { error: 'text is required' };
  const max = Math.min(Math.max(1, args.max ?? 5), 20);
  const { stdout, status } = callBin('barf', ['query', args.text, '--max', String(max)]);
  if (status !== 0) return { error: 'barf query failed — binary may not be built' };
  return { results: stdout };
}

// ── JSON-RPC 2.0 stdio transport ──────────────────────────────────────────────

function respond(id, result) {
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n');
}

function respondError(id, code, message) {
  process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code, message } }) + '\n');
}

function dispatch(msg) {
  const { id, method, params } = msg;

  if (method === 'initialize') {
    return respond(id, {
      protocolVersion: '2024-11-05',
      capabilities: { tools: {} },
      serverInfo: { name: 'gently-cc', version: '1.0.0' },
    });
  }

  if (method === 'tools/list') {
    return respond(id, { tools: TOOLS });
  }

  if (method === 'tools/call') {
    const { name, arguments: args } = params || {};
    let result;
    switch (name) {
      case 'codec_pin_list':     result = handleCodecPinList(args); break;
      case 'codec_pin_define':   result = handleCodecPinDefine(args); break;
      case 'codec_pin_conflicts': result = handleCodecPinConflicts(); break;
      case 'barf_query':         result = handleBarfQuery(args); break;
      default: return respondError(id, -32601, `unknown tool: ${name}`);
    }
    return respond(id, {
      content: [{ type: 'text', text: typeof result === 'string' ? result : JSON.stringify(result, null, 2) }],
    });
  }

  // Ignore notifications (no id) silently
  if (id !== undefined) {
    respondError(id, -32601, `unknown method: ${method}`);
  }
}

// Read newline-delimited JSON from stdin
let buf = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => {
  buf += chunk;
  const lines = buf.split('\n');
  buf = lines.pop();
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      dispatch(JSON.parse(trimmed));
    } catch {
      respondError(null, -32700, 'parse error');
    }
  }
});

process.stdin.on('end', () => process.exit(0));
process.stderr.write('gently-cc MCP server ready (stdio)\n');
