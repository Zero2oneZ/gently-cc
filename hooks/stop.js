#!/usr/bin/env node
// gently-cc hook: Stop → SYNTH calc + crystal checkpoint + session summary
// grim_hash: sha256:{grim:hook:stop} | orc: forge

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, basename, dirname } from 'path';
import { createHash } from 'crypto';
import { spawnSync } from 'child_process';

const TROLLZ_API = process.env.TROLLZ_API_URL || 'https://trollz.fun';
const HOME_GENTLY = join(process.env.HOME, '.gently');
const SESSION_ID = process.env.CLAUDE_SESSION_ID || 'unknown';
const SYNTH_USD_RATE = parseFloat(process.env.SYNTH_USD_RATE || '1000');
const RARITY_MULTIPLIER = parseFloat(process.env.AGENT_RARITY_MULTIPLIER || '1.0');

function loadCodieStats() {
  try {
    return JSON.parse(readFileSync(join(HOME_GENTLY, 'codie-session.json'), 'utf8'));
  } catch {
    return { tokensSaved: 0, pctSum: 0, turns: 0 };
  }
}

function qualityScore(stats) {
  // Simplified: based on turns completed and compression ratio
  const completionScore = Math.min(stats.turns / 10, 1) * 0.4;
  const compressionScore = ((stats.pctSum / Math.max(stats.turns, 1)) / 100) * 0.3;
  return Math.min(completionScore + compressionScore + 0.3, 1.0);
}

function synthCalc(stats) {
  if (stats.turns === 0) return 0;
  const apiCostUsd = (stats.tokensSaved * 0.000003); // rough est
  const qs = qualityScore(stats);
  const interactionBonus = Math.log10(stats.turns + 1);
  return apiCostUsd * RARITY_MULTIPLIER * interactionBonus * qs * SYNTH_USD_RATE;
}

function crystalCheckpoint(projectPath, stats) {
  const stateStr = JSON.stringify({ session: SESSION_ID, stats, ts: Date.now() });
  const hash = 'sha256:' + createHash('sha256').update(stateStr).digest('hex');

  const crystalsDir = join(projectPath, '.gently', 'crystals');
  if (!existsSync(crystalsDir)) mkdirSync(crystalsDir, { recursive: true });

  const crystal = {
    id: hash,
    session_id: SESSION_ID,
    phase: 'checkpoint',
    synth_earned: synthCalc(stats),
    quality_score: qualityScore(stats),
    codie_stats: stats,
    timestamp: Date.now(),
  };
  writeFileSync(join(crystalsDir, `${Date.now()}-checkpoint.json`), JSON.stringify(crystal, null, 2));
  return crystal;
}

async function flushToAlexandria(stats) {
  try {
    await fetch(`${TROLLZ_API}/api/mem/align`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        from: `session:${SESSION_ID}`,
        to: `codie:stats:${stats.turns}turns`,
        direction: '→',
      }),
      signal: AbortSignal.timeout(2000),
    });
  } catch {}
}

function barfInsertSession(projectPath) {
  const label = `session:${basename(projectPath)}:${Date.now()}`;
  spawnSync('barf', ['insert', label, '--tokens', '1'], { encoding: 'utf8', timeout: 2000 });
}

function sealCrystal(sessionId) {
  const pkgRoot = join(dirname(new URL(import.meta.url).pathname), '..');
  const localBin = join(pkgRoot, 'target/release/codec');
  const bin = existsSync(localBin) ? localBin : 'codec';
  return spawnSync(bin, ['crystal', '--session', sessionId],
    { encoding: 'utf8', timeout: 3000 });
}

async function main() {
  const input = JSON.parse(readFileSync('/dev/stdin', 'utf8'));
  const projectPath = input.cwd || process.cwd();
  const stats = loadCodieStats();

  barfInsertSession(projectPath);

  const crystalR = sealCrystal(SESSION_ID || 'gently-cc');

  const crystal = crystalCheckpoint(projectPath, stats);
  await flushToAlexandria(stats);

  const synth = crystal.synth_earned.toFixed(6);
  const avgPct = stats.turns > 0 ? (stats.pctSum / stats.turns).toFixed(1) : '0.0';

  const foamR = spawnSync('barf', ['stats'], { encoding: 'utf8', timeout: 2000 });
  const foamLine = foamR.stdout
    ? foamR.stdout.split('\n').slice(1).map(l => l.trim()).filter(Boolean).join(' · ')
    : 'unavailable';

  const codecCrystalId = crystalR && crystalR.stdout
    ? crystalR.stdout.split('\n')[0].trim()
    : null;

  process.stderr.write([
    ``,
    `── GENTLY-CC STOP ─────────────────────────────────────`,
    `   CODIE   : ${stats.tokensSaved.toLocaleString()} tokens saved · ${avgPct}% avg · ${stats.turns} turns`,
    `   FOAM    : ${foamLine}`,
    `   PINS    : ${codecCrystalId ? codecCrystalId.slice(0, 38) + '...' : 'no pin grid'}`,
    `   SYNTH   : +${synth} earned (QS: ${crystal.quality_score.toFixed(2)})`,
    `   Crystal : ${crystal.id.slice(0, 38)}...`,
    `────────────────────────────────────────────────────────`,
    ``,
  ].join('\n'));

  process.stdout.write(JSON.stringify({ continue: true }));
}

main().catch(() => {
  process.stdout.write(JSON.stringify({ continue: true }));
});
