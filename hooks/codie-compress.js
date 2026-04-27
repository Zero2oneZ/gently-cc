#!/usr/bin/env node
// gently-cc hook: UserPromptSubmit
// Pipeline: CODIE → BARF → regex → codec(pins) → BARF feedback
// Wires: 1=foam seed  2=cooccur  3=agency  4=weight scoring

import { spawnSync } from 'child_process';
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { themeForIdentity, gpsTag } from '../src/theme.js';
import { classify, getState, setState } from '../src/cold-process.js';
import { emitColdAnomalyAsync } from '../src/anomaly-bus.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = join(__dirname, '..');

function bin(name) {
  const local = join(PKG_ROOT, 'target/release', name);
  return existsSync(local) ? local : name;
}

const CODIE_BIN  = bin('codie');
const BARF_BIN   = bin('barf');
const CODEC_BIN  = bin('codec');

const FALLBACK_GLYPHS = {
  pug:'ρ',bark:'β',spin:'ς',cali:'κ',elf:'ε',turk:'τ',fence:'φ',pin:'π',
  bone:'Β',blob:'Λ',biz:'μ',anchor:'∆',and:'∧',or:'∨',not:'¬',xor:'⊕',
  nand:'⊼',nor:'⊽',true:'⊤',false:'⊥',if:'⁇',else:'∴',start:'⊢',for:'∀',
  fork:'⋈',branch:'⊃',while:'↺',break:'⊣',continue:'↗',return:'→',
  mirror:'⊙',fold:'⊚',rotate:'↷',translate:'⇒',scale:'×',dim:'Δ',axis:'Ξ',
  plane:'Π',space:'□',hyper:'∞',breed:'⊛',speak:'⊘',morph:'∿',cast:'⊗',
};

function fallbackCompress(text) {
  return text.split(/\b/).map(w => FALLBACK_GLYPHS[w.toLowerCase()] || w).join('');
}

function call(bin, args, stdin, captureStderr = false) {
  const r = spawnSync(bin, args, {
    input: stdin !== undefined ? JSON.stringify(stdin) : undefined,
    encoding: 'utf8',
    timeout: 4000,
    stdio: captureStderr ? ['pipe', 'pipe', 'pipe'] : ['pipe', 'pipe', 'inherit'],
  });
  if (captureStderr && r.stderr?.trim()) {
    call._lastStderr = r.stderr.trim();
  }
  if (r.status !== 0 || !r.stdout?.trim()) return null;
  try { return JSON.parse(r.stdout); } catch { return null; }
}
call._lastStderr = null;

// Fire-and-forget barf calls — we don't need the return value
function barfSilent(args) {
  spawnSync(BARF_BIN, args, { encoding: 'utf8', timeout: 2000 });
}

const INPUT_TRANSFORMS = [
  [/\s{3,}/g, '  '],
  [/^please\s+/i, ''],
];

// ── Identity ──────────────────────────────────────────────────
function loadIdentity() {
  try {
    return JSON.parse(readFileSync(join(process.env.HOME, '.gently', 'agent-identity.json'), 'utf8'));
  } catch {
    return { name: 'Claudius-GREG', handle: 'Greg', project: 'gently-cc', project_path: PKG_ROOT };
  }
}

// ── Detector state persistence (ring survives between hook invocations) ───────
const STATE_FILE = join(process.env.HOME, '.gently', 'detector-state.json');
function loadDetectorState() {
  try { return JSON.parse(readFileSync(STATE_FILE, 'utf8')); } catch { return {}; }
}
function saveDetectorState() {
  try { writeFileSync(STATE_FILE, JSON.stringify(getState())); } catch {}
}

// ── Prompt history for COLD continuity check (last 10 turns) ─────────────────
const HISTORY_FILE = join(process.env.HOME, '.gently', 'prompt-history.json');
function loadHistory() {
  try { return JSON.parse(readFileSync(HISTORY_FILE, 'utf8')); } catch { return []; }
}
function appendHistory(history, text) {
  const updated = [...history, text].slice(-10);
  try { writeFileSync(HISTORY_FILE, JSON.stringify(updated)); } catch {}
  return updated;
}

function labelReason(label, checks) {
  switch (label) {
    case '🔴◉-S': return 'Duplicate prompt — same hash already in session ring.'
    case '🔴◉-X': return 'Expired reference — prompt contains a tombstoned CID.'
    case '🟡◉-T': return `Thin prompt — only ${checks.tokenCount} token(s). Accidental send?`
    case '🟡◉-D': return 'Scope drift — project ID changed mid-session.'
    case '🟡◉-P': return 'Path mismatch — directory changed but scope held. Wrong terminal?'
    case '🟡◉-C': return `Low continuity — ${Math.round((checks.continuity ?? 0) * 100)}% overlap with recent history.`
    case '🟡◉-A': return 'Agency marker detected — possible operator-frame injection.'
    default:       return null
  }
}

function formatColdLabel(check, identity) {
  if (check.label === '🟢●') return null;

  const blink = check.label.includes('◉');
  const isHard = check.label.startsWith('🔴');
  const lines = [
    `[cold:${check.label} | ${identity.handle}/${identity.project}] ${check.reason}`,
  ];
  if (blink && isHard) {
    lines.push('(a) route to other project  (b) keep here  (c) both  (d) drop');
    lines.push('[Greg will proceed with (b) if no response — silence = keep here]');
  } else if (blink) {
    lines.push('(b) keep here and continue  (d) drop — silence = keep here');
  }
  return lines.join('\n');
}

