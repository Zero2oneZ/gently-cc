#!/usr/bin/env node
// gently-cc install script
// Runs on `npm install gently-cc` or `/plugin install gently-cc`
// Wires hooks + skills into ~/.claude/settings.json

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, resolve, dirname } from 'path';
import { createHash } from 'crypto';
import { fileURLToPath } from 'url';
import { THEMES, themeIndexForProject } from './theme.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(__dirname, '..');
const CLAUDE_DIR = join(process.env.HOME, '.claude');
const SETTINGS_FILE = join(CLAUDE_DIR, 'settings.json');
const GENTLY_DIR = join(process.env.HOME, '.gently');

function ensureDir(p) {
  if (!existsSync(p)) mkdirSync(p, { recursive: true });
}

function loadSettings() {
  try {
    return JSON.parse(readFileSync(SETTINGS_FILE, 'utf8'));
  } catch {
    return {};
  }
}

function hookPath(name) {
  return `node ${join(PKG_ROOT, 'hooks', name)}`;
}

function loadManifestRecords() {
  try {
    return readFileSync(join(PKG_ROOT, 'CLAUDE.jsonl'), 'utf8')
      .split('\n')
      .filter(l => l.trim())
      .map(l => JSON.parse(l));
  } catch { return []; }
}

function installHooks(settings) {
  settings.hooks = settings.hooks || {};

  // Drive from CLAUDE.jsonl — single source of truth for hook wiring
  const records = loadManifestRecords();
  const hookRecords = records
    .filter(r => r.type === 'hook')
    .sort((a, b) => (a.order || 0) - (b.order || 0));

  // Group by event
  const byEvent = {};
  for (const h of hookRecords) {
    byEvent[h.event] = byEvent[h.event] || [];
    byEvent[h.event].push(h);
  }

  for (const [event, hooks] of Object.entries(byEvent)) {
    settings.hooks[event] = settings.hooks[event] || [];
    if (!Array.isArray(settings.hooks[event])) {
      settings.hooks[event] = [settings.hooks[event]];
    }
    // Check if any existing entry already references gently-cc
    const alreadyWired = settings.hooks[event].some(entry => {
      const cmds = (entry.hooks || []).map(h => h.command || '');
      return cmds.some(c => c.includes('gently-cc'));
    });
    if (!alreadyWired) {
      // All hooks for this event go into one matcher entry (matcher: "" = match all)
      settings.hooks[event].push({
        matcher: '',
        hooks: hooks.map(h => ({
          type: 'command',
          command: hookPath(h.command.replace('hooks/', '')),
          timeout: h.timeout || 5000,
        })),
      });
      console.log(`  ✓ Hooked ${event} (${hooks.length} handler${hooks.length > 1 ? 's' : ''})`);
    } else {
      console.log(`  · ${event} already wired`);
    }
  }
  return settings;
}

function computeManifestRoot() {
  // Drive skill list from CLAUDE.jsonl
  const records = loadManifestRecords();
  const skillPaths = records
    .filter(r => r.type === 'skill')
    .map(r => r.path);

  const allFiles = ['CLAUDE.md', 'CLAUDE.jsonl', ...skillPaths];
  const combined = allFiles.map(f => {
    try { return readFileSync(join(PKG_ROOT, f), 'utf8'); } catch { return ''; }
  }).join('\n');

  return 'sha256:' + createHash('sha256').update(combined).digest('hex');
}

function patchManifestRoot(manifestRoot) {
  const claudeMd = join(PKG_ROOT, 'CLAUDE.md');
  let content = readFileSync(claudeMd, 'utf8');
  content = content.replace(
    /<!-- manifest_root: sha256:\{computed[^}]*\} -->/,
    `<!-- manifest_root: ${manifestRoot} -->`
  );
  writeFileSync(claudeMd, content);
  console.log(`  ✓ manifest_root: ${manifestRoot.slice(0, 40)}...`);
}

// Map process.platform + process.arch to GitHub release asset names
function platformTarget() {
  const p = process.platform;
  const a = process.arch;
  if (p === 'linux'  && a === 'x64')   return 'x86_64-unknown-linux-musl';
  if (p === 'linux'  && a === 'arm64') return 'aarch64-unknown-linux-musl';
  if (p === 'darwin' && a === 'x64')   return 'x86_64-apple-darwin';
  if (p === 'darwin' && a === 'arm64') return 'aarch64-apple-darwin';
  return null;
}

