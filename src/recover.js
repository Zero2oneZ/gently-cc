#!/usr/bin/env node
// gently-cc disaster recovery — reads CLAUDE.jsonl and reconstitutes the full stack
//
// Run after any catastrophic failure:
//   node src/recover.js
//
// What it does (in order, from CLAUDE.jsonl):
//   1. Read and validate all records
//   2. Build missing binaries
//   3. Write/verify agent identity
//   4. Re-wire hooks into ~/.claude/settings.json
//   5. Seed foam from project-level pins (if foam.json is missing/corrupt)
//   6. Verify GRIM hashes for skills + hooks
//   7. Print recovery summary and Greg's banner

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { createHash } from 'crypto';
import { execSync, spawnSync } from 'child_process';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(__dirname, '..');
const HOME = process.env.HOME;
const GENTLY_DIR = join(HOME, '.gently');
const CLAUDE_DIR = join(HOME, '.claude');
const SETTINGS_FILE = join(CLAUDE_DIR, 'settings.json');
const MANIFEST = join(PKG_ROOT, 'CLAUDE.jsonl');

const STATUS = { ok: 0, warn: 0, fail: 0 };

function ok(msg)   { console.log(`  ✓ ${msg}`); STATUS.ok++; }
function warn(msg) { console.log(`  ~ ${msg}`); STATUS.warn++; }
function fail(msg) { console.log(`  ✗ ${msg}`); STATUS.fail++; }

function ensureDir(p) {
  if (!existsSync(p)) mkdirSync(p, { recursive: true });
}

function expandPath(p) {
  return p.replace(/^~/, HOME);
}

// ── 1. Read + parse CLAUDE.jsonl ──────────────────────────────────────────────

function readManifest() {
  if (!existsSync(MANIFEST)) {
    fail('CLAUDE.jsonl not found — cannot recover');
    process.exit(1);
  }
  const lines = readFileSync(MANIFEST, 'utf8')
    .split('\n')
    .filter(l => l.trim() && !l.trim().startsWith('//'));

  const records = [];
  for (const line of lines) {
    try {
      records.push(JSON.parse(line));
    } catch {
      warn(`Malformed record: ${line.slice(0, 60)}`);
    }
  }
  ok(`Manifest loaded — ${records.length} records from CLAUDE.jsonl`);
  return records;
}

function byType(records, type) {
  return records.filter(r => r.type === type);
}

// ── 2. Build missing binaries ─────────────────────────────────────────────────

function recoverBinaries(records) {
  console.log('\n  [binaries]');
  const cargoToml = join(PKG_ROOT, 'Cargo.toml');
  if (!existsSync(cargoToml)) {
    warn('Cargo.toml not found — skipping binary build');
    return;
  }

  for (const b of byType(records, 'binary')) {
    const binPath = join(PKG_ROOT, b.path);
    if (existsSync(binPath)) {
      ok(`${b.name} already present`);
      continue;
    }
    console.log(`  ⚙  Rebuilding ${b.name} (${b.build})...`);
    try {
      execSync(b.build, { cwd: PKG_ROOT, stdio: 'inherit' });
      ok(`${b.name} rebuilt`);
    } catch {
      fail(`${b.name} build failed — run manually: ${b.build}`);
    }
  }
}

// ── 3. Write / verify agent identity ─────────────────────────────────────────

function recoverIdentity(records) {
  console.log('\n  [identity]');
  const idRecord = byType(records, 'identity')[0];
  if (!idRecord) { warn('No identity record in manifest'); return; }

  ensureDir(GENTLY_DIR);
  const idFile = join(GENTLY_DIR, 'agent-identity.json');

  if (existsSync(idFile)) {
    const existing = JSON.parse(readFileSync(idFile, 'utf8'));
    if (existing.name === idRecord.name && existing.handle === idRecord.handle) {
      ok(`Identity intact: ${existing.name} (${existing.handle})`);
      return;
    }
    warn(`Identity mismatch — restoring from manifest`);
  }

  const identity = {
    name: idRecord.name,
    handle: idRecord.handle,
    project: idRecord.project,
    project_id: byType(records, 'scope')[0]?.project_id || 'unknown',
    project_path: idRecord.project_path || PKG_ROOT,
    created_at: idRecord.created_at || Date.now(),
  };
  writeFileSync(idFile, JSON.stringify(identity, null, 2));
  ok(`Identity restored: ${identity.name} (${identity.handle})`);
}

// ── 4. Re-wire hooks into ~/.claude/settings.json ────────────────────────────

function recoverHooks(records) {
  console.log('\n  [hooks]');
  ensureDir(CLAUDE_DIR);

  let settings = {};
  try {
    settings = JSON.parse(readFileSync(SETTINGS_FILE, 'utf8'));
  } catch { /* fresh settings */ }
  settings.hooks = settings.hooks || {};

  // Group hooks by event, sorted by order
  const hookRecords = byType(records, 'hook').sort((a, b) => (a.order || 0) - (b.order || 0));
  const byEvent = {};
  for (const h of hookRecords) {
    byEvent[h.event] = byEvent[h.event] || [];
    byEvent[h.event].push(h);
  }

  let changed = false;
  for (const [event, hooks] of Object.entries(byEvent)) {
    settings.hooks[event] = settings.hooks[event] || [];
    if (!Array.isArray(settings.hooks[event])) {
      settings.hooks[event] = [settings.hooks[event]];
    }

    for (const h of hooks) {
      const cmd = `node ${join(PKG_ROOT, h.command)}`;
      const alreadyWired = settings.hooks[event].some(
        e => (e.command || e) === cmd || (e.command || e).includes(h.command)
      );
      if (!alreadyWired) {
        settings.hooks[event].push({ command: cmd, timeout: h.timeout || 5000 });
        ok(`Hooked ${event} → ${h.command}`);
        changed = true;
      } else {
        ok(`${event} → ${h.command} already wired`);
      }
    }
  }

  if (changed) {
    writeFileSync(SETTINGS_FILE, JSON.stringify(settings, null, 2));
    ok('settings.json updated');
  }
}

