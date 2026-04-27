#!/usr/bin/env node
// gently-cc hook: Stop (pre-seal) — extract pin candidates from model responses
// Scans assistant turns for backtick phrases and noun compounds, seeds foam.
// Fire-and-forget: no stdout required (Stop hook order: this runs first, stop.js seals second).

import { existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { spawnSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = join(__dirname, '..');

function bin(name) {
  const local = join(PKG_ROOT, 'target/release', name);
  return existsSync(local) ? local : name;
}

function extractCandidates(text) {
  const candidates = new Set();

  // Backtick phrases: `some concept`
  for (const m of text.matchAll(/`([^`\n]{4,40})`/g)) {
    candidates.add(m[1].trim());
  }

  // Capitalized noun compounds: "Authentication Flow", "DB Connection Pool"
  for (const m of text.matchAll(/\b([A-Z][a-z]{2,}(?:\s+[A-Z]?[a-z]{2,}){1,3})\b/g)) {
    candidates.add(m[1].trim());
  }

  // snake_case identifiers that look like concepts (len 6+)
  for (const m of text.matchAll(/\b([a-z][a-z0-9]{2,}_[a-z][a-z0-9_]{2,})\b/g)) {
    if (m[1].length >= 6 && m[1].length <= 40) {
      candidates.add(m[1].trim());
    }
  }

  return [...candidates].slice(0, 8); // cap at 8 candidates per response
}

function barfInsert(label) {
  const tokens = Math.max(1, Math.ceil(label.length / 4));
  spawnSync(bin('barf'), ['insert', label, '--tokens', String(tokens)], {
    encoding: 'utf8',
    timeout: 2000,
  });
}

async function main() {
  // Stop hook input: { session_id, cwd, transcript? }
  // We look for assistant_response in env or skip gracefully if unavailable
  const responseText = process.env.CLAUDE_LAST_RESPONSE || '';

  if (!responseText || responseText.length < 20) {
    process.stdout.write(JSON.stringify({ continue: true }));
    return;
  }

  const candidates = extractCandidates(responseText);
  if (candidates.length > 0) {
    for (const label of candidates) {
      barfInsert(label);
    }
    process.stderr.write(`🌱 response-extract: seeded ${candidates.length} candidates\n`);
  }

  process.stdout.write(JSON.stringify({ continue: true }));
}

main().catch(() => {
  process.stdout.write(JSON.stringify({ continue: true }));
});