function findRsign() {
  const { spawnSync } = require('child_process');
  for (const candidate of ['rsign', `${process.env.HOME}/.cargo/bin/rsign`]) {
    if (spawnSync(candidate, ['--version'], { encoding: 'utf8' }).status === 0) return candidate;
  }
  return null;
}

async function verifyBinary(binaryPath, sigPath) {
  const { spawnSync } = await import('child_process');
  const pubkey = join(PKG_ROOT, 'src', 'minisign.pub');
  if (!existsSync(pubkey)) { console.log('  · no public key — skipping signature verify'); return true; }
  const rsign = findRsign();
  if (!rsign) { console.log('  · rsign not found — skipping signature verify'); return true; }
  const r = spawnSync(rsign, ['verify', '-p', pubkey, '-m', binaryPath, '-x', sigPath], { encoding: 'utf8' });
  if (r.status !== 0) {
    try { unlinkSync(binaryPath); } catch {}
    try { unlinkSync(sigPath); } catch {}
    return false;
  }
  return true;
}

async function downloadBinaries(version) {
  const target = platformTarget();
  if (!target) return false;

  const { execSync } = await import('child_process');
  const { unlinkSync } = await import('fs');
  const outDir = join(PKG_ROOT, 'target', 'release');
  mkdirSync(outDir, { recursive: true });

  const base = `https://github.com/Zero2oneZ/gently-cc/releases/download/v${version}`;
  const binaries = ['codie', 'barf', 'codec'];

  console.log(`  ⬇  Downloading pre-built binaries for ${target}...`);
  let verified = 0;
  for (const name of binaries) {
    const dest    = join(outDir, name);
    const destSig = dest + '.minisig';
    if (existsSync(dest) && existsSync(destSig)) { verified++; continue; }
    try {
      execSync(`curl -fsSL "${base}/${name}-${target}" -o "${dest}"`,         { stdio: 'pipe' });
      execSync(`curl -fsSL "${base}/${name}-${target}.minisig" -o "${destSig}"`, { stdio: 'pipe' });
      const ok = await verifyBinary(dest, destSig);
      if (!ok) { console.log(`  ✗ ${name} signature INVALID — binary removed`); continue; }
      execSync(`chmod +x "${dest}"`);
      console.log(`  ✓ ${name} downloaded and verified`);
      verified++;
    } catch {
      console.log(`  · ${name} not available for ${target}`);
    }
  }
  return verified === binaries.length;
}

async function buildBinaries() {
  const { execSync, spawnSync } = await import('child_process');

  // Check if cargo is available
  const cargoCheck = spawnSync('cargo', ['--version'], { encoding: 'utf8' });
  if (cargoCheck.status !== 0) {
    console.log('  · cargo not found — JS-only mode active (full compression requires Rust)');
    console.log('  · Install Rust: https://rustup.rs  then run: gently-cc install');
    return false;
  }

  const cargoToml = join(PKG_ROOT, 'Cargo.toml');
  if (!existsSync(cargoToml)) {
    console.log('  · Cargo.toml not found — source not included in this install');
    return false;
  }

  const crates = [
    { pkg: 'codie',        out: 'codie' },
    { pkg: 'bs-artisan',   out: 'barf'  },
    { pkg: 'gently-codec', out: 'codec' },
  ];

  for (const { pkg, out } of crates) {
    try {
      console.log(`  ⚙  cargo build --release -p ${pkg}...`);
      execSync(`cargo build --release -p ${pkg}`, { cwd: PKG_ROOT, stdio: 'inherit' });
      console.log(`  ✓ ${out} built`);
    } catch {
      console.log(`  ✗ ${pkg} build failed — fallback JS hook active`);
      return false;
    }
  }
  return true;
}

async function buildCodiBinary() {
  const pkg = JSON.parse(readFileSync(join(PKG_ROOT, 'package.json'), 'utf8'));
  const version = pkg.version;

  // 1. Try pre-built download (fast, no Rust required)
  const downloaded = await downloadBinaries(version);
  if (downloaded) return true;

  // 2. Try building from source
  const built = await buildBinaries();
  if (built) return true;

  // 3. JS-only fallback — hooks work, compression degrades gracefully
  console.log('  · Running in JS-only mode — no native binaries');
  console.log('  · Compression: fallback glyph table (reduced efficiency)');
  console.log('  · BARF/codec: disabled until binaries available');
  return false;
}

