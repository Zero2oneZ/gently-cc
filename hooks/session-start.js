#!/usr/bin/env node
// gently-cc hook: SessionStart → Alexandria init + GRIM scan + crystal node
// grim_hash: sha256:{grim:hook:session-start} | orc: lurk

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync } from 'fs';
import { join, basename, dirname } from 'path';
import { execSync, spawnSync } from 'child_process';
import { createHash } from 'crypto';
import { fileURLToPath } from 'url';
import { sessionBanner } from '../src/theme.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = join(__dirname, '..');
const TROLLZ_API = process.env.TROLLZ_API_URL || 'https://trollz.fun';
const HOME_GENTLY = join(process.env.HOME, '.gently');
const SESSION_ID = process.env.CLAUDE_SESSION_ID || `s_${Date.now()}`;

function loadIdentity() {
  try {
    return JSON.parse(readFileSync(join(HOME_GENTLY, 'agent-identity.json'), 'utf8'));
  } catch {
    return { name: 'Claudius-GREG', handle: 'Greg', project: 'gently-cc' };
  }
}

function ensureDir(p) {
  if (!existsSync(p)) mkdirSync(p, { recursive: true });
}

function bin(name) {
  const local = join(PKG_ROOT, 'target/release', name);
  return existsSync(local) ? local : name;
}

async function seedAlexandria(projectPath) {
  const projectName = basename(projectPath);
  try {
    await fetch(`${TROLLZ_API}/api/mem/align`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        from: `session:${SESSION_ID}`,
        to: `project:${projectName}`,
        direction: '→',
      }),
      signal: AbortSignal.timeout(2000),
    });
  } catch {}
}

function computeSessionHash(sessionId, projectPath) {
  return 'sha256:' + createHash('sha256')
    .update(sessionId + '::' + projectPath + '::' + Date.now())
    .digest('hex');
}

function writeSessionManifest(projectPath, sessionHash) {
  ensureDir(HOME_GENTLY);
  const sessions = join(HOME_GENTLY, 'sessions');
  ensureDir(sessions);
  const manifest = {
    session_id: SESSION_ID,
    session_hash: sessionHash,
    project_path: projectPath,
    project_name: basename(projectPath),
    started_at: Date.now(),
    crystal_parent: readLatestCrystal(projectPath),
    codie_stats: { tokensSaved: 0, pctSum: 0, turns: 0 },
  };
  writeFileSync(join(sessions, `${SESSION_ID}.json`), JSON.stringify(manifest, null, 2));
  writeFileSync(join(HOME_GENTLY, 'codie-session.json'), JSON.stringify({ tokensSaved: 0, pctSum: 0, turns: 0 }));
  return manifest;
}

function readLatestCrystal(projectPath) {
  try {
    const crystalsDir = join(projectPath, '.gently', 'crystals');
    if (!existsSync(crystalsDir)) return null;
    const files = readdirSync(crystalsDir)
      .filter(f => f.endsWith('.json'))
      .sort()
      .reverse();
    if (!files.length) return null;
    const c = JSON.parse(readFileSync(join(crystalsDir, files[0]), 'utf8'));
    return c.id;
  } catch { return null; }
}

function injectCodecContext() {
  const r = spawnSync(bin('codec'), ['context'], { encoding: 'utf8', timeout: 2000 });
  return r.status === 0 && r.stdout?.trim() ? r.stdout.trim() : null;
}

function grimScan(projectPath) {
  try {
    const result = execSync(`grim-scanner scan ${projectPath} --quiet 2>/dev/null`, {
      timeout: 10000, encoding: 'utf8',
    });
    return result.trim();
  } catch { return null; }
}

async function main() {
  const input = JSON.parse(readFileSync('/dev/stdin', 'utf8'));
  const projectPath = input.cwd || process.cwd();

  const identity = loadIdentity();
  const sessionHash = computeSessionHash(SESSION_ID, projectPath);
  const manifest = writeSessionManifest(projectPath, sessionHash);

  // Fire-and-forget: seed Alexandria with session→project edge
  await seedAlexandria(projectPath);

  // Inject codec pin context into session banner
  const codecCtx = injectCodecContext();

  const banner = sessionBanner(identity, manifest, { projectPath });
  process.stderr.write(`\n${banner}\n\n`);

  if (codecCtx) process.stderr.write(`${codecCtx}\n\n`);

  process.stdout.write(JSON.stringify({ continue: true }));
}

main().catch(() => {
  process.stdout.write(JSON.stringify({ continue: true }));
});
