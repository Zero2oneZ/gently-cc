// gently-cc claim <task_cid>
//
// Mark a task as claimed (in progress) by this agent.
// Phase 1: writes ~/.gently/claims/<task_cid>.json  (local)
// Phase 2: posts to chain so other agents see it claimed

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT  = join(__dirname, '..');
const DAG_FILE  = join(PKG_ROOT, 'plan.dag.json');

export async function claim(taskCid) {
  const home      = process.env.HOME;
  const idFile    = join(home, '.gently', 'agent-identity.json');
  const claimsDir = join(home, '.gently', 'claims');

  if (!existsSync(idFile)) {
    console.error('  No agent identity — run: gently-cc install');
    process.exit(1);
  }

  const id = JSON.parse(readFileSync(idFile, 'utf8'));

  // Find the task in plan.dag.json
  let task = null;
  if (existsSync(DAG_FILE)) {
    const dag = JSON.parse(readFileSync(DAG_FILE, 'utf8'));
    task = Object.values(dag.tasks).find(t => t.task_cid === taskCid || t.id === taskCid);
    if (task && task.claimed_by && !task.completed) {
      console.log(`\n  Task ${task.id} already claimed by ${task.claimed_by}\n`);
      process.exit(1);
    }
    if (task) {
      task.claimed_by = id.handle;
      writeFileSync(DAG_FILE, JSON.stringify(dag, null, 2) + '\n', 'utf8');
    }
  }

  // Write claim file
  mkdirSync(claimsDir, { recursive: true });
  const claim = {
    task_cid:   taskCid,
    task_id:    task?.id || taskCid,
    task_title: task?.title || '',
    claimed_by: id.handle,
    project:    id.project,
    ts:         new Date().toISOString(),
    output_cid: null,  // filled when task is completed + output anchored
  };
  writeFileSync(join(claimsDir, `${taskCid.replace('sha256:', '').slice(0,16)}.json`), JSON.stringify(claim, null, 2) + '\n', 'utf8');

  console.log('\ngently-cc claim\n');
  if (task) {
    console.log(`  Task     : [${task.id}] ${task.title}`);
    console.log(`  Requires : ${task.requires.join(', ') || '(none)'}`);
    console.log(`  Files    : ${task.files.join(', ') || '(see PLAN.md)'}`);
  } else {
    console.log(`  CID      : ${taskCid}`);
  }
  console.log(`  Claimed  : ${id.handle}  (${claim.ts})\n`);
  console.log('  When done: gently-cc materialize --verify  →  gently-cc anchor <root_cid>\n');
}