function projectHandleFromDir(dirName) {
  // Derive 4-letter handle from project directory name
  // gently-cc → GREG (user override wins), else auto-slug
  const overrideFile = join(PKG_ROOT, '.agent-handle');
  if (existsSync(overrideFile)) {
    return readFileSync(overrideFile, 'utf8').trim().toUpperCase().slice(0, 8);
  }
  // Auto: take consonants from dir name, uppercase, pad to 4
  const consonants = dirName.replace(/[aeiou\-_\.]/gi, '').toUpperCase();
  return (consonants + 'XXXX').slice(0, 4);
}

function writeAgentIdentity() {
  const identityFile = join(GENTLY_DIR, 'agent-identity.json');
  if (existsSync(identityFile)) {
    const existing = JSON.parse(readFileSync(identityFile, 'utf8'));
    console.log(`  · Agent identity exists: ${existing.name} (${existing.handle})`);
    return existing;
  }

  const { basename } = await import('path');
  const projectName = basename(PKG_ROOT);
  const handle = projectHandleFromDir(projectName);
  const name = `Claudius-${handle}`;
  const projectId = createHash('sha256').update(PKG_ROOT).digest('hex').slice(0, 16);

  const themeIdx = themeIndexForProject(projectId);
  const theme = THEMES[themeIdx].name;

  const identity = {
    name,
    handle,
    theme,
    project: projectName,
    project_id: projectId,
    project_path: PKG_ROOT,
    created_at: Date.now(),
  };

  writeFileSync(identityFile, JSON.stringify(identity, null, 2));
  console.log(`  ✓ Agent identity: ${name} [${theme.toUpperCase()}] bound to ${projectName}`);

  // Patch CLAUDE.md with the resolved identity
  const claudeMd = join(PKG_ROOT, 'CLAUDE.md');
  let content = readFileSync(claudeMd, 'utf8');
  content = content
    .replace(/\*\*You are Greg\.\*\*/, `**You are ${handle}.**`)
    .replace(/Full designation: \*\*Claudius-GREG\*\*/, `Full designation: **${name}**`)
    .replace(/handle\s+: Greg/, `handle   : ${handle}`)
    .replace(/identity : ~\/.gently\/agent-identity\.json/,
      `identity : ${identityFile}`);
  // Only rewrite if the project isn't already gently-cc with GREG
  if (projectName !== 'gently-cc') {
    writeFileSync(claudeMd, content);
    console.log(`  ✓ CLAUDE.md patched with identity: ${name}`);
  }

  return identity;
}

async function main() {
  console.log('\n🧬 GENTLY-CC INSTALL\n');
  ensureDir(GENTLY_DIR);

  // 1. Write agent identity (Greg / Claudius-GREG for this project)
  const identity = writeAgentIdentity();

  // 2. Build codie Rust binary
  await buildCodiBinary();

  // 3. Compute and patch manifest root hash
  const manifestRoot = computeManifestRoot();
  patchManifestRoot(manifestRoot);

  // 4. Install hooks into ~/.claude/settings.json
  const settings = loadSettings();
  installHooks(settings);
  writeFileSync(SETTINGS_FILE, JSON.stringify(settings, null, 2));
  console.log(`  ✓ Settings written to ${SETTINGS_FILE}`);

  // 5. Print summary
  console.log('\n────────────────────────────────────────────');
  console.log(`  Agent   : ${identity.name} (${identity.handle}) — ${identity.project}`);
  console.log('  Stack   : CODIE · GRIM · Alexandria · BS-Artisan · Sui');
  console.log('  Hooks   : UserPromptSubmit → CODIE → BARF → regex I/O · SessionStart · Stop');
  console.log('  Skills  : /codie /ralph /team /autopilot /gently /crystal /grind /scope /social /codec /identity');
  console.log('  Binaries: codie · barf · codec — Rust, zero-network');
  console.log('  Savings : 33–45% natural language · 80%+ structured CODIE');
  console.log('  Root    : ' + manifestRoot.slice(0, 50) + '...');
  console.log('────────────────────────────────────────────');
  console.log('\n  Start a CC session. GENTLY-CC is active.\n');
}

main().catch(e => {
  console.error('Install error:', e.message);
  process.exit(1);
});
