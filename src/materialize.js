// gently-cc materialize — generate/verify the full project tree from project.codex.json
//
// On first run: hashes every file, writes CIDs into codex, runs gen on generated_targets.
// On --verify: re-hashes everything, checks CIDs match. Drift = determinism broken.
// On --update: re-runs gen for any generated target whose spec changed.
//
// This is the Nix model applied to CODIE codegen:
//   project.codex.json = the .nix files (source of truth, tiny, shareable)
//   generated output = the /nix/store (derived, reproducible, verifiable)

import { createHash } from 'crypto';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { spawnSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT  = join(__dirname, '..');
const CODEX     = join(PKG_ROOT, 'project.codex.json');
const GEN_BIN   = join(PKG_ROOT, 'target', 'release', 'gen');

function sha256(data) {
  return createHash('sha256').update(data).digest('hex');
}

// CID-lite: sha256:hex (no IPFS overhead for local operation; swap to real CID when IPFS is wired)
function cid(data) {
  return 'sha256:' + sha256(typeof data === 'string' ? data : data.toString('utf8'));
}

function readCodex() {
  if (!existsSync(CODEX)) throw new Error('project.codex.json not found — run: gently-cc install');
  return JSON.parse(readFileSync(CODEX, 'utf8'));
}

function writeCodex(codex) {
  writeFileSync(CODEX, JSON.stringify(codex, null, 2) + '\n', 'utf8');
}

function hashFile(relPath) {
  const abs = join(PKG_ROOT, relPath);
  if (!existsSync(abs)) return null;
  return cid(readFileSync(abs));
}

function runGen(spec) {
  if (!existsSync(GEN_BIN)) {
    console.error('  gen binary not found — run: cargo build --release -p gently-codegen');
    return null;
  }
  const r = spawnSync(GEN_BIN, [spec], { encoding: 'utf8' });
  return r.status === 0 ? r.stdout : null;
}

function runGenTemplate(templateArgs) {
  // templateArgs: e.g. "--template crud User"
  if (!existsSync(GEN_BIN)) return null;
  const args = templateArgs.split(' ').filter(Boolean);
  // strip leading "gen " if present
  const clean = args[0] === 'gen' ? args.slice(1) : args;
  const r = spawnSync(GEN_BIN, clean, { encoding: 'utf8' });
  return r.status === 0 ? r.stdout : null;
}

async function materialize(flags = {}) {
  const codex  = readCodex();
  let   dirty  = false;
  let   errors = 0;

  console.log('\ngently-cc materialize\n');

  // ── 1. Hash static files ──────────────────────────────────────
  if (!flags.verify) console.log('  Hashing static files...');
  for (const [relPath, entry] of Object.entries(codex.files)) {
    if (entry.source !== 'static') continue;
    const current = hashFile(relPath);
    if (!current) {
      if (flags.verify) {
        console.log(`  ✗ MISSING  ${relPath}`);
        errors++;
      } else {
        console.log(`  - MISSING  ${relPath}  (not yet created)`);
      }
      continue;
    }
    if (flags.verify) {
      if (entry.output_cid && entry.output_cid !== current) {
        console.log(`  ✗ DRIFT    ${relPath}`);
        console.log(`    stored : ${entry.output_cid}`);
        console.log(`    current: ${current}`);
        errors++;
      } else {
        console.log(`  ✓ ok       ${relPath}`);
      }
    } else {
      if (entry.output_cid !== current) {
        entry.output_cid = current;
        dirty = true;
      }
      console.log(`  ✓ hashed   ${relPath}`);
    }
  }

  // ── 2. Generate + verify generated targets ────────────────────
  console.log('');
  if (!flags.verify) console.log('  Running generators...');

  for (const [relPath, entry] of Object.entries(codex.generated_targets || {})) {
    if (entry.source !== 'generated') continue;

    // Hash the spec itself
    const specCid = cid(entry.spec);

    // Run gen
    const generator = entry.generator || `gen '${entry.spec}'`;
    const isTemplate = generator.includes('--template');
    const args = isTemplate
      ? generator.replace(/^gen\s*/, '')
      : entry.spec;

    const output = isTemplate ? runGenTemplate(args) : runGen(args);
    if (!output) {
      console.log(`  ✗ GENFAIL  ${relPath}`);
      errors++;
      continue;
    }

    const outputCid = cid(output);

    if (flags.verify) {
      if (entry.output_cid && entry.output_cid !== outputCid) {
        console.log(`  ✗ NON-DET  ${relPath}  (output CID changed — emitter is not deterministic)`);
        errors++;
      } else {
        console.log(`  ✓ det ok   ${relPath}`);
      }
    } else {
      // Write the output file
      const abs = join(PKG_ROOT, relPath);
      mkdirSync(dirname(abs), { recursive: true });
      writeFileSync(abs, output, 'utf8');

      entry.spec_cid   = specCid;
      entry.output_cid = outputCid;
      dirty = true;
      console.log(`  ✓ gen      ${relPath}  [${outputCid.slice(0,16)}...]`);
    }
  }

  // ── 3. Hash skills ─────────────────────────────────────────────
  console.log('');
  for (const [id, skill] of Object.entries(codex.skills || {})) {
    const current = hashFile(skill.file);
    if (!current) continue;
    if (!flags.verify) {
      if (skill.hash !== current) { skill.hash = current; dirty = true; }
    } else {
      if (skill.hash && skill.hash !== current) {
        console.log(`  ✗ DRIFT    skills/${id}  ${skill.file}`);
        errors++;
      }
    }
  }

  // ── 4. Compute root CID from all known CIDs ────────────────────
  const allCids = [
    ...Object.values(codex.files).map(e => e.output_cid).filter(Boolean),
    ...Object.values(codex.generated_targets || {}).map(e => e.output_cid).filter(Boolean),
    ...Object.values(codex.skills || {}).map(e => e.hash).filter(Boolean),
  ].sort().join('\n');

  const rootCid = allCids ? cid(allCids) : null;

  if (flags.verify) {
    if (codex.root_cid && codex.root_cid !== rootCid) {
      console.log(`\n  ✗ ROOT CID MISMATCH`);
      console.log(`    stored : ${codex.root_cid}`);
      console.log(`    current: ${rootCid}`);
      errors++;
    } else {
      console.log(`\n  ✓ root     ${rootCid}`);
    }
    console.log(errors === 0 ? '\n  All checks passed.\n' : `\n  ${errors} check(s) failed.\n`);
    if (errors > 0) process.exit(1);
  } else {
    codex.root_cid = rootCid;
    if (dirty) {
      writeCodex(codex);
      console.log(`\n  root_cid: ${rootCid}`);
      console.log('  Codex updated: project.codex.json\n');
    } else {
      console.log('\n  Codex up to date.\n');
    }
  }
}

// ── CLI ────────────────────────────────────────────────────────
const args = process.argv.slice(2);
const flags = {
  verify: args.includes('--verify'),
  update: args.includes('--update'),
};

materialize(flags).catch(e => { console.error(e.message); process.exit(1); });
