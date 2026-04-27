// Cross-session anomaly emitter — two sinks, both fire-and-forget, both fail silently.
//
// Sink 1: ~/.gently/anomalies.jsonl — machine-wide log, every CC session can tail it.
// Sink 2: anchor.js chain sink — if TROLLZ_API_URL is set, POST to /ops/emit.
//         (replaces the CC-fork watchdog; same wire, public trollz-api endpoint)
//
// Neither sink blocks the response. Both are optional.
// No watchdog, no CC-fork deps, no getWalletSession.

import { appendFileSync, mkdirSync }  from 'node:fs'
import { homedir }                    from 'node:os'
import { join }                       from 'node:path'

/**
 * Fire-and-forget anomaly emit. Call after any non-OK label. Never throws.
 * @param {{ label: string, prompt: string, path: string, scopeId?: string, ts: string }} event
 */
export function emitColdAnomalyAsync(event) {
  _emit(event).catch(() => {})
}

async function _emit(event) {
  const truncated = { ...event, prompt: event.prompt.slice(0, 120) }
  await Promise.allSettled([
    _emitToChain(truncated),
    _emitToFile(truncated),
  ])
}

async function _emitToChain(event) {
  const baseUrl = process.env.TROLLZ_API_URL
  if (!baseUrl) return

  await fetch(`${baseUrl}/ops/emit`, {
    method:  'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      event_type: 'cc.cold_process',
      source:     'cc',
      subsystem:  'cold_process',
      scope_id:   event.scopeId,
      payload: {
        label:  event.label,
        prompt: event.prompt,
        path:   event.path,
      },
    }),
    signal: AbortSignal.timeout(5_000),
  })
}

function _emitToFile(event) {
  return new Promise((resolve) => {
    try {
      const dir = join(homedir(), '.gently')
      mkdirSync(dir, { recursive: true })
      appendFileSync(join(dir, 'anomalies.jsonl'), JSON.stringify(event) + '\n', 'utf8')
    } catch {}
    resolve()
  })
}
