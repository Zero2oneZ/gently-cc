// gently/src/llm.js — unified LLM client
//
// One call surface: chat(messages, opts) → AsyncIterator<chunk>
// Provider is selected by opts.provider or auto-detected from available keys.
// Streaming first — callers always get an async iterator, even for non-streaming
// providers (we wrap the single response in a one-item iterator).
//
// Supported:
//   anthropic  → claude-sonnet-4-6 (default), any claude-* model
//   openai     → gpt-4o (default), any OpenAI model, any compatible endpoint
//   compatible → anything with an OpenAI-compatible /v1/chat/completions endpoint

import { getAnthropicKey, getOpenAIKey, discoverProviders } from './keys.js'

// ── Model defaults ────────────────────────────────────────────────────────────

export const DEFAULTS = {
  anthropic: 'claude-sonnet-4-6',
  openai:    'gpt-4o',
}

// ── Main export ───────────────────────────────────────────────────────────────

/**
 * Stream a chat completion.
 * @param {Array<{role:'user'|'assistant'|'system', content:string}>} messages
 * @param {{ provider?:string, model?:string, maxTokens?:number, system?:string }} opts
 * @yields {string} text chunks as they arrive
 */
export async function* chat(messages, opts = {}) {
  const { providers, default: defaultProvider } = discoverProviders()

  const provider = opts.provider ?? defaultProvider
  if (!provider) {
    throw new Error(
      'No API key found.\n' +
      'Set ANTHROPIC_API_KEY or OPENAI_API_KEY, or run: gently keys set'
    )
  }

  if (provider === 'anthropic') {
    yield* _anthropicStream(messages, opts)
  } else if (provider === 'openai' || provider === 'compatible') {
    yield* _openaiStream(messages, opts)
  } else {
    throw new Error(`Unknown provider: ${provider}. Supported: anthropic, openai, compatible`)
  }
}

/**
 * Collect full response (non-streaming convenience wrapper).
 */
export async function chatFull(messages, opts = {}) {
  let out = ''
  for await (const chunk of chat(messages, opts)) out += chunk
  return out
}

/**
 * Return provider + model string for display.
 * e.g. "anthropic/claude-sonnet-4-6"
 */
export function modelTag(opts = {}) {
  const { default: provider } = discoverProviders()
  const p     = opts.provider ?? provider ?? 'unknown'
  const model = opts.model    ?? DEFAULTS[p] ?? 'unknown'
  return `${p}/${model}`
}

// ── Anthropic ─────────────────────────────────────────────────────────────────

async function* _anthropicStream(messages, opts) {
  const { default: Anthropic } = await import('@anthropic-ai/sdk')
  const key = getAnthropicKey()
  if (!key) throw new Error('ANTHROPIC_API_KEY not found')

  const client = new Anthropic({ apiKey: key })
  const model  = opts.model    ?? DEFAULTS.anthropic
  const max    = opts.maxTokens ?? 8096

  // Anthropic separates system from messages
  const sys  = opts.system ?? null
  const msgs = messages.filter(m => m.role !== 'system')

  const stream = client.messages.stream({
    model,
    max_tokens: max,
    ...(sys ? { system: sys } : {}),
    messages: msgs,
  })

  for await (const event of stream) {
    if (event.type === 'content_block_delta' && event.delta?.type === 'text_delta') {
      yield event.delta.text
    }
  }
}

// ── OpenAI / compatible ───────────────────────────────────────────────────────

async function* _openaiStream(messages, opts) {
  const { default: OpenAI } = await import('openai')
  const key     = getOpenAIKey()
  const baseURL = opts.baseURL ?? process.env.OPENAI_BASE_URL ?? undefined
  if (!key && !baseURL) throw new Error('OPENAI_API_KEY not found')

  const client  = new OpenAI({ apiKey: key ?? 'local', ...(baseURL ? { baseURL } : {}) })
  const model   = opts.model ?? DEFAULTS.openai
  const max     = opts.maxTokens ?? 4096

  // Merge system into messages array (OpenAI style)
  const msgs = opts.system
    ? [{ role: 'system', content: opts.system }, ...messages]
    : messages

  const stream = await client.chat.completions.create({
    model,
    max_tokens: max,
    stream: true,
    messages: msgs,
  })

  for await (const chunk of stream) {
    const text = chunk.choices[0]?.delta?.content
    if (text) yield text
  }
}
