// gently-cc plan — parse PLAN.md into a content-addressed task DAG
//
// Each task becomes a canonical JSON node:
//   { id, title, phase, requires, ref_by, files, spec_cid }
//
// The spec_cid is SHA-256 of the canonical JSON (excluding spec_cid itself).
// The root CID is SHA-256 of all task CIDs sorted.
//
// Output: plan.dag.json  — the task board as a Merkle DAG.
// This file IS the project — it's tiny, shareable, and the root CID is anchored on-chain.

import { createHash } from 'crypto';
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT  = join(__dirname, '..');
const PLAN_FILE = join(PKG_ROOT, 'PLAN.md');
const DAG_FILE  = join(PKG_ROOT, 'plan.dag.json');

function sha256(s) {
  return 'sha256:' + createHash('sha256').update(s).digest('hex');
}

function parsePlan(md) {
  const tasks = [];
  const lines = md.split('\n');
  let current = null;

  for (const line of lines) {
    // Task header: #### [A.1.1] Title
    const header = line.match(/^####\s+\[([A-Z]\.\d+\.\d+)\]\s+(.+)/);
    if (header) {
      if (current) tasks.push(current);
      current = {
        id:       header[1],
        title:    header[2].trim(),
        phase:    header[1][0],
        requires: [],
        ref_by:   [],
        files:    [],
        body:     [],
      };
      continue;
    }

    if (!current) continue;

    // Requires: [A.1.1], [A.1.2] or (none)
    const req = line.match(/^-\s+\*\*Requires:\*\*\s+(.+)/);
    if (req) {
      const refs = req[1].match(/\[([A-Z]\.\d+\.\d+)\]/g) || [];
      current.requires = refs.map(r => r.slice(1,-1));
      continue;
    }

    // Referenced by:
    const refBy = line.match(/^-\s+\*\*Referenced by:\*\*\s+(.+)/);
    if (refBy) {
      const refs = refBy[1].match(/\[([A-Z]\.\d+\.\d+)\]/g) || [];
      current.ref_by = refs.map(r => r.slice(1,-1));
      continue;
    }

    // Files: `path`, `path2`
    const files = line.match(/^-\s+\*\*Files:\*\*\s+(.+)/);
    if (files) {
      const paths = files[1].match(/`([^`]+)`/g) || [];
      current.files = paths.map(p => p.slice(1,-1));
      continue;
    }

    current.body.push(line);
  }
  if (current) tasks.push(current);
  return tasks;
}

function buildDag(tasks) {
  const nodes = {};

  for (const t of tasks) {
    // Canonical JSON for this task (body stripped to first non-empty line = action summary)
    const actionLine = t.body.find(l => l.trim().startsWith('In ') || l.trim().startsWith('Add ') || l.trim().startsWith('Write ') || l.trim().startsWith('Create ') || l.trim().length > 10);
    const canonical = JSON.stringify({
      id:       t.id,
      title:    t.title,
      phase:    t.phase,
      requires: t.requires.sort(),
      ref_by:   t.ref_by.sort(),
      files:    t.files.sort(),
      action:   actionLine ? actionLine.trim() : '',
    });

    const taskCid = sha256(canonical);

    nodes[t.id] = {
      id:        t.id,
      title:     t.title,
      phase:     t.phase,
      requires:  t.requires,
      ref_by:    t.ref_by,
      files:     t.files,
      task_cid:  taskCid,
      // output_cid is null until the task is completed and its output is anchored
      output_cid: null,
      claimed_by: null,
      completed:  false,
    };
  }

  // Root CID = hash of all task CIDs sorted
  const rootInput = Object.values(nodes)
    .map(n => n.task_cid)
    .sort()
    .join('\n');
  const rootCid = sha256(rootInput);

  return { root_cid: rootCid, tasks: nodes };
}

// ── main ──────────────────────────────────────────────────────
if (!existsSync(PLAN_FILE)) {
  console.error('  PLAN.md not found');
  process.exit(1);
}

const md    = readFileSync(PLAN_FILE, 'utf8');
const tasks = parsePlan(md);
const dag   = buildDag(tasks);

// Preserve existing output_cid / claimed_by / completed if we're re-running
if (existsSync(DAG_FILE)) {
  const existing = JSON.parse(readFileSync(DAG_FILE, 'utf8'));
  for (const [id, node] of Object.entries(existing.tasks || {})) {
    if (dag.tasks[id]) {
      dag.tasks[id].output_cid = node.output_cid;
      dag.tasks[id].claimed_by = node.claimed_by;
      dag.tasks[id].completed  = node.completed;
    }
  }
}

writeFileSync(DAG_FILE, JSON.stringify(dag, null, 2) + '\n', 'utf8');

const total     = Object.keys(dag.tasks).length;
const done      = Object.values(dag.tasks).filter(t => t.completed).length;
const claimed   = Object.values(dag.tasks).filter(t => t.claimed_by && !t.completed).length;
const open      = total - done - claimed;

console.log('\ngently-cc plan\n');
console.log(`  Tasks parsed : ${total}`);
console.log(`  Completed    : ${done}`);
console.log(`  Claimed      : ${claimed}`);
console.log(`  Open         : ${open}`);
console.log(`  Root CID     : ${dag.root_cid}`);
console.log(`  Output       : plan.dag.json\n`);