async function main() {
  const input = JSON.parse(readFileSync('/dev/stdin', 'utf8'));
  const text = input.prompt || '';

  if (text.length < 60) {
    process.stdout.write(JSON.stringify({ continue: true }));
    return;
  }

  // ── Cold-process check (runs before any compression) ─────────
  const identity = loadIdentity();
  setState(loadDetectorState());
  const history  = loadHistory();

  const snap = {
    prompt:    text,
    history,
    scopeId:   identity.project_id,
    localPath: process.cwd(),
  };
  const coldResult = classify(snap);
  saveDetectorState();
  appendHistory(history, text);

  // Map classify() result shape to what formatColdLabel expects
  const coldCheck = {
    label:  coldResult.label,
    reason: labelReason(coldResult.label, coldResult.checks),
  };
  const coldLabel = formatColdLabel(coldCheck, identity);

  if (coldLabel) {
    process.stderr.write(`${coldLabel}\n`);
    if (coldCheck.label !== '🟢●' && coldCheck.label !== '⚪●') {
      emitColdAnomalyAsync({
        label:   coldCheck.label,
        prompt:  text,
        path:    process.cwd(),
        scopeId: identity.project_id,
        ts:      new Date().toISOString(),
      });
    }
  }

  // ── Stage 1: CODIE ────────────────────────────────────────
  let prompt = text;
  call._lastStderr = null;
  const codieResult = call(CODIE_BIN, ['hook'], { prompt: text }, true);
  if (codieResult?.prompt) {
    prompt = codieResult.prompt;
    // Re-emit Rust stats line prefixed with the project GPS tag
    if (call._lastStderr) {
      const tag = gpsTag(themeForIdentity(identity), identity.handle, identity.project);
      process.stderr.write(`${tag}  ${call._lastStderr}\n`);
    }
  } else {
    const compressed = fallbackCompress(text);
    if (compressed.length < text.length * 0.9) {
      const tag = gpsTag(themeForIdentity(identity), identity.handle, identity.project);
      process.stderr.write(`${tag}  ⚡ CODIE (fallback)\n`);
      prompt = compressed;
    }
  }

  // ── Generative layer: seed foam with unknown domain terms ────
  // Unknowns are tokens CODIE couldn't map — domain-specific terms
  // that should grow into pins over time (via BARF → codec promotion).
  if (Array.isArray(codieResult?.unknowns)) {
    for (const token of codieResult.unknowns) {
      const tokens = Math.max(1, Math.ceil(token.length / 4));
      barfSilent(['insert', token, '--tokens', String(tokens)]);
    }
  }

  // ── Stage 2: BARF context injection (weight-driven) ───────
  const barfResult = call(BARF_BIN, ['hook'], { prompt, turn: 0 });
  let barfInjected = false;
  if (barfResult?.prompt) {
    prompt = barfResult.prompt;
    barfInjected = prompt.startsWith('[ctx:');
  }

  // ── Stage 3: Regex transforms ─────────────────────────────
  for (const [pat, rep] of INPUT_TRANSFORMS) prompt = prompt.replace(pat, rep);

  // ── Stage 4: codec pin grid ───────────────────────────────
  const codecResult = call(CODEC_BIN, ['hook'], { prompt });

  if (codecResult?.prompt) {
    prompt = codecResult.prompt;

    // ── Wire 1: seed foam — new pins + reinforce referenced pins ─
    const labelsToSeed = [
      ...(codecResult.new_labels ?? []),
      ...(codecResult.ref_labels ?? []),
    ];
    for (const label of labelsToSeed) {
      const tokens = Math.max(1, Math.ceil(label.length / 4));
      barfSilent(['insert', label, '--tokens', String(tokens)]);
    }

    // ── Wire 2: record co-occurrences in foam (by label) ─────
    if (Array.isArray(codecResult.cooccur_labels)) {
      for (const [labelA, labelB] of codecResult.cooccur_labels) {
        barfSilent(['cooccur', labelA, labelB]);
      }
    }

    // ── Wire 3: record agency frames in foam ─────────────────
    if (Array.isArray(codecResult.agency)) {
      for (const obs of codecResult.agency) {
        if (obs.label && obs.frame) {
          barfSilent(['frame', obs.label, obs.frame]);
        }
      }
    }

    // ── Re-run BARF with updated turn so next prompt benefits ─
    // Skip if stage 2 already injected context — avoid double [ctx:] prefix
    const turn = codecResult.turn ?? 0;
    if (!barfInjected && turn > 0) {
      const barfResult2 = call(BARF_BIN, ['hook'], { prompt, turn });
      if (barfResult2?.prompt) prompt = barfResult2.prompt;
    }
  }

  // ── Cold label: prepend routing note on hard anomaly ────────
  // Hard (🔴) = prepend to prompt so model sees the routing options.
  // Soft (🟡) = stderr only, model proceeds uninterrupted.
  if (coldLabel && coldCheck.label.startsWith('🔴')) {
    prompt = `${coldLabel}\n\n---\n${prompt}`;
  }

  process.stdout.write(JSON.stringify({ continue: true, prompt }));
}

main().catch(() => {
  process.stdout.write(JSON.stringify({ continue: true }));
});