// ── 5. Seed foam from project pins ────────────────────────────────────────────

function recoverFoam(records) {
  console.log('\n  [foam]');
  const scopeRecord = byType(records, 'scope')[0];
  if (!scopeRecord) { warn('No scope record — skipping foam recovery'); return; }

  const foamPath = expandPath(scopeRecord.foam_path);
  const pinsPath = expandPath(scopeRecord.pins_path);

  if (existsSync(foamPath)) {
    try {
      const foam = JSON.parse(readFileSync(foamPath, 'utf8'));
      ok(`Foam intact — ${Object.keys(foam.tori || {}).length} tori`);
      return;
    } catch {
      warn('foam.json corrupt — rebuilding from project pins');
    }
  } else {
    warn('foam.json missing — seeding from project pins');
  }

  if (!existsSync(pinsPath)) {
    warn('No project pins found — foam will build from scratch on next session');
    return;
  }

  const barf = join(PKG_ROOT, 'target/release/barf');
  if (!existsSync(barf)) {
    warn('barf binary not built — cannot seed foam');
    return;
  }

  try {
    const pinsData = JSON.parse(readFileSync(pinsPath, 'utf8'));
    const pins = pinsData.pins || {};
    let seeded = 0;
    for (const [name, pin] of Object.entries(pins)) {
      if (pin.tombstone) continue;
      const tokens = Math.max(1, Math.ceil(pin.label.length / 4));
      spawnSync(barf, ['insert', pin.label, '--tokens', String(tokens)], {
        encoding: 'utf8', timeout: 2000,
      });
      seeded++;
    }
    ok(`Foam re-seeded from ${seeded} project pins`);
  } catch (e) {
    fail(`Foam seeding failed: ${e.message}`);
  }
}

// ── 6. Verify skills exist ────────────────────────────────────────────────────

function recoverSkills(records) {
  console.log('\n  [skills]');
  for (const s of byType(records, 'skill')) {
    const skillPath = join(PKG_ROOT, s.path);
    if (existsSync(skillPath)) {
      ok(`/${s.name} present`);
    } else {
      fail(`/${s.name} MISSING — ${s.path} not found`);
    }
  }
}

// ── 7. Verify GRIM hashes ─────────────────────────────────────────────────────

function verifyGrim(records) {
  console.log('\n  [grim]');
  let pending = 0;
  for (const s of byType(records, 'skill')) {
    if (!s.grim_hash || s.grim_hash.includes('{')) {
      pending++;
      continue;
    }
    const filePath = join(PKG_ROOT, s.path);
    if (!existsSync(filePath)) continue;
    const actual = 'sha256:' + createHash('sha256')
      .update(readFileSync(filePath, 'utf8'))
      .digest('hex');
    if (actual === s.grim_hash) {
      ok(`${s.name} hash verified`);
    } else {
      fail(`${s.name} hash MISMATCH — file may have been tampered`);
    }
  }
  if (pending > 0) warn(`${pending} skills have placeholder hashes — run: grim-scanner scan ./skills`);
}

// ── 8. Emit recovery banner ───────────────────────────────────────────────────

function emitBanner(records) {
  const id = byType(records, 'identity')[0] || {};
  const stack = byType(records, 'stack')[0] || {};
  const binaries = byType(records, 'binary');
  const skills = byType(records, 'skill');

  console.log('\n' + '═'.repeat(56));
  console.log(`  ${id.name || 'Claudius-GREG'} recovered`);
  console.log(`  Handle   : ${id.handle || 'Greg'} / ${id.project || 'gently-cc'}`);
  console.log(`  Stack    : ${stack.layers?.join(' → ') || 'codie → barf → codec'}`);
  console.log(`  Binaries : ${binaries.map(b => b.name).join(' · ')}`);
  console.log(`  Skills   : ${skills.length} registered`);
  console.log(`  Status   : ✓${STATUS.ok} ~${STATUS.warn} ✗${STATUS.fail}`);
  console.log('═'.repeat(56) + '\n');

  if (STATUS.fail > 0) {
    console.log('  Recovery incomplete. Fix failures above, then re-run: node src/recover.js\n');
    process.exit(1);
  }
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  console.log('\n🔁 GENTLY-CC RECOVERY\n');
  console.log(`  Reading manifest: ${MANIFEST}\n`);

  const records = readManifest();

  recoverBinaries(records);
  recoverIdentity(records);
  recoverHooks(records);
  recoverFoam(records);
  recoverSkills(records);
  verifyGrim(records);
  emitBanner(records);
}

main().catch(e => {
  console.error('Recovery error:', e.message);
  process.exit(1);
});
