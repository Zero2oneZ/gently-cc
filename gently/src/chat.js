// gently/src/chat.js — REPL with full pipeline
//
// Every turn:
//   user input → classify (cold-process) → CODIE compress → LLM stream
//   → seed BARF foam → emit GPS tag + compression stats
//
// Works with any provider. The compression and memory layers are
// provider-agnostic — swap Anthropic for OpenAI and nothing changes.

import { createInterface }   from 'node:readline'
import { spawnSync }         from 'node:child_process'
import { existsSync }        from 'node:fs'
import { join, dirname }     from 'node:path'
import { fileURLToPath }     from 'node:url'

import { classify, getState, setState } from '../../src/cold-process.js'
import { emitColdAnomalyAsync }         from '../../src/anomaly-bus.js'
import { themeForIdentity, gpsTag }     from '../../src/theme.js'
import { chat, modelTag }               from './llm.js'
import { discoverProviders }            from './keys.js'
import { loadDetectorState, saveDetectorState, loadHistory, appendHistory, loadIdentity } from './session.js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const PKG_ROOT  = join(__dirname, '..', '..')

function bin(name) {
  const p = join(PKG_ROOT, 'target', 'release', name)
  return existsSync(p) ? p : name
}

const CODIE_BIN = bin('codie')
const BARF_BIN  = bin('barf')

function codie(text) {
  const r = spawnSync(CODIE_BIN, ['hook'], {
    input: JSON.stringify({ prompt: text }), encoding: 'utf8', timeout: 3000,
  })
  if (r.status !== 0 || !r.stdout?.trim()) return { prompt: text, saved: 0, ratio: 0 }
  try {
    const d = JSON.parse(r.stdout)
    return { prompt: d.prompt ?? text, saved: d.tokens_saved ?? 0, ratio: d.ratio ?? 0, unknowns: d.unknowns ?? [] }
  } catch { return { prompt: text, saved: 0, ratio: 0 } }
}

function barfInsert(term) {
  spawnSync(BARF_BIN, ['insert', term, '--tokens', String(Math.max(1, Math.ceil(term.length / 4)))],
    { encoding: 'utf8', timeout: 1500 })
}

function barfContext(prompt) {
  const r = spawnSync(BARF_BIN, ['hook'], {
    input: JSON.stringify({ prompt, turn: 0 }), encoding: 'utf8', timeout: 2000,
  })
  try { return JSON.parse(r.stdout)?.prompt ?? prompt } catch { return prompt }
}

// ── Session stats ─────────────────────────────────────────────────────────────

let _sessionTokensSaved = 0
let _sessionTurns       = 0

// ── Main export ───────────────────────────────────────────────────────────────

