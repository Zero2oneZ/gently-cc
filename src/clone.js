// gently-cc clone <root_cid>
//
// Phase 1 (local): look up root_cid in ~/.gently/anchors.jsonl,
//   copy project.codex.json from a known local path, then run materialize.
// Phase 2 (IPFS):  ipfs get <root_cid> → directory tree, then materialize.
//
// The clone command is what makes gently-cc self-distributing:
//   any machine with gently-cc + a root CID can recreate a full working environment.

import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import { spawnSync } from 'child_process';
import { dirname, fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT  = join(__dirname, '..');

export async function clone(rootCid) {
  console.log('\ngently-cc clone\n');
  console.log(`  Root CID : ${rootCid}`);

  // Phase 1: check local anchors
  const home     = process.env.HOME;
  const anchFile = join(home, '.gently', 'anchors.jsonl');

  if (existsSync(anchFile)) {
    const lines   = readFileSync(anchFile, 'utf8').trim().split('\n').filter(Boolean);
    const anchors = lines.map(l => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);
    const match   = anchors.find(a => a.root_cid === rootCid);

    if (match) {
      console.log(`  Found    : local anchor for ${match.project} (${match.ts})`);
      console.log('  Running materialize from local codex...\n');
      const r = spawnSync('node', [join(PKG_ROOT, 'src', 'materialize.js')], { stdio: 'inherit' });
      process.exit(r.status ?? 0);
    }
  }

  // Phase 2: try IPFS
  const ipfs = spawnSync('ipfs', ['--version'], { encoding: 'utf8' });
  if (ipfs.status === 0) {
    console.log('  Pulling from IPFS...');
    const r = spawnSync('ipfs', ['get', rootCid, '-o', '.'], { stdio: 'inherit', cwd: PKG_ROOT });
    if (r.status === 0) {
      console.log('  Running materialize...\n');
      const m = spawnSync('node', [join(PKG_ROOT, 'src', 'materialize.js')], { stdio: 'inherit' });
      process.exit(m.status ?? 0);
    }
  }

  console.log(`\n  Could not resolve ${rootCid}`);
  console.log('  Options:');
  console.log('    - Run on the machine that anchored this CID to use local resolution');
  console.log('    - Install IPFS (https://docs.ipfs.tech/install/) for network resolution');
  console.log('    - Set TROLLZ_API_URL to use trollz chain anchor lookup\n');
  process.exit(1);
}
