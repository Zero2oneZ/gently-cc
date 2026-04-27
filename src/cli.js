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
        const identity = JSON.parse(readFileSync(join(home, '.gently', 'agent-identity.json'), 'utf8'));
        const theme = themeForIdentity(identity);
        console.log('\n' + gpsTag(theme, identity.handle, identity.project));
        console.log(`  Name    : ${identity.name}`);
        console.log(`  Project : ${identity.project}`);
        console.log(`  Path    : ${identity.project_path}`);
        console.log(`  Theme   : ${identity.theme || theme.name}`);
        console.log(`  ID      : ${identity.project_id}`);
        try {
          const session = JSON.parse(readFileSync(join(home, '.gently', 'codie-session.json'), 'utf8'));
          console.log(`  Session : ${session.turns} turns · ${session.tokens_saved} tokens saved`);
        } catch {}
        try {
          const anomalies = readFileSync(join(home, '.gently', 'anomalies.jsonl'), 'utf8')
            .trim().split('\n').filter(Boolean);
          console.log(`  Anomalies: ${anomalies.length} logged`);
        } catch {}
        console.log('');
      } catch {
        console.error('  No agent identity found. Run: gently-cc install\n');
        process.exit(1);
      }
      break;
    }

    case 'demo': {
      const { execSync } = await import('child_process');
      const demoScript = join(__dirname, '..', 'demo', 'run-demo.sh');
      if (!existsSync(demoScript)) {
        console.error('  demo/ not found in package');
        process.exit(1);
      }
      const flag = args.includes('--no-tmux') ? '--no-tmux' : '';
      execSync(`bash "${demoScript}" ${flag}`, { stdio: 'inherit' });
      break;
    }

    default:
      console.error(`  Unknown command: ${cmd}\n  Run: gently-cc --help\n`);
      process.exit(1);
  }
}

main().catch(e => {
  console.error(e.message);
  process.exit(1);
});
