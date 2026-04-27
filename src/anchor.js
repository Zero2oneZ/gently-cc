// gently-cc anchor <root_cid>
//
// Phase 1 (local): write ~/.gently/anchors.jsonl  { root_cid, scope_id, ts, pubkey }
// Phase 2 (Sui):   POST to trollz-api /scope/anchor with the same payload + tx receipt
//
// The anchor IS the project's identity on chain. One string = the whole state.
// gently-cc clone <root_cid> pulls by this string from any machine.

import { createHash } from 'crypto';
import { readFileSync, writeFileSync, existsSync, mkdirSync, appendFileSync } from 'fs';
import { join } from 'path';

export async function anchor(rootCid) {
  const home     = process.env.HOME;
  const gently   = join(home, '.gently');
  const idFile   = join(gently, 'agent-identity.json');
  const anchFile = join(gently, 'anchors.jsonl');

  if (!existsSync(idFile)) {
    console.error('  No agent identity — run: gently-cc install');
    process.exit(1);
  }

  const id = JSON.parse(readFileSync(idFile, 'utf8'));

  const entry = {
    root_cid:   rootCid,
    scope_id:   id.project_id,
    project:    id.project,
    handle:     id.handle,
    ts:         new Date().toISOString(),
    pubkey:     id.pubkey || null,
    sui_tx:     null,  // Phase 2: filled after on-chain tx
  };

  mkdirSync(gently, { recursive: true });
  appendFileSync(anchFile, JSON.stringify(entry) + '\n', 'utf8');

  console.log('\ngently-cc anchor\n');
  console.log(`  Project  : ${id.project}`);
  console.log(`  Root CID : ${rootCid}`);
  console.log(`  Anchored : ${entry.ts}  (local — Sui tx pending Phase 2)`);
  console.log(`  Log      : ${anchFile}\n`);

  // Phase 2 hook — if TROLLZ_API_URL is set, post to chain
  const apiUrl = process.env.TROLLZ_API_URL;
  if (apiUrl) {
    try {
      const res = await fetch(`${apiUrl}/scope/anchor`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(entry),
      });
      if (res.ok) {
        const data = await res.json();
        entry.sui_tx = data.tx;
        // update last line in anchors.jsonl
        const lines = readFileSync(anchFile, 'utf8').trim().split('\n');
        lines[lines.length - 1] = JSON.stringify(entry);
        writeFileSync(anchFile, lines.join('\n') + '\n', 'utf8');
        console.log(`  Sui tx   : ${data.tx}\n`);
      }
    } catch { /* API not reachable — local anchor still recorded */ }
  }
}
