// gently/src/keys.js — multi-provider API key discovery
//
// Priority order for each provider:
//   1. Environment variable (fastest, CI-friendly)
//   2. Well-known CLI config files (user already authenticated elsewhere)
//   3. gently's own config (~/.config/gently/keys.json)
//   4. null — caller prompts user and saves to gently config
//
// Never writes keys anywhere except ~/.config/gently/keys.json (explicit save).
// Never reads from project directories — keys don't belong in codebases.

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs'
import { homedir } from 'node:os'
import { join }    from 'node:path'

const HOME       = homedir()
const GENTLY_CFG = join(HOME, '.config', 'gently', 'keys.json')

// ── Discovery ────────────────────────────────────────────────────────────────

export function getAnthropicKey() {
  return (
    process.env.ANTHROPIC_API_KEY                              ||
    _fromClaudeCli()                                           ||
    _fromGentlyConfig('anthropic')                             ||
    null
  )
}

export function getOpenAIKey() {
  return (
    process.env.OPENAI_API_KEY                                 ||
    _fromOpenAICli()                                           ||
    _fromGentlyConfig('openai')                                ||
    null
  )
}

/**
 * Discover all available providers.
 * Returns { anthropic: bool, openai: bool, providers: string[] }
 */
export function discoverProviders() {
  const anthropic = !!getAnthropicKey()
  const openai    = !!getOpenAIKey()
  return {
    anthropic,
    openai,
    providers:  [anthropic && 'anthropic', openai && 'openai'].filter(Boolean),
    default:    anthropic ? 'anthropic' : openai ? 'openai' : null,
  }
}

/**
 * Save a key to ~/.config/gently/keys.json for future sessions.
 */
export function saveKey(provider, key) {
  mkdirSync(join(HOME, '.config', 'gently'), { recursive: true })
  const existing = _fromGentlyConfigRaw()
  existing[provider] = key
  writeFileSync(GENTLY_CFG, JSON.stringify(existing, null, 2) + '\n', 'utf8')
}

// ── Private readers ───────────────────────────────────────────────────────────

function _fromClaudeCli() {
  // Claude Code stores the key in ~/.claude/settings.json under apiKey or
  // in the environment it was launched with. Check both locations.
  const paths = [
    join(HOME, '.claude', 'settings.json'),
    join(HOME, '.claude', '.credentials.json'),
  ]
  for (const p of paths) {
    try {
      const d = JSON.parse(readFileSync(p, 'utf8'))
      const k = d.apiKey || d.anthropicApiKey || d.api_key
      if (k && k.startsWith('sk-ant-')) return k
    } catch {}
  }
  return null
}

function _fromOpenAICli() {
  // openai CLI stores in ~/.config/openai/credentials or OPENAI_API_KEY
  const paths = [
    join(HOME, '.config', 'openai', 'credentials'),
    join(HOME, '.openai', 'credentials'),
  ]
  for (const p of paths) {
    try {
      // INI-like: api_key = sk-...  OR JSON
      const raw = readFileSync(p, 'utf8')
      const match = raw.match(/api[_-]key\s*[=:]\s*(sk-[A-Za-z0-9\-_]+)/)
      if (match) return match[1]
      const d = JSON.parse(raw)
      const k = d.api_key || d.apiKey || d.openai_api_key
      if (k && k.startsWith('sk-')) return k
    } catch {}
  }
  return null
}

function _fromGentlyConfig(provider) {
  return _fromGentlyConfigRaw()[provider] || null
}

function _fromGentlyConfigRaw() {
  try { return JSON.parse(readFileSync(GENTLY_CFG, 'utf8')) } catch { return {} }
}
