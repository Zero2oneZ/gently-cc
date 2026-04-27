// gently/src/agents.js — agent launcher
//
// Phase 1: look up agent CID or task ID in plan.dag.json,
//          start a gently chat session with the task spec as system context.
// Phase 2: pull CID from IPFS → materialize CODIE spec → gen output → run.
//
// trollz.fun agents, VOS Orcs, Unreal plugin agents — all the same thing:
// a CID, a CODIE spec, a system prompt. One primitive.

import { existsSync, readFileSync } from 'node:fs'
import { join, dirname }            from 'node:path'
import { fileURLToPath }            from 'node:url'
import { startChat }                from './chat.js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const PKG_ROOT  = join(__dirname, '..', '..')
const DAG_FILE  = join(PKG_ROOT, 'plan.dag.json')

// Pre-defined trollz.fun agent specs — each is a CODIE expression + system role.
// Phase 2: these live as CIDs in IPFS, pulled by gently launch <cid>.
const KNOWN_AGENTS = {
  'lurk':    { role: 'read-only recon',        codie: 'κ[lurk] φ[write] μ[observe]' },
  'grub':    { role: 'scaffolding',             codie: 'κ[grub] μ[scaffold] Λ[any]' },
  'slog':    { role: 'DB migrations',           codie: 'κ[slog] μ[migrate] Β[schema]' },
  'worg':    { role: 'service wrappers',        codie: 'κ[worg] μ[wrap] ε[service]' },
  'zug':     { role: 'API routes',              codie: 'κ[zug] μ[route] ε[endpoint]' },
  'morg':    { role: 'frontend',                codie: 'κ[morg] μ[render] ε[component]' },
  'krak':    { role: 'tests',                   codie: 'κ[krak] μ[test] Λ[coverage]' },
  'codie':   { role: 'CODIE pipeline',          codie: 'κ[codie] μ[compress] Β[canonical]' },
  'forge':   { role: 'bb-runtime',              codie: 'κ[forge] μ[runtime] ε[crystal]' },
  'chain':   { role: 'Sui contracts',           codie: 'κ[chain] μ[contract] Β[linear]' },
  'grim':    { role: 'manifest scanner',        codie: 'κ[grim] μ[hash] Β[dispatch]' },
  // Unreal Engine plugin agents
  'ue-scene':  { role: 'Unreal scene query',    codie: 'κ[ue-scene] β[scene/actors] μ[query]' },
  'ue-asset':  { role: 'Unreal asset manager',  codie: 'κ[ue-asset] β[content/browser] μ[manage]' },
  'ue-bp':     { role: 'Unreal blueprint agent', codie: 'κ[ue-bp] β[blueprint/graph] μ[generate]' },
}

export async function launchAgent(cidOrName, opts = {}) {
  // Check known agent names first
  const known = KNOWN_AGENTS[cidOrName.toLowerCase()]
  if (known) {
    const system = buildSystemPrompt(cidOrName, known)
    console.log(`  Agent: ${cidOrName} — ${known.role}`)
    console.log(`  Spec:  ${known.codie}\n`)
    await startChat({ ...opts, system })
    return
  }

  // Check plan.dag.json task by ID or CID
  if (existsSync(DAG_FILE)) {
    const dag  = JSON.parse(readFileSync(DAG_FILE, 'utf8'))
    const task = Object.values(dag.tasks).find(t =>
      t.id === cidOrName || t.task_cid === cidOrName
    )
    if (task) {
      const system = [
        `You are a specialized agent executing task [${task.id}]: ${task.title}.`,
        `Phase: ${task.phase}. Files: ${task.files.join(', ') || 'see PLAN.md'}.`,
        `Required prior tasks: ${task.requires.join(', ') || 'none'}.`,
        `Complete this task fully before responding. Apply gently compression and memory.`,
      ].join(' ')
      console.log(`  Task: [${task.id}] ${task.title}`)
      console.log(`  Files: ${task.files.join(', ') || '(see PLAN.md)'}\n`)
      await startChat({ ...opts, system })
      return
    }
  }

  // Phase 2: IPFS pull
  console.log(`  Phase 2 (IPFS pull for ${cidOrName}) — not yet implemented.`)
  console.log('  Available agents: ' + Object.keys(KNOWN_AGENTS).join(', '))
  console.log('  Or pass a task ID from plan.dag.json (e.g. A.1.4)\n')
}

function buildSystemPrompt(name, agent) {
  return [
    `You are the ${name} agent (${agent.role}).`,
    `CODIE spec: ${agent.codie}.`,
    `Stay within your orc boundary. Be precise and complete.`,
    `Apply gently compression norms: dense, no filler.`,
  ].join(' ')
}
