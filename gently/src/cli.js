#!/usr/bin/env node
// gently — universal agentic CLI
// CODIE compression + BARF memory on top of any LLM.
// Claude, GPT-4o, or anything OpenAI-compatible. Bring your own key.

import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { existsSync }    from 'node:fs'

const __dirname = dirname(fileURLToPath(import.meta.url))

const COMMANDS = {
  chat:       'Start a compressed, memory-aware chat session',
  keys:       'Show discovered API keys and providers',
  launch:     'Launch a trollz.fun agent by CID:  gently launch <cid>',
  plan:       'Show the project task DAG:          gently plan [--open]',
  claim:      'Claim a task:                       gently claim <task_id>',
  anchor:     'Anchor project state on chain:      gently anchor',
  clone:      'Clone a project from CID:           gently clone <cid>',
  materialize:'Regenerate all files from codex:    gently materialize [--verify]',
}

async function main() {
  const [,, cmd, ...args] = process.argv

  if (!cmd || cmd === '--help' || cmd === '-h') {
    console.log('\ngently — universal agentic CLI\n')
    console.log('  CODIE compression + BARF memory on top of any LLM.\n')
    console.log('Usage: gently <command> [options]\n')
    for (const [name, desc] of Object.entries(COMMANDS)) {
      console.log(`  ${name.padEnd(14)} ${desc}`)
    }
    console.log('\nProviders:  ANTHROPIC_API_KEY · OPENAI_API_KEY · ~/.claude/settings.json')
    console.log('Models:     claude-sonnet-4-6 · gpt-4o · any OpenAI-compatible endpoint')
    console.log('\nDocs:  https://gentlyos.io\n')
    return
  }

  // Parse --provider and --model flags from args
  const providerIdx = args.indexOf('--provider')
  const modelIdx    = args.indexOf('--model')
  const provider    = providerIdx >= 0 ? args[providerIdx + 1] : undefined
  const model       = modelIdx    >= 0 ? args[modelIdx    + 1] : undefined

  switch (cmd) {
    case 'chat': {
      const { startChat } = await import('./chat.js')
      await startChat({ provider, model })
      break
    }

    case 'keys': {
      const { discoverProviders, getAnthropicKey, getOpenAIKey } = await import('./keys.js')
      const disc = discoverProviders()
      console.log('\n  gently — provider discovery\n')
      console.log(`  anthropic  ${disc.anthropic ? '✓ key found' : '✗ not found'}`)
      console.log(`  openai     ${disc.openai    ? '✓ key found' : '✗ not found'}`)
      console.log(`  default    ${disc.default   ?? '(none)'}`)
      if (!disc.default) {
        console.log('\n  Set one of:')
        console.log('    export ANTHROPIC_API_KEY=sk-ant-...')
        console.log('    export OPENAI_API_KEY=sk-...')
        console.log('    or run: gently keys set\n')
      }
      console.log('')
      break
    }

    case 'keys set': {
      // Interactive key setup — prompts and saves to ~/.config/gently/keys.json
      const { saveKey } = await import('./keys.js')
      const { createInterface } = await import('node:readline')
      const rl = createInterface({ input: process.stdin, output: process.stdout })
      const ask = (q) => new Promise(r => rl.question(q, r))
      console.log('\n  gently — set API keys\n')
      const p = await ask('  Provider (anthropic/openai): ')
      const k = await ask(`  ${p} API key: `)
      saveKey(p.trim(), k.trim())
      rl.close()
      console.log(`  Saved to ~/.config/gently/keys.json\n`)
      break
    }

    case 'launch': {
      const cid = args.find(a => !a.startsWith('--'))
      if (!cid) { console.error('  Usage: gently launch <agent_cid>\n'); process.exit(1) }
      console.log(`\n  Launching agent: ${cid}\n`)
      // Phase 2: pull CID from IPFS → materialize spec → run
      // Phase 1: look up in plan.dag.json and start a chat with that task as system prompt
      const { launchAgent } = await import('./agents.js')
      await launchAgent(cid, { provider, model })
      break
    }

    case 'plan': {
      // Re-use gently-cc's plan.js from the parent package
      const pkgRoot = join(__dirname, '..', '..')
      const { spawnSync } = await import('node:child_process')
      const r = spawnSync('node', [join(pkgRoot, 'src', 'plan.js')], { stdio: 'inherit' })
      process.exit(r.status ?? 0)
    }

    case 'claim': {
      const taskId = args.find(a => !a.startsWith('--'))
      if (!taskId) { console.error('  Usage: gently claim <task_id>\n'); process.exit(1) }
      const pkgRoot = join(__dirname, '..', '..')
      const { spawnSync } = await import('node:child_process')
      const r = spawnSync('node', [join(pkgRoot, 'src', 'cli.js'), 'claim', taskId], { stdio: 'inherit' })
      process.exit(r.status ?? 0)
    }

    case 'anchor':
    case 'clone':
    case 'materialize': {
      // Delegate to gently-cc's implementations
      const pkgRoot = join(__dirname, '..', '..')
      const { spawnSync } = await import('node:child_process')
      const r = spawnSync('node', [join(pkgRoot, 'src', 'cli.js'), cmd, ...args], { stdio: 'inherit' })
      process.exit(r.status ?? 0)
    }

    default:
      console.error(`  Unknown command: ${cmd}\n  Run: gently --help\n`)
      process.exit(1)
  }
}

main().catch(e => { console.error(`  ${e.message}`); process.exit(1) })
