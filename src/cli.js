#!/usr/bin/env node
// gently-cc CLI entry point
// Usage: gently-cc <command> [options]

import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { existsSync } from 'fs';

const __dirname = dirname(fileURLToPath(import.meta.url));

const COMMANDS = {
  install:  'Install hooks into ~/.claude/settings.json and build binaries',
  verify:   'Verify skill hashes, hook wiring, and binary presence',
  recover:  'Rebuild stack from CLAUDE.jsonl after catastrophic failure',
  status:   'Show current agent identity, theme, and session stats',
  demo:     'Run the multi-session demo (requires tmux)',
  gen:      'Generate Rust code from CODIE symbol expression',
};

async function main() {
  const [,, cmd, ...args] = process.argv;

  if (!cmd || cmd === '--help' || cmd === '-h') {
    console.log('\ngently-cc — GentlyOS Claude Code substrate\n');
    console.log('Usage: gently-cc <command>\n');
    for (const [name, desc] of Object.entries(COMMANDS)) {
      console.log(`  ${name.padEnd(10)} ${desc}`);
    }
    console.log('');
    return;
  }

  switch (cmd) {
    case 'install':
      await import('./install.js');
      break;

    case 'verify':
      await import('./verify.js');
      break;

    case 'recover':
      await import('./recover.js');
      break;

    case 'status': {
      const { readFileSync } = await import('fs');
      const { themeForIdentity, gpsTag } = await import('./theme.js');
      const home = process.env.HOME;
      try {
        const id = JSON.parse(readFileSync(join(home, '.gently', 'agent-identity.json'), 'utf8'));
        const theme = themeForIdentity(id);
        console.log('\n' + gpsTag(theme, id.handle, id.project));
        console.log(`  Name    : ${id.name}\n  Project : ${id.project}\n  Path    : ${id.project_path}\n  Theme   : ${id.theme || theme.name}\n  ID      : ${id.project_id}`);
        try { const s = JSON.parse(readFileSync(join(home, '.gently', 'codie-session.json'), 'utf8')); console.log(`  Session : ${s.turns} turns · ${s.tokens_saved} tokens saved`); } catch {}
        try { const a = readFileSync(join(home, '.gently', 'anomalies.jsonl'), 'utf8').trim().split('\n').filter(Boolean); console.log(`  Anomalies: ${a.length} logged`); } catch {}
        console.log('');
      } catch { console.error('  No agent identity found. Run: gently-cc install\n'); process.exit(1); }
      break;
    }
    case 'demo': {
      const { execSync } = await import('child_process');
      const demoScript = join(__dirname, '..', 'demo', 'run-demo.sh');
      if (!existsSync(demoScript)) { console.error('  demo/ not found in package'); process.exit(1); }
      execSync(`bash "${demoScript}" ${args.includes('--no-tmux') ? '--no-tmux' : ''}`, { stdio: 'inherit' });
      break;
    }
    case 'gen': {
      const { spawnSync } = await import('child_process');
      const genBin = join(__dirname, '..', 'target', 'release', 'gen');
      if (!existsSync(genBin)) { console.error('  gen binary not found — run: cargo build --release -p gently-codegen'); process.exit(1); }
      const expr = args.join(' ');
      if (!expr) { const r = spawnSync(genBin, ['--dag'], { stdio: ['inherit','inherit','inherit'] }); process.exit(r.status ?? 0); }
      const flag = args[0];
      if (flag === '--template' || flag === '-t') { const r = spawnSync(genBin, args, { stdio: 'inherit' }); process.exit(r.status ?? 0); }
      const r = spawnSync(genBin, [expr], { stdio: 'inherit' });
      process.exit(r.status ?? 0);
    }
    default:
      console.error(`  Unknown command: ${cmd}\n  Run: gently-cc --help\n`);
      process.exit(1);
  }
}

main().catch(e => { console.error(e.message); process.exit(1); });