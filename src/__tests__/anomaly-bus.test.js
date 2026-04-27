// Anomaly bus emitter tests — no real network, no real filesystem writes.
// Run: node --test src/__tests__/anomaly-bus.test.js

import { describe, it, afterEach } from 'node:test'
import assert from 'node:assert/strict'
import { emitColdAnomalyAsync } from '../anomaly-bus.js'

function setChainUrl(url) {
  if (url) process.env.TROLLZ_API_URL = url
  else     delete process.env.TROLLZ_API_URL
}

function mockFetch(handler) {
  const original    = globalThis.fetch
  globalThis.fetch  = async (url, init) => handler(url.toString(), init)
  return () => { globalThis.fetch = original }
}

function makeEvent(overrides = {}) {
  return {
    label:   '🟡◉-C',
    prompt:  'tell me about the raspberry pi overclock heatsink temperature',
    path:    '/home/tom/Projects/TrollzDotFun',
    scopeId: 'scope-uuid-test',
    ts:      '2026-04-27T12:00:00.000Z',
    ...overrides,
  }
}

function tick(ms = 20) {
  return new Promise(r => setTimeout(r, ms))
}

afterEach(() => setChainUrl(undefined))

// ── chain POST ────────────────────────────────────────────────────────────────

describe('emitColdAnomalyAsync — chain POST', () => {
  it('does not call fetch when TROLLZ_API_URL is not set', async () => {
    setChainUrl(undefined)
    let called = false
    const restore = mockFetch(() => { called = true; return new Response('') })
    emitColdAnomalyAsync(makeEvent())
    await tick()
    restore()
    assert.equal(called, false)
  })

  it('POSTs to /ops/emit with correct event shape', async () => {
    setChainUrl('http://localhost:8080')
    let capturedBody

    const restore = mockFetch((url, init) => {
      if (url.includes('/ops/emit')) {
        capturedBody = JSON.parse(init?.body)
        return new Response(JSON.stringify({ id: 'evt-1', status: 'pending' }), { status: 200 })
      }
      return new Response('not found', { status: 404 })
    })

    emitColdAnomalyAsync(makeEvent())
    await tick()
    restore()

    assert.ok(capturedBody)
    assert.equal(capturedBody.event_type, 'cc.cold_process')
    assert.equal(capturedBody.source, 'cc')
    assert.equal(capturedBody.subsystem, 'cold_process')
    assert.equal(capturedBody.payload.label, '🟡◉-C')
    assert.equal(capturedBody.payload.path, '/home/tom/Projects/TrollzDotFun')
  })

  it('truncates prompt to 120 chars in the POST body', async () => {
    setChainUrl('http://localhost:8080')
    let capturedPayload

    const restore = mockFetch((url, init) => {
      if (url.includes('/ops/emit')) {
        capturedPayload = JSON.parse(init?.body).payload
        return new Response('{}', { status: 200 })
      }
      return new Response('', { status: 404 })
    })

    emitColdAnomalyAsync(makeEvent({ prompt: 'x'.repeat(300) }))
    await tick()
    restore()

    assert.ok(capturedPayload.prompt.length <= 120)
  })

  it('does not throw when /ops/emit returns 500', async () => {
    setChainUrl('http://localhost:8080')
    const restore = mockFetch(() => new Response('error', { status: 500 }))
    assert.doesNotThrow(() => emitColdAnomalyAsync(makeEvent()))
    await tick()
    restore()
  })

  it('does not throw when fetch throws (network down)', async () => {
    setChainUrl('http://localhost:8080')
    const restore = mockFetch(() => { throw new Error('ECONNREFUSED') })
    assert.doesNotThrow(() => emitColdAnomalyAsync(makeEvent()))
    await tick()
    restore()
  })
})

// ── all labels route correctly ────────────────────────────────────────────────

describe('emitColdAnomalyAsync — all non-OK labels route to chain', () => {
  const nonOkLabels = ['🔴◉-S', '🔴◉-X', '🟡◉-T', '🟡◉-D', '🟡◉-P', '🟡◉-C', '🟡◉-A']

  for (const label of nonOkLabels) {
    it(`emits for label ${label}`, async () => {
      setChainUrl('http://localhost:8080')
      let emittedLabel

      const restore = mockFetch((url, init) => {
        if (url.includes('/ops/emit')) {
          emittedLabel = JSON.parse(init?.body).payload?.label
          return new Response('{}', { status: 200 })
        }
        return new Response('', { status: 404 })
      })

      emitColdAnomalyAsync(makeEvent({ label }))
      await tick()
      restore()

      assert.equal(emittedLabel, label)
    })
  }
})
