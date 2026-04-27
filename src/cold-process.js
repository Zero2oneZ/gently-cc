// Cold-process anomaly detector — deterministic, no LLM, no network.
//
// Classifies each turn using pure logic. First match wins (priority top-down):
//   🔴◉-S  STALE    duplicate slot (same prompt hash in last 8 turns)
//   🔴◉-X  EXPIRED  tombstoned CID referenced in prompt
//   🟡◉-T  THIN     very short prompt (< 12 tokens) — likely accidental
//   🟡◉-D  DRIFT    scope ID changed mid-session
//   🟡◉-P  PATH     cwd changed but scope ID held — wrong session for this dir
//   🟡◉-C  COLD     low lexical continuity with recent history (< 20%)
//   🟡◉-A  AGENCY   operator-frame markers present (ambiguous intent signal)
//   🟢●    OK       all checks passed
//   ⚪●    UNKNOWN  detector threw (fail-open, never blocks)
//
// Augments gently-cc/hooks/codie-compress.js (which fires the same label set
// via Rust binary calls). This module adds the 5 axes that need zero Rust deps:
// THIN, COLD, AGENCY, DRIFT, PATH.

const CONTINUITY_THRESHOLD = 0.20
const THIN_TOKEN_THRESHOLD = 12
const STALE_WINDOW         = 8
const AGENCY_MARKERS_RE    = /(?:^|\s)[?><!=>]{1,2}(?:\s|$)/m

// Module-level ring state — single-session, reset on session start
let _lastScopeId   = undefined
let _lastLocalPath = undefined
let _slotRing      = []

/**
 * Classify the current turn. Never throws — returns ⚪● on any error.
 * @param {{ prompt: string, history: string[], scopeId?: string, localPath?: string, activePins?: string[], tombstonedPins?: string[] }} snap
 * @returns {{ label: string, checks: Record<string, boolean|string|number> }}
 */
export function classify(snap) {
  try {
    return _classify(snap)
  } catch {
    return { label: '⚪●', checks: { error: true } }
  }
}

/** Format a label for footer injection: two leading spaces + label. */
export function formatLabel(label) {
  return `  ${label}`
}

/** Reset module-level state. Call between test cases or on session start. */
export function resetState() {
  _lastScopeId   = undefined
  _lastLocalPath = undefined
  _slotRing      = []
}

/**
 * Serialize ring state for cross-process persistence.
 * Each hook invocation is a new process — serialize after classify(), restore before.
 */
export function getState() {
  return { slotRing: [..._slotRing], lastScopeId: _lastScopeId, lastLocalPath: _lastLocalPath }
}

/** Restore previously serialized state. */
export function setState(s) {
  if (Array.isArray(s.slotRing))        _slotRing      = s.slotRing
  if (s.lastScopeId   !== undefined)    _lastScopeId   = s.lastScopeId
  if (s.lastLocalPath !== undefined)    _lastLocalPath = s.lastLocalPath
}

// ── Internals ─────────────────────────────────────────────────────────────────

function _classify(snap) {
  const checks = {}
  const tokens = tokenize(snap.prompt)

  // STALE: same prompt hash in last STALE_WINDOW turns
  const hash    = slotHash(snap.prompt)
  const isStale = _slotRing.includes(hash)
  checks.stale  = isStale
  _slotRing.push(hash)
  if (_slotRing.length > STALE_WINDOW) _slotRing.shift()
  if (isStale) return { label: '🔴◉-S', checks }

  // EXPIRED: prompt references a tombstoned CID
  const tombstones   = snap.tombstonedPins ?? []
  const hasTombstone = tombstones.some(cid => snap.prompt.includes(cid))
  checks.tombstone   = hasTombstone
  if (hasTombstone) return { label: '🔴◉-X', checks }

  // THIN: very short prompt
  checks.tokenCount = tokens.length
  if (tokens.length < THIN_TOKEN_THRESHOLD) return { label: '🟡◉-T', checks }

  // DRIFT + PATH: scope and directory consistency
  const scopeId   = snap.scopeId   ?? process.env.GENTLYOS_SCOPE_ID
  const localPath = snap.localPath ?? undefined

  const drifted      = _lastScopeId   !== undefined && scopeId   !== _lastScopeId
  const pathMoved    = _lastLocalPath !== undefined
                    && localPath      !== undefined
                    && localPath      !== _lastLocalPath
  const pathMismatch = pathMoved && !drifted

  checks.scopeDrift   = drifted
  checks.pathMismatch = pathMismatch

  _lastScopeId = scopeId
  if (localPath !== undefined) _lastLocalPath = localPath

  if (drifted)      return { label: '🟡◉-D', checks }
  if (pathMismatch) return { label: '🟡◉-P', checks }

  // COLD: low lexical continuity with recent history
  const recent    = snap.history.slice(-6).join(' ')
  const edgeRatio = continuityRatio(tokens, recent)
  checks.continuity = edgeRatio
  if (edgeRatio < CONTINUITY_THRESHOLD && snap.history.length > 0) {
    return { label: '🟡◉-C', checks }
  }

  // AGENCY: operator-frame markers
  const hasAgency = AGENCY_MARKERS_RE.test(snap.prompt)
  checks.agency   = hasAgency
  if (hasAgency) return { label: '🟡◉-A', checks }

  return { label: '🟢●', checks }
}

function tokenize(text) {
  return text.toLowerCase().match(/\b\w+\b/g) ?? []
}

/** djb2 hash — deterministic, collision-resistant for an 8-slot ring */
function slotHash(text) {
  const s = text.trim().toLowerCase()
  let h = 5381
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) + h) ^ s.charCodeAt(i)
    h = h >>> 0
  }
  return h.toString(16)
}

/**
 * Fraction of prompt tokens that appear in recent history.
 * Higher = more continuity with the ongoing conversation.
 */
function continuityRatio(promptTokens, recentHistory) {
  if (promptTokens.length === 0) return 1.0
  const historySet = new Set(tokenize(recentHistory))
  const matches    = promptTokens.filter(t => historySet.has(t)).length
  return matches / promptTokens.length
}
