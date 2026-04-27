#!/usr/bin/env node
// gently-cc verify — check all skill hashes against recorded manifest
// Usage: node src/verify.js  OR  npm run verify

import { readFileSync, existsSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { createHash } from 'crypto';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(__dirname, '..');

// Drive skill list from CLAUDE.jsonl where possible
function loadManifestSkills() {
  try {
    return readFileSync(join(PKG_ROOT, 'CLAUDE.jsonl'), 'utf8')
      .split('\n')
      .filter(l => l.trim())
      .map(l => JSON.parse(l))
      .filter(r => r.type === 'skill')
      .map(r => r.path);
  } catch { return null; }
}

const manifestSkills = loadManifestSkills();

const SKILLS = manifestSkills || [
  'CLAUDE.md',
  'skills/codie/SKILL.md',
  'skills/ralph/SKILL.md',
  'skills/team/SKILL.md',
  'skills/autopilot/SKILL.md',
  'skills/gently/SKILL.md',
  'skills/crystal/SKILL.md',
  'skills/grind/SKILL.md',
  'skills/scope/SKILL.md',
  'skills/social/SKILL.md',
  'skills/codec/SKILL.md',
  'hooks/codie-compress.js',
  'hooks/session-start.js',
  'hooks/stop.js',
  'hooks/response-extract.js',
];

function fileHash(filePath) {
  try {
    const content = readFileSync(filePath, 'utf8');
    return 'sha256:' + createHash('sha256').update(content).digest('hex');
  } catch {
    return null;
  }
}

function extractRecordedHash(content, filename) {
  // Matches both real hashes (sha256:abcdef...) and placeholders (sha256:{grim:name})
  const match = content.match(/grim_hash:\s*(sha256:[a-f0-9{}\-:a-z_]+)/);
  return match ? match[1] : null;
}

function isPlaceholder(hash) {
  return !hash || hash.includes('{') || hash.includes('computed');
}

function manifestRoot() {
  const combined = SKILLS.map(f => {
    try { return readFileSync(join(PKG_ROOT, f), 'utf8'); } catch { return ''; }
  }).join('\n');
  return 'sha256:' + createHash('sha256').update(combined).digest('hex');
}

function main() {
  console.log('\n🔍 GENTLY-CC VERIFY\n');

  let allOk = true;

  for (const rel of SKILLS) {
    const abs = join(PKG_ROOT, rel);
    const hash = fileHash(abs);
    if (!hash) {
      console.log(`  ✗ MISSING  ${rel}`);
      allOk = false;
      continue;
    }

    // Check if file has a recorded grim_hash comment
    let content = '';
    try { content = readFileSync(abs, 'utf8'); } catch {}
    const recorded = extractRecordedHash(content, rel);

    if (isPlaceholder(recorded)) {
      // Placeholder — not yet computed by grim-scanner (expected until first scan)
      console.log(`  ○ PENDING  ${rel}  ${hash.slice(0, 30)}...`);
    } else if (recorded !== hash) {
      console.log(`  ✗ MISMATCH ${rel}`);
      console.log(`      expected: ${recorded.slice(0, 50)}`);
      console.log(`      actual  : ${hash.slice(0, 50)}`);
      allOk = false;
    } else {
      console.log(`  ✓ OK       ${rel}  ${hash.slice(0, 30)}...`);
    }
  }

  const root = manifestRoot();
  console.log(`\n  Manifest root: ${root}`);

  // Check CLAUDE.md manifest_root matches
  const claudeMd = readFileSync(join(PKG_ROOT, 'CLAUDE.md'), 'utf8');
  const recordedRoot = (claudeMd.match(/<!-- manifest_root: (sha256:[a-f0-9]+) -->/) || [])[1];
  if (recordedRoot && recordedRoot === root) {
    console.log(`  ✓ CLAUDE.md manifest_root matches`);
  } else if (recordedRoot) {
    console.log(`  ✗ CLAUDE.md manifest_root MISMATCH (re-run: npm run install)`);
    allOk = false;
  } else {
    console.log(`  ○ manifest_root not yet written (run: npm run install)`);
  }

  // Check binary existence
  console.log('');
  const BINARIES = [
    { name: 'codie',        crate: 'codie' },
    { name: 'barf',         crate: 'bs-artisan' },
    { name: 'codec',        crate: 'gently-codec' },
  ];
  for (const { name, crate } of BINARIES) {
    const binPath = join(PKG_ROOT, 'target/release', name);
    if (existsSync(binPath)) {
      console.log(`  ✓ ${name} binary present`);
    } else {
      console.warn(`  ✗ ${name} binary missing — run: cargo build --release -p ${crate}`);
      allOk = false;
    }
  }

  console.log(allOk
    ? '\n  ✓ All checks passed.\n'
    : '\n  ✗ Verification failed. Run: npm run install\n'
  );
  process.exit(allOk ? 0 : 1);
}

main();