export async function startChat(opts = {}) {
  const identity = loadIdentity(PKG_ROOT)
  const theme    = themeForIdentity(identity)
  const tag      = gpsTag(theme, identity.handle, identity.project)
  const { providers, default: defaultProvider } = discoverProviders()

  if (!defaultProvider) {
    console.error('\n  No API key found. Set ANTHROPIC_API_KEY or OPENAI_API_KEY.\n')
    process.exit(1)
  }

  const provider = opts.provider ?? defaultProvider
  const model    = opts.model    ?? undefined
  const mTag     = modelTag({ provider, model })

  console.log(`\n${tag}  gently — ${mTag}`)
  console.log(`  ${providers.join(' · ')} · /help · /exit\n`)

  const rl = createInterface({ input: process.stdin, output: process.stdout, terminal: true })
  const messages = []

  // System prompt: Greg's identity + CODIE context
  const system = opts.system ?? [
    `You are ${identity.handle} (${identity.designation}), project-scoped agent for ${identity.project}.`,
    `Be concise and direct. You are running inside the gently substrate with CODIE compression active.`,
  ].join(' ')

  setState(loadDetectorState(PKG_ROOT))
  let history = loadHistory(PKG_ROOT)

  const prompt = (q) => new Promise(resolve => rl.question(q, resolve))

  while (true) {
    const raw = await prompt(`${tag} › `)

    // Slash commands
    if (raw.trim() === '/exit' || raw.trim() === '/quit') break
    if (raw.trim() === '/help') { _printHelp(tag, mTag); continue }
    if (raw.trim() === '/stats') { _printStats(tag, _sessionTurns, _sessionTokensSaved); continue }
    if (raw.trim() === '/clear') { messages.length = 0; console.log('  context cleared\n'); continue }
    if (raw.trim().startsWith('/model ')) {
      opts.model = raw.trim().slice(7).trim()
      console.log(`  model → ${opts.model}\n`); continue
    }
    if (!raw.trim()) continue

    // ── Cold-process classify ────────────────────────────────
    setState(loadDetectorState(PKG_ROOT))
    const snap   = { prompt: raw, history, scopeId: identity.project_id, localPath: process.cwd() }
    const result = classify(snap)
    saveDetectorState(PKG_ROOT)
    history = appendHistory(PKG_ROOT, history, raw)

    if (result.label !== '🟢●' && result.label !== '⚪●') {
      const reason = _labelReason(result.label, result.checks)
      process.stderr.write(`  [cold:${result.label}] ${reason}\n`)
      emitColdAnomalyAsync({ label: result.label, prompt: raw, path: process.cwd(),
        scopeId: identity.project_id, ts: new Date().toISOString() })
      if (result.label.startsWith('🔴')) {
        const ans = await prompt('  Proceed? (y/n) › ')
        if (ans.trim().toLowerCase() !== 'y') continue
      }
    }

    // ── CODIE compress ───────────────────────────────────────
    const compressed = codie(raw)
    for (const u of compressed.unknowns) barfInsert(u)
    const withCtx = barfContext(compressed.prompt)

    messages.push({ role: 'user', content: withCtx })

    // ── Stream LLM response ──────────────────────────────────
    process.stdout.write('\n')
    let fullResponse = ''
    try {
      for await (const chunk of chat(messages, { provider, model: opts.model, system })) {
        process.stdout.write(chunk)
        fullResponse += chunk
      }
    } catch (e) {
      console.error(`\n  [error] ${e.message}\n`)
      messages.pop()
      continue
    }
    process.stdout.write('\n')

    messages.push({ role: 'assistant', content: fullResponse })

    // Seed foam with key terms from response
    const terms = fullResponse.match(/`([^`]+)`/g) ?? []
    for (const t of terms.slice(0, 6)) barfInsert(t.slice(1, -1))

    // ── Stats footer ─────────────────────────────────────────
    _sessionTokensSaved += compressed.saved
    _sessionTurns++
    if (compressed.saved > 0) {
      const pct = Math.round(compressed.ratio * 100)
      process.stdout.write(`  ${tag}  ⚡ ${pct}% compression · ${compressed.saved} tokens saved · session: ${_sessionTokensSaved} saved\n\n`)
    } else {
      process.stdout.write(`  ${tag}  turn ${_sessionTurns}\n\n`)
    }
  }

  rl.close()
  console.log(`\n  Session: ${_sessionTurns} turns · ${_sessionTokensSaved} tokens saved\n`)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function _printHelp(tag, mTag) {
  console.log(`\n  ${tag}  ${mTag}`)
  console.log('  /clear   — clear context window')
  console.log('  /stats   — show session stats')
  console.log('  /model <name> — switch model mid-session')
  console.log('  /exit    — quit\n')
}

function _printStats(tag, turns, saved) {
  console.log(`\n  ${tag}  turns: ${turns} · tokens saved: ${saved}\n`)
}

function _labelReason(label, checks) {
  switch (label) {
    case '🔴◉-S': return 'Duplicate prompt in ring.'
    case '🔴◉-X': return 'Tombstoned CID reference.'
    case '🟡◉-T': return `Thin — ${checks.tokenCount} token(s).`
    case '🟡◉-D': return 'Scope drift.'
    case '🟡◉-P': return 'Path mismatch.'
    case '🟡◉-C': return `Low continuity (${Math.round((checks.continuity ?? 0) * 100)}%).`
    case '🟡◉-A': return 'Agency marker detected.'
    default:       return label
  }
}
