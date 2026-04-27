// gently/src/session.js — shared session state helpers
// (ring persistence, history, identity — same pattern as gently-cc but path-portable)

import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'
import { getState } from '../../src/cold-process.js'

const HOME = homedir()

export function loadIdentity(pkgRoot) {
  try {
    return JSON.parse(readFileSync(join(HOME, '.gently', 'agent-identity.json'), 'utf8'))
  } catch {
    return { name: 'Claudius-GREG', handle: 'Greg', designation: 'Claudius-GREG',
             project: 'gently', project_id: 'gently', project_path: pkgRoot }
  }
}

export function loadDetectorState(pkgRoot) {
  try { return JSON.parse(readFileSync(join(HOME, '.gently', 'detector-state.json'), 'utf8')) }
  catch { return {} }
}

export function saveDetectorState(pkgRoot) {
  try { writeFileSync(join(HOME, '.gently', 'detector-state.json'), JSON.stringify(getState())) }
  catch {}
}

export function loadHistory(pkgRoot) {
  try { return JSON.parse(readFileSync(join(HOME, '.gently', 'prompt-history.json'), 'utf8')) }
  catch { return [] }
}

export function appendHistory(pkgRoot, history, text) {
  const updated = [...history, text].slice(-10)
  try { writeFileSync(join(HOME, '.gently', 'prompt-history.json'), JSON.stringify(updated)) }
  catch {}
  return updated
}
