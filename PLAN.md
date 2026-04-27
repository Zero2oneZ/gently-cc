# GENTLY-CC PRODUCTION PLAN
## Cross-Grid Agentic Execution Map

---

### HOW TO READ THIS DOCUMENT (for agents)

Every task is labeled `[PHASE.STEP.SUBSTEP]` — e.g. `[A.2.3]`.

Each task block contains:
- **Requires:** grid refs that MUST be complete before starting this task
- **Files:** exact paths to create or modify
- **Referenced by:** grid refs that depend on this task completing

When an agent picks up any task, it reads the `Requires:` line first,
verifies those tasks are done (check git log / file existence), then executes.
Cross-referencing grid codes in commit messages keeps the dependency graph alive
across agent context boundaries.

```
Phase A — Security Foundation       (do first, everything depends on this)
Phase B — Real CLI / TUI            (requires A complete)
Phase C — Trollz.fun Integration    (requires A + B.1 complete)
Phase D — Distribution & Scale      (requires A + B + C complete)
```

---

## PHASE A — SECURITY FOUNDATION

---

### [A.1] — Binary Signing with minisign

**Goal:** Every pre-built binary is signed at release time.
Every install verifies the signature before executing anything.
Without this, a compromised GitHub release = code execution in every CC session
on every user's machine.

---

#### [A.1.1] Generate minisign keypair and store public key in repo

- **Requires:** (none)
- **Files:** `src/minisign.pub`, `.github/workflows/release.yml`
- **Action:**
  1. Install minisign locally: `cargo install minisign` or `brew install minisign`
  2. Generate keypair: `minisign -G -p src/minisign.pub -s ~/.gently/minisign.key`
     - Private key stays at `~/.gently/minisign.key` — NEVER commit this
     - Public key (`src/minisign.pub`) goes into the repo
  3. Add `MINISIGN_SECRET_KEY` to GitHub repo secrets
     (Settings → Secrets → Actions → New → paste contents of `~/.gently/minisign.key`)
  4. Commit `src/minisign.pub` to repo
- **Verify:** `cat src/minisign.pub` starts with `untrusted comment:` and `RWSA...`
- **Referenced by:** [A.1.2], [A.1.3], [D.1.3]

---

#### [A.1.2] Sign binaries in release workflow

- **Requires:** [A.1.1]
- **Files:** `.github/workflows/release.yml`
- **Action:**
  After the `Stage release assets` step, add:
  ```yaml
  - name: Sign binaries
    env:
      MINISIGN_SECRET_KEY: ${{ secrets.MINISIGN_SECRET_KEY }}
    run: |
      echo "$MINISIGN_SECRET_KEY" > /tmp/minisign.key
      for f in dist/*; do
        minisign -S -s /tmp/minisign.key -m "$f"
      done
      rm /tmp/minisign.key
  ```
  Update the upload step to include `dist/*.minisig` alongside the binaries.
- **Verify:** Check a release — every binary asset has a matching `.minisig` asset
- **Referenced by:** [A.1.3]

---

#### [A.1.3] Verify signatures at install time

- **Requires:** [A.1.1], [A.1.2]
- **Files:** `src/install.js`
- **Action:**
  In `downloadBinaries()`, after downloading each binary, download its `.minisig`
  and verify before `chmod +x`:
  ```js
  async function verifyBinary(binaryPath, sigPath, pubkeyPath) {
    const { spawnSync } = await import('child_process');
    const r = spawnSync('minisign', ['-V', '-p', pubkeyPath, '-m', binaryPath, '-x', sigPath],
      { encoding: 'utf8' });
    if (r.status !== 0) {
      unlinkSync(binaryPath);
      unlinkSync(sigPath);
      throw new Error(`Signature verification failed for ${binaryPath}`);
    }
  }
  ```
  If `minisign` is not installed, fall back to SHA-256 checksum verification
  (checksums published as `checksums.txt` in the release — add this to release.yml).
  Hard-fail if neither verification path works — do NOT run unverified binaries.
- **Verify:** Corrupt a downloaded binary, run `gently-cc install` — should refuse and print
  `Signature verification failed`, not silently execute the binary
- **Referenced by:** [A.1.4], [B.3.4]

---

#### [A.1.4] Add verification to recover.js

- **Requires:** [A.1.3]
- **Files:** `src/recover.js`
- **Action:**
  In the `recoverBinaries` stage, call the same `verifyBinary()` function
  imported from a shared `src/verify-binary.js` module.
  Extract the verification logic from `install.js` into `src/verify-binary.js`
  so both install and recover share the same code path.
- **Verify:** Run `node src/recover.js` — output shows `✓ codie verified` etc.
- **Referenced by:** [D.1.4]

---

### [A.2] — Ed25519 Identity Keypair

**Goal:** Agent identity is cryptographically bound to the machine.
Currently `~/.gently/agent-identity.json` is plain JSON — anyone can edit it.
Anomaly bus entries are unsigned — a compromised entry could frame the wrong project.
Ed25519 keypair: private key never leaves `~/.gently/`, public key in identity JSON,
all anomaly emissions signed.

---

#### [A.2.1] Generate Ed25519 keypair at install time

- **Requires:** (none — parallel with [A.1.1])
- **Files:** `src/install.js`
- **Action:**
  In `writeAgentIdentity()`, after writing the identity JSON, generate a keypair:
  ```js
  import { generateKeyPairSync } from 'crypto';

  function generateIdentityKeypair(identityFile) {
    const keyFile = identityFile.replace('.json', '.key');
    if (existsSync(keyFile)) return JSON.parse(readFileSync(keyFile.replace('.key', '.pub'), 'utf8'));

    const { privateKey, publicKey } = generateKeyPairSync('ed25519', {
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
      publicKeyEncoding:  { type: 'spki',  format: 'pem' },
    });

    writeFileSync(keyFile, privateKey, { mode: 0o600 }); // owner read-only
    const pub = { algorithm: 'ed25519', key: publicKey, created_at: Date.now() };
    writeFileSync(keyFile.replace('.key', '.pub'), JSON.stringify(pub, null, 2));
    return pub;
  }
  ```
  Store public key PEM in `agent-identity.json` under `public_key`.
  Private key at `~/.gently/agent-identity.key` with `chmod 600`.
- **Verify:** `ls -la ~/.gently/agent-identity.key` → permissions `-rw-------`
- **Referenced by:** [A.2.2], [A.2.3], [A.2.4], [C.1.3]

---

#### [A.2.2] Sign function — shared crypto module

- **Requires:** [A.2.1]
- **Files:** `src/identity-crypto.js` (new file)
- **Action:**
  Create `src/identity-crypto.js` exporting:
  ```js
  export function signPayload(payload, privateKeyPath) {
    // reads privateKeyPath, signs JSON.stringify(payload) with Ed25519
    // returns base64url signature string
  }

  export function verifyPayload(payload, signature, publicKeyPem) {
    // verifies signature against payload
    // returns boolean
  }

  export function loadPrivateKey() {
    // reads ~/.gently/agent-identity.key
    // throws if not found or wrong permissions
  }
  ```
  All signing operations in the codebase import from here — single implementation,
  no drift between install/hooks/recover.
- **Verify:** `node --input-type=module -e "import {signPayload} from './src/identity-crypto.js'; console.log(signPayload({test:1}))"` → base64url string
- **Referenced by:** [A.2.3], [A.2.4], [C.2.1]

---

#### [A.2.3] Sign anomaly bus entries

- **Requires:** [A.2.1], [A.2.2]
- **Files:** `hooks/codie-compress.js`
- **Action:**
  Update `emitAnomaly()` to sign each entry before writing:
  ```js
  import { signPayload, loadPrivateKey } from '../src/identity-crypto.js';

  function emitAnomaly(label, reason, identity, text) {
    const entry = { ts: Date.now(), label, reason, project: identity.project,
                    handle: identity.handle, session: process.env.CLAUDE_SESSION_ID,
                    preview: text.slice(0, 80).replace(/\n/g, ' ') };
    try {
      const key = loadPrivateKey();
      entry.sig = signPayload(entry, key);
    } catch {} // sign if possible, emit unsigned if key unavailable
    writeFileSync(ANOMALY_PATH, JSON.stringify(entry) + '\n', { flag: 'a' });
  }
  ```
  Unsigned entries are still accepted locally (backward compat) but the C.2 remote
  flush will reject unsigned entries.
- **Verify:** Run demo, `cat ~/.gently/anomalies.jsonl | jq .sig` → non-null base64url strings
- **Referenced by:** [C.2.1], [C.2.4]

---

#### [A.2.4] Verify identity on session start

- **Requires:** [A.2.1], [A.2.2]
- **Files:** `hooks/session-start.js`
- **Action:**
  After loading identity, verify the keypair is consistent
  (private key exists + signs a nonce + public key in identity.json verifies it):
  ```js
  function verifyIdentityKeypair(identity) {
    try {
      const nonce = crypto.randomUUID();
      const sig = signPayload({ nonce }, loadPrivateKey());
      return verifyPayload({ nonce }, sig, identity.public_key.key);
    } catch { return false; }
  }
  ```
  If verification fails: emit warning to stderr, continue session (don't block work),
  but set `identity.key_verified = false` so downstream checks can gate on it.
- **Verify:** Delete `~/.gently/agent-identity.key`, start session → banner shows
  `⚠ identity keypair missing — run: gently-cc install`
- **Referenced by:** [C.1.5]

---

### [A.3] — Real GRIM Hash Computation

**Goal:** Replace every `sha256:{grim:name}` placeholder with actual SHA-256 hashes.
The "hashes are ground truth" invariant currently isn't true — the hashes were never run.

---

#### [A.3.1] Write grim-scanner.js

- **Requires:** (none — parallel with [A.1], [A.2])
- **Files:** `src/grim-scanner.js` (new file)
- **Action:**
  Create a scanner that:
  1. Reads `CLAUDE.jsonl` to get the list of skill files
  2. Computes `sha256:` + SHA-256 hex digest of each file's UTF-8 content
  3. Patches `CLAUDE.md` skill manifest table — replaces `sha256:{grim:name}`
     with the real hash for each skill
  4. Patches each `SKILL.md` and `SKILL.jsonl` with their own `grim_hash` field
  5. Recomputes and patches `manifest_root` in `CLAUDE.md`
  6. Writes a `grim.json` summary: `{ scanned_at, files: [{path, hash}] }`
  ```js
  // Usage: node src/grim-scanner.js [--verify]
  // --verify: check existing hashes match, exit 1 on mismatch (for CI)
  // (no flag): compute and patch
  ```
- **Verify:** `node src/grim-scanner.js` → no `sha256:{grim:` patterns remain in CLAUDE.md.
  `node src/grim-scanner.js --verify` exits 0.
- **Referenced by:** [A.3.2], [A.3.3], [A.3.4]

---

#### [A.3.2] Run the scanner — replace all placeholders

- **Requires:** [A.3.1]
- **Files:** `CLAUDE.md`, `skills/*/SKILL.md`, `skills/*/SKILL.jsonl`
- **Action:**
  1. `node src/grim-scanner.js` — patches all files
  2. `git diff --stat` — confirm only hash fields changed, no logic changes
  3. Commit: `git commit -m "[A.3.2] compute real GRIM hashes — replace all placeholders"`
  4. All future changes to skill files MUST re-run the scanner and commit the updated hashes
- **Verify:** `grep -r 'sha256:{grim:' CLAUDE.md skills/` → no output
- **Referenced by:** [A.3.3], [A.3.4]

---

#### [A.3.3] Verify GRIM hashes on session start

- **Requires:** [A.3.1], [A.3.2]
- **Files:** `hooks/session-start.js`
- **Action:**
  Add a fast GRIM check at session start (non-blocking — run async, emit warning if mismatch):
  ```js
  async function checkGrimHashes() {
    const { spawnSync } = await import('child_process');
    const r = spawnSync('node', [join(PKG_ROOT, 'src/grim-scanner.js'), '--verify'],
      { encoding: 'utf8', timeout: 3000 });
    if (r.status !== 0) {
      process.stderr.write(`⚠ GRIM: skill file modified since last scan — run: gently-cc verify\n`);
    }
  }
  // fire-and-forget, don't await in the critical path
  checkGrimHashes().catch(() => {});
  ```
- **Verify:** Manually edit one character in `skills/codie/SKILL.md`, start a session →
  banner includes `⚠ GRIM: skill file modified` warning
- **Referenced by:** [D.1.4]

---

#### [A.3.4] Add GRIM verify to CI

- **Requires:** [A.3.1], [A.3.2]
- **Files:** `.github/workflows/ci.yml`
- **Action:**
  Add a step to the `node` job:
  ```yaml
  - name: GRIM hash verification
    run: node src/grim-scanner.js --verify
  ```
  This means any PR that modifies a skill file without re-running the scanner
  will fail CI — enforcing the hash invariant automatically.
- **Verify:** Open a PR that edits `skills/codie/SKILL.md` without re-running scanner →
  CI fails at GRIM step
- **Referenced by:** [D.1.2]

---

### [A.4] — Hook Input Sanitization

**Goal:** Hooks accept JSON from Claude Code with user-supplied text.
That text gets passed to Rust binaries as arguments. Prevent command injection.

---

#### [A.4.1] Sanitize barf/codec arguments

- **Requires:** (none)
- **Files:** `hooks/codie-compress.js`, `hooks/response-extract.js`
- **Action:**
  Create `src/sanitize.js`:
  ```js
  export function sanitizeToken(token) {
    // allow: letters, numbers, hyphens, underscores, dots, colons, slashes
    // reject: shell metacharacters, null bytes, control chars
    return typeof token === 'string'
      && token.length >= 2
      && token.length <= 120
      && /^[\w\-.:\/]+$/.test(token);
  }
  ```
  In `codie-compress.js` generative layer, wrap `barfSilent` calls:
  ```js
  if (sanitizeToken(token)) {
    barfSilent(['insert', token, '--tokens', String(tokens)]);
  }
  ```
  In `response-extract.js`, same guard on every extracted candidate.
- **Verify:** Pass a prompt containing `; rm -rf /tmp/test` as a token — verify
  the barf call is skipped, nothing is executed
- **Referenced by:** [B.2.6]

---

## PHASE B — REAL CLI / TUI

*Requires Phase A complete before starting.*

---

### [B.1] — Ratatui TUI Dashboard

**Goal:** `gently-cc dashboard` opens a live terminal UI showing all session activity,
anomaly stream, and CODIE stats. This is what makes it a real CLI tool,
not just a set of hooks.

---

#### [B.1.1] Create gently-tui Rust crate

- **Requires:** [A.1.3] (binaries verified), [A.3.2] (GRIM hashes real)
- **Files:** `gently-tui/Cargo.toml`, `gently-tui/src/main.rs`, `Cargo.toml` (workspace)
- **Action:**
  Add to workspace `Cargo.toml`:
  ```toml
  [workspace]
  members = ["codie", "bs-artisan", "gently-codec", "gently-tui"]
  ```
  `gently-tui/Cargo.toml` dependencies:
  ```toml
  [dependencies]
  ratatui = "0.28"
  crossterm = "0.28"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  tokio = { version = "1", features = ["full"] }
  notify = "6"     # file watch for live anomaly tail
  ```
  `gently-tui/src/main.rs`: skeleton app with crossterm backend, 4-panel layout:
  top-left (sessions), top-right (CODIE stats), bottom-left (anomaly stream),
  bottom-right (foam stats). Stub panels with placeholder text first.
- **Verify:** `cargo build -p gently-tui` compiles clean. `./target/release/gently-tui`
  opens a terminal window with 4 panels (even if empty).
- **Referenced by:** [B.1.2], [B.1.3], [B.1.4], [B.1.5], [B.1.6]

---

#### [B.1.2] Sessions panel

- **Requires:** [B.1.1]
- **Files:** `gently-tui/src/panels/sessions.rs`
- **Action:**
  Read `~/.gently/sessions/*.json` (created by `session-start.js` — see [A.2.4] for format).
  Display as a table: session_id (truncated 8 chars), project, started_at (relative: "2m ago"),
  crystal_parent (✓ or —), codie_turns.
  Color each row with the project's theme color (load from identity JSON, map to ratatui Color).
  Refresh every 2 seconds via `tokio::time::interval`.
  Clicking/selecting a row shows session detail in a popup.
- **Verify:** Start a CC session in another terminal, `gently-tui` sessions panel
  updates within 2s showing the new session row
- **Referenced by:** [B.1.6], [C.3.4]

---

#### [B.1.3] Anomaly stream panel

- **Requires:** [B.1.1], [A.2.3]
- **Files:** `gently-tui/src/panels/anomalies.rs`
- **Action:**
  Use the `notify` crate to watch `~/.gently/anomalies.jsonl` for append events.
  Parse each new line as an anomaly entry (see [A.2.3] for schema).
  Render as a scrolling log with color coding:
  - `🟢●` → dim green
  - `🟡◉-D` → yellow (duplicate)
  - `🟡◉-C` → yellow (cluster)
  - `🔴◉-S` → red bold (foreign scope)
  Show: timestamp, label, project (in that project's theme color), reason preview.
  `s` key to toggle signature verification display (ref [A.2.3]).
- **Verify:** Run `demo/run-demo.sh --no-tmux` while `gently-tui` is open —
  watch anomaly panel fill in real time as the demo fires events
- **Referenced by:** [B.1.6], [C.2.3]

---

#### [B.1.4] CODIE stats panel

- **Requires:** [B.1.1]
- **Files:** `gently-tui/src/panels/codie.rs`
- **Action:**
  Read `~/.gently/codie-session.json` (ref session-start.js — written each session).
  Display:
  - Current session: turns, tokens_saved, avg compression %
  - Lifetime: sum across all session JSON files in `~/.gently/sessions/`
  - Sparkline: last 20 turns' compression percentages (use ratatui Sparkline widget)
  - Token savings in dollar terms (estimated at $0.000015/token for Sonnet)
- **Verify:** Send a few prompts through CC, watch the sparkline update and totals increment
- **Referenced by:** [B.1.6], [C.3.1]

---

#### [B.1.5] Foam stats panel

- **Requires:** [B.1.1]
- **Files:** `gently-tui/src/panels/foam.rs`
- **Action:**
  Call `barf stats` (binary from [A.1.3]) and parse JSON output.
  Display:
  - Total tori count
  - Top 10 tori by weight (bar chart, colored by weight)
  - Recent inserts (last 5 tokens seeded via generative layer — ref [A.4.1])
  - Co-occurrence cluster count
  `r` key to trigger `barf query <selected_torus>` and show related tori.
- **Verify:** After a few sessions, foam panel shows non-zero torus count
  with recognizable project terms near the top
- **Referenced by:** [B.1.6]

---

#### [B.1.6] Wire dashboard command to CLI

- **Requires:** [B.1.1], [B.1.2], [B.1.3], [B.1.4], [B.1.5]
- **Files:** `src/cli.js`, `.github/workflows/release.yml`
- **Action:**
  In `src/cli.js` `dashboard` command:
  ```js
  case 'dashboard': {
    const tui = join(PKG_ROOT, 'target/release/gently-tui');
    if (!existsSync(tui)) {
      console.error('  gently-tui binary not found — run: cargo build --release -p gently-tui');
      process.exit(1);
    }
    spawnSync(tui, [], { stdio: 'inherit' });
    break;
  }
  ```
  Add `gently-tui` to the release build matrix in `.github/workflows/release.yml`
  (add to each build step alongside codie/barf/codec, and to dist/ staging).
  Add `gently-tui` to the binary verification list in `src/install.js` (ref [A.1.3]).
- **Verify:** `gently-cc dashboard` opens the 4-panel TUI
- **Referenced by:** [C.3.4]

---

### [B.2] — MCP Server Hardened

**Goal:** The current `src/mcp-server.js` is a stub — 4 tools that spawn subprocesses
with minimal error handling and no capability gating. Needs to be production-grade.

---

#### [B.2.1] Request queuing and connection lifecycle

- **Requires:** [A.4.1]
- **Files:** `src/mcp-server.js`
- **Action:**
  Replace the current single-pass stdin reader with a proper line-delimited
  JSON-RPC loop using `readline`:
  ```js
  import { createInterface } from 'readline';
  const rl = createInterface({ input: process.stdin, terminal: false });
  rl.on('line', async (line) => {
    const req = JSON.parse(line);
    const res = await dispatch(req);
    process.stdout.write(JSON.stringify(res) + '\n');
  });
  ```
  Add request ID tracking, proper error response format
  (`{jsonrpc: "2.0", id, error: {code, message}}`), and timeout per tool call (5s).
- **Verify:** Send 10 concurrent requests via `printf '...\n%.0s' {1..10} | node src/mcp-server.js`
  — all 10 get responses, none dropped
- **Referenced by:** [B.2.2], [B.2.3], [B.2.4], [B.2.5]

---

#### [B.2.2] Capability check middleware

- **Requires:** [B.2.1], [A.2.1]
- **Files:** `src/mcp-server.js`, `src/capabilities.js` (new)
- **Action:**
  Create `src/capabilities.js` — a capability registry loaded from `CLAUDE.jsonl`:
  ```js
  export const TOOL_CAPS = {
    'codec_pin_define':    'codec:write',
    'codec_pin_list':      'codec:read',
    'codec_pin_conflicts': 'codec:read',
    'barf_query':          'foam:read',
    'barf_insert':         'foam:write',
    'anomaly_list':        'anomaly:read',
    'anomaly_export':      'anomaly:export',
  };
  export function checkCap(toolName, identity) {
    const required = TOOL_CAPS[toolName];
    if (!required) return true; // public tool
    return (identity.capabilities || []).includes(required);
  }
  ```
  In `dispatch()`, call `checkCap()` before every tool execution.
  Return `{code: -32603, message: 'capability required: codec:write'}` on failure.
  Default capabilities (no AgentNFT yet) include `codec:read` and `foam:read` only.
  Write capability earns via [C.1.3].
- **Verify:** Call `codec_pin_define` without write capability →
  error response with capability message
- **Referenced by:** [B.2.3], [B.2.4], [C.1.5]

---

#### [B.2.3] Full codec tool surface

- **Requires:** [B.2.1], [B.2.2]
- **Files:** `src/mcp-server.js`
- **Action:**
  Implement all codec subcommands as MCP tools:
  ```
  codec_pin_ref      {name}           → codec ref <name>
  codec_pin_list     {}               → codec list
  codec_pin_context  {}               → codec context
  codec_pin_define   {name, label}    → codec define <name> <label>    [cap: codec:write]
  codec_pin_crystal  {}               → codec crystal                  [cap: codec:write]
  codec_pin_load     {prefix}         → codec load <prefix>
  codec_pin_conflicts {}              → codec conflicts
  codec_pin_promote  {name, scope}    → codec promote <name> <scope>   [cap: codec:write]
  ```
  Each tool: validate inputs against [A.4.1] sanitize rules, spawn binary,
  parse output, return structured JSON.
- **Verify:** `echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"codec_pin_list","arguments":{}}}' | node src/mcp-server.js` → returns pin list
- **Referenced by:** [C.1.4]

---

#### [B.2.4] Full barf tool surface

- **Requires:** [B.2.1], [B.2.2]
- **Files:** `src/mcp-server.js`
- **Action:**
  ```
  barf_query    {term}           → barf query <term>
  barf_insert   {term, tokens}   → barf insert <term> --tokens <n>  [cap: foam:write]
  barf_stats    {}               → barf stats
  barf_cooccur  {a, b}           → barf cooccur <a> <b>             [cap: foam:write]
  barf_forget   {term}           → barf forget <term>               [cap: foam:write]
  ```
  Validate `tokens` is integer 1–64. Validate `term` with `sanitizeToken()` (ref [A.4.1]).
- **Verify:** Query a term that exists in foam after running the demo → returns non-empty result
- **Referenced by:** [C.2.5]

---

#### [B.2.5] Anomaly tool surface

- **Requires:** [B.2.1], [B.2.2], [A.2.3]
- **Files:** `src/mcp-server.js`
- **Action:**
  ```
  anomaly_list    {limit?, label?}  → reads ~/.gently/anomalies.jsonl, filters, returns array
  anomaly_export  {format}          → json | csv | markdown summary
  anomaly_verify  {index}           → verifies Ed25519 signature on entry N (ref [A.2.3])
  anomaly_clear   {}                → truncates file  [cap: anomaly:export]
  ```
- **Verify:** List anomalies after demo run → returns the 3 logged entries with sig fields
- **Referenced by:** [C.2.3]

---

### [B.3] — CLI Polish

---

#### [B.3.1] `gently-cc theme` — interactive theme preview

- **Requires:** (none — parallel with B.1)
- **Files:** `src/cli.js`
- **Action:**
  Print all 16 themes as a grid, each showing:
  - The ASCII art block character colored in that theme
  - Theme name and index
  - Sample GPS tag line
  Prompt: `Select theme [0-15] or Enter to keep current:` → write selection to
  `agent-identity.json` as `theme` field.
  ```js
  case 'theme': {
    const { THEMES, asciiArt, gpsTag } = await import('./theme.js');
    THEMES.forEach((t, i) => {
      const art = asciiArt(t)[0];
      console.log(`  [${String(i).padStart(2)}] ${art}  ${t.name.padEnd(8)} ${gpsTag(t, identity.handle, identity.project)}`);
    });
    // readline prompt → update identity.json
    break;
  }
  ```
- **Verify:** `gently-cc theme` → 16 colored rows, selection updates identity.json
- **Referenced by:** (cosmetic — no functional dependents)

---

#### [B.3.2] `gently-cc sessions` — session inspector

- **Requires:** (none)
- **Files:** `src/cli.js`
- **Action:**
  List all sessions from `~/.gently/sessions/*.json`:
  ```
  gently-cc sessions           → table: id, project, started_at, turns, saved
  gently-cc sessions <id>      → full JSON dump of session manifest
  gently-cc sessions --prune   → delete sessions older than 30 days
  ```
- **Verify:** After a few CC sessions, `gently-cc sessions` shows them with correct stats
- **Referenced by:** [C.3.1]

---

#### [B.3.3] `gently-cc anomalies` — anomaly inspector

- **Requires:** [A.2.3]
- **Files:** `src/cli.js`
- **Action:**
  ```
  gently-cc anomalies           → tail -style, last 20 entries colored by label
  gently-cc anomalies --label D → filter by label type (D/C/S)
  gently-cc anomalies --since 1h → filter by time
  gently-cc anomalies --verify  → verify Ed25519 signatures on all entries (ref [A.2.3])
  gently-cc anomalies --export  → write markdown report to stdout
  ```
- **Verify:** `gently-cc anomalies --verify` after demo run → all 3 entries show `✓ sig valid`
- **Referenced by:** (no functional dependents — UX feature)

---

#### [B.3.4] `gently-cc update` — self-update

- **Requires:** [A.1.3]
- **Files:** `src/cli.js`
- **Action:**
  1. Fetch latest version from GitHub API:
     `https://api.github.com/repos/Zero2oneZ/gently-cc/releases/latest`
  2. Compare to current `package.json` version
  3. If newer: download all 3 binaries for current platform (ref [A.1.3] download logic)
  4. Verify signatures (ref [A.1.3])
  5. Replace binaries in `target/release/`
  6. Print changelog (release notes from GitHub API response)
  Rate limit: check at most once per 24h (cache check time in `~/.gently/last-update-check`)
- **Verify:** Manually bump version in a test release, run `gently-cc update` → downloads and verifies
- **Referenced by:** (no functional dependents)

---

#### [B.3.5] `gently-cc uninstall` — clean removal

- **Requires:** (none)
- **Files:** `src/cli.js`
- **Action:**
  ```
  gently-cc uninstall           → interactive: confirm before each step
  gently-cc uninstall --force   → no prompts
  ```
  Steps:
  1. Remove gently-cc hooks from `~/.claude/settings.json`
  2. Optionally remove `~/.gently/` (prompt separately — user may want to keep history)
  3. Print: `gently-cc removed. Run 'npm uninstall -g gently-cc' to remove the package.`
- **Verify:** After uninstall, `~/.claude/settings.json` has no gently-cc entries.
  Starting a new CC session shows no CODIE banner.
- **Referenced by:** (no functional dependents)

---

#### [B.3.6] Shell completions

- **Requires:** [B.3.1], [B.3.2], [B.3.3], [B.3.4], [B.3.5]
- **Files:** `completions/gently-cc.bash`, `completions/gently-cc.zsh`, `completions/gently-cc.fish`
- **Action:**
  Generate completion scripts for all CLI commands and subcommands.
  Add to `src/cli.js`:
  ```
  gently-cc completions bash    → prints bash completion script
  gently-cc completions zsh     → prints zsh completion script
  gently-cc completions fish    → prints fish completion script
  ```
  Install instructions: `gently-cc completions bash >> ~/.bashrc`
- **Verify:** Tab-complete `gently-cc <TAB>` → shows all commands
- **Referenced by:** (no functional dependents — polish)

---

## PHASE C — TROLLZ.FUN INTEGRATION

*Requires Phase A complete. Requires [B.1.1] for dashboard SYNTH panel.*

---

### [C.1] — AgentNFT Registration

**Goal:** Register gently-cc as an on-chain agent on Sui.
This binds the capability_hash to the GRIM manifest root (ref [A.3.2]),
making the agent's permissions cryptographically immutable.

---

#### [C.1.1] Add `gently-cc register` command

- **Requires:** [A.2.1], [A.3.2]
- **Files:** `src/cli.js`, `src/register.js` (new)
- **Action:**
  Create `src/register.js`:
  ```js
  // Calls POST https://trollz.fun/api/agents/register
  // Body: { handle, project, public_key, manifest_root, grim_hash }
  // manifest_root from CLAUDE.md (ref [A.3.2])
  // public_key from identity.json (ref [A.2.1])
  // Response: { nft_address, capability_hash, tx_digest }
  // Saves to agent-identity.json: nft_address, capability_hash, registered_at
  ```
  The trollz-api endpoint mints an AgentNFT on Sui testnet with:
  - `capability_hash` = SHA-256 of the capability set
  - `manifest_root` = the GRIM manifest root (ties NFT to this exact codebase state)
- **Verify:** `gently-cc register` → prints `✓ AgentNFT minted: 0x...` and updates identity.json
- **Referenced by:** [C.1.2], [C.1.3], [C.1.4], [C.1.5]

---

#### [C.1.2] Capability hash from GRIM manifest root

- **Requires:** [C.1.1], [A.3.2]
- **Files:** `src/register.js`
- **Action:**
  The `capability_hash` sent to trollz-api is:
  ```js
  SHA-256(manifest_root + '::' + JSON.stringify(sorted_capabilities))
  ```
  Where `sorted_capabilities` is the sorted array of capability strings from `CLAUDE.jsonl`
  (the `invariant` record's `values` field — ref CLAUDE.jsonl `type: invariant`).
  This means changing the manifest (adding/removing skill files) produces a different
  capability_hash, which requires re-registration — enforcing the immutability invariant.
- **Verify:** Change one character in any skill file (without re-running grim-scanner),
  recompute capability_hash → different from registered hash → registration check fails
- **Referenced by:** [C.1.3], [C.1.4]

---

#### [C.1.3] Store NFT address and bind identity keypair

- **Requires:** [C.1.1], [C.1.2], [A.2.1]
- **Files:** `src/register.js`, `~/.gently/agent-identity.json`
- **Action:**
  After successful registration, update `agent-identity.json`:
  ```json
  {
    "nft_address": "0x...",
    "capability_hash": "0x...",
    "registered_at": 1234567890000,
    "capabilities": ["codec:read", "codec:write", "foam:read", "foam:write", "anomaly:read"]
  }
  ```
  The `capabilities` array is returned by trollz-api from the AgentNFT's capability_set.
  This is what [B.2.2]'s `checkCap()` reads to gate MCP tool access.
- **Verify:** After register, `gently-cc status` shows `NFT: 0x...` and capabilities list.
  `codec_pin_define` MCP call (which requires `codec:write`) now succeeds.
- **Referenced by:** [C.1.4], [C.1.5], [B.2.2]

---

#### [C.1.4] Capability verification in session-start

- **Requires:** [C.1.3], [A.2.4]
- **Files:** `hooks/session-start.js`
- **Action:**
  After identity verification (ref [A.2.4]), check NFT capabilities against
  what the session needs. Display in banner:
  ```
  [CRIMSON] Claudius-GREG
  gently-cc · CODIE·BARF·CODEC  ~/Projects/gently-cc
  NFT: 0x3f91... ✓ · caps: codec:rw foam:rw anomaly:r
  ```
  If no NFT (unregistered): show `(unregistered — run: gently-cc register)` in dim.
  Do NOT block the session — just inform.
- **Verify:** After registration, banner shows NFT address + capability set
- **Referenced by:** (no blocking dependents — informational)

---

### [C.2] — Anomaly Bus → ops_events

**Goal:** Local `anomalies.jsonl` is invisible to other machines and future sessions.
Route signed anomalies to the trollz.fun ops_events bus so the Organizer view
works across the whole fleet, not just one terminal.

---

#### [C.2.1] HTTP flush function

- **Requires:** [A.2.3], [C.1.3]
- **Files:** `hooks/codie-compress.js`, `src/ops-client.js` (new)
- **Action:**
  Create `src/ops-client.js`:
  ```js
  export async function flushAnomaly(entry, identity) {
    // Only if: identity.nft_address exists (registered) AND telemetry not opted out
    if (!identity.nft_address || identity.telemetry === false) return;
    // POST to trollz.fun/api/ops/events
    // Body: { ...entry, agent_nft: identity.nft_address }
    // Auth: Authorization: Bearer <sign nonce with identity keypair>
    // Fire-and-forget with 2s timeout — never block the hook
  }
  ```
  In `emitAnomaly()` (ref [A.2.3]), call `flushAnomaly()` after local write.
  The signed entry (ref [A.2.3]) is what gets sent — server can verify sig against
  the public key registered with the AgentNFT.
- **Verify:** Register ([C.1.1]), then trigger an anomaly → check trollz.fun ops stream
  via `trollz_ops_poll` MCP tool shows the entry
- **Referenced by:** [C.2.2], [C.2.3]

---

#### [C.2.2] Batch mode + session-end flush

- **Requires:** [C.2.1]
- **Files:** `hooks/codie-compress.js`, `hooks/stop.js`
- **Action:**
  Buffer anomalies in `~/.gently/anomaly-buffer.jsonl` during session.
  In `hooks/stop.js` (crystal seal hook), flush the buffer:
  ```js
  async function flushAnomalyBuffer() {
    const bufPath = join(HOME_GENTLY, 'anomaly-buffer.jsonl');
    if (!existsSync(bufPath)) return;
    const entries = readFileSync(bufPath, 'utf8').trim().split('\n')
      .filter(Boolean).map(l => JSON.parse(l));
    for (const entry of entries) await flushAnomaly(entry, identity);
    writeFileSync(bufPath, ''); // clear buffer
  }
  ```
  This reduces HTTP calls from N per session to 1 batch at session end.
- **Verify:** Generate multiple anomalies in one session → only one HTTP call fires at stop
- **Referenced by:** [C.2.3]

---

#### [C.2.3] Privacy mode

- **Requires:** [C.2.1]
- **Files:** `src/ops-client.js`, `src/cli.js`
- **Action:**
  Add `gently-cc privacy` command that sets `identity.telemetry = false`:
  - Full opt-out: nothing sent to trollz.fun
  - Privacy mode: project names hashed before sending, previews stripped
  Default: opt-in for registered agents (refs [C.1.3]), opt-out for unregistered.
  Privacy setting stored in `~/.gently/agent-identity.json` as `telemetry: true|false|"hashed"`.
  In `flushAnomaly()`:
  ```js
  if (identity.telemetry === 'hashed') {
    entry = { ...entry, project: SHA256(entry.project).slice(0,8), preview: null };
  }
  ```
- **Verify:** Set `telemetry: "hashed"`, trigger anomaly → ops stream shows hashed project, no preview
- **Referenced by:** (no blocking dependents — compliance feature)

---

### [C.3] — SYNTH Accrual

**Goal:** CODIE compression = measurable compute contribution = SYNTH earned.
Every session's token savings map to the `compute` SYNTH axis on trollz.fun.

---

#### [C.3.1] Calculate SYNTH per session

- **Requires:** [C.1.3], [B.3.2]
- **Files:** `hooks/stop.js`, `src/synth.js` (new)
- **Action:**
  Create `src/synth.js`:
  ```js
  const SYNTH_USD_RATE = 1000; // 1000 SYNTH per USD of compute saved
  const SONNET_COST_PER_TOKEN = 0.000015; // input token cost

  export function calculateSessionSynth(session) {
    // session from ~/.gently/sessions/<id>.json (ref [B.3.2])
    const usdSaved = session.codie_stats.tokens_saved * SONNET_COST_PER_TOKEN;
    const rarity = 1.0; // common — could tier up with usage patterns
    const quality = 0.4 + (session.codie_stats.turns > 10 ? 0.3 : 0);
    return usdSaved * rarity * Math.log10(session.codie_stats.turns + 1) * quality * SYNTH_USD_RATE;
  }
  ```
  In `hooks/stop.js`, after crystal seal, compute and record SYNTH earned:
  ```js
  const synth = calculateSessionSynth(manifest);
  manifest.synth_earned = synth;
  writeFileSync(sessionPath, JSON.stringify(manifest, null, 2));
  ```
- **Verify:** After a 10-turn session, session JSON has `synth_earned > 0`
- **Referenced by:** [C.3.2], [C.3.3]

---

#### [C.3.2] POST SYNTH accrual to trollz-api

- **Requires:** [C.3.1], [C.1.3]
- **Files:** `hooks/stop.js`, `src/ops-client.js`
- **Action:**
  Add to `src/ops-client.js`:
  ```js
  export async function accrueSynth(sessionManifest, identity) {
    if (!identity.nft_address) return; // must be registered
    // POST trollz.fun/api/synth/accrue
    // Body: { agent_nft, session_id, synth_amount, axis: 'compute',
    //         proof: { tokens_saved, turns, manifest_root } }
    // Auth: signed with identity keypair (ref [A.2.2])
    // Response: { tx_digest, balance }
    // Store tx_digest in session JSON
  }
  ```
  Called at session end alongside anomaly buffer flush (ref [C.2.2]).
- **Verify:** After a session, trollz.fun SYNTH balance increases.
  Session JSON has `synth_tx` field.
- **Referenced by:** [C.3.3]

---

#### [C.3.3] `gently-cc earnings` command

- **Requires:** [C.3.1], [C.3.2]
- **Files:** `src/cli.js`
- **Action:**
  ```
  gently-cc earnings            → lifetime SYNTH earned, current balance
  gently-cc earnings --sessions → per-session breakdown table
  gently-cc earnings --export   → CSV for accounting
  ```
  Lifetime SYNTH: sum of `synth_earned` across all session JSONs in `~/.gently/sessions/`.
  Current balance: GET `trollz.fun/api/synth/balance?agent_nft=<address>`.
- **Verify:** `gently-cc earnings` shows non-zero SYNTH after a registered session
- **Referenced by:** [C.3.4]

---

#### [C.3.4] SYNTH panel in TUI dashboard

- **Requires:** [C.3.3], [B.1.6]
- **Files:** `gently-tui/src/panels/synth.rs`
- **Action:**
  Replace or extend the foam stats panel (ref [B.1.5]) with a combined
  Foam + SYNTH panel:
  - Foam torus count + top tori (from [B.1.5])
  - Divider
  - Session SYNTH earned (from `~/.gently/sessions/current.json`)
  - Lifetime total
  - On-chain balance (cached, refresh on `r` key)
- **Verify:** Dashboard shows SYNTH balance updating after each session
- **Referenced by:** (final integration milestone)

---

## PHASE D — DISTRIBUTION & SCALE

*Requires Phase A, B, C complete.*

---

### [D.1] — npm Publish Automation

---

#### [D.1.1] Add NPM_TOKEN to GitHub secrets

- **Requires:** (none — manual prerequisite)
- **Files:** (no code changes — GitHub UI action)
- **Action:**
  1. Log in to npmjs.com → Access Tokens → Generate New Token → Type: Automation
  2. Copy token
  3. GitHub repo → Settings → Secrets and Variables → Actions → New repository secret
     Name: `NPM_TOKEN`, Value: paste token
  4. Confirm by checking `.github/workflows/release.yml` npm-publish job references
     `secrets.NPM_TOKEN` (it already does from the initial commit)
- **Verify:** Push a new tag → GitHub Actions → release workflow → npm-publish job succeeds
- **Referenced by:** [D.1.2]

---

#### [D.1.2] Verify end-to-end release pipeline

- **Requires:** [D.1.1], [A.1.2], [A.3.4]
- **Files:** `.github/workflows/release.yml`
- **Action:**
  Tag `v1.0.1` (or next version), push, watch the Actions run:
  1. All 4 platform builds succeed
  2. Binaries + `.minisig` files attached to release (ref [A.1.2])
  3. `checksums.txt` attached (add this step to release.yml)
  4. npm-publish job runs after all build jobs
  5. `npm install -g gently-cc@1.0.1` on a clean machine works end to end
  If any step fails: fix root cause, increment patch version, re-tag.
- **Verify:** `npm install -g gently-cc` on a machine without Rust → downloads
  pre-built binary, verifies signature (ref [A.1.3]), wires hooks → session starts
  with [CRIMSON] banner
- **Referenced by:** [D.1.3], [D.1.4]

---

#### [D.1.3] npm provenance

- **Requires:** [D.1.2], [A.1.1]
- **Files:** `.github/workflows/release.yml`
- **Action:**
  Add `--provenance` flag to npm publish:
  ```yaml
  - name: Publish
    run: npm publish --access public --provenance
    permissions:
      id-token: write  # required for provenance
  ```
  This cryptographically links the published npm package to the GitHub Actions run
  that produced it. Users can verify: `npm audit signatures gently-cc`.
  Combined with binary minisign (ref [A.1.2]) this gives two independent supply
  chain integrity layers.
- **Verify:** `npm audit signatures gently-cc` → `gently-cc@x.x.x: Signed by GitHub Actions`
- **Referenced by:** [D.1.4]

---

#### [D.1.4] Install integrity end-to-end test

- **Requires:** [D.1.3], [A.1.4], [A.3.3]
- **Files:** `.github/workflows/ci.yml`
- **Action:**
  Add an integration job to CI:
  ```yaml
  integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install from npm (simulated — use local package)
        run: npm install -g .
      - name: Verify install result
        run: |
          gently-cc verify
          gently-cc status
      - name: Run GRIM check
        run: node src/grim-scanner.js --verify
  ```
  This catches install regressions before they reach users.
- **Verify:** CI passes on clean ubuntu runner with no pre-existing state
- **Referenced by:** (final gate — no dependents, this is the exit condition)

---

## EXECUTION SUMMARY

```
Phase A (Security)    — do in parallel: [A.1] + [A.2] + [A.3] + [A.4]
                        A.1 and A.2 can start immediately.
                        A.3 needs grim-scanner written first [A.3.1].
                        A.4 is independent.

Phase B (CLI/TUI)     — start after A complete.
                        B.1 (TUI) is the longest — start first.
                        B.2 (MCP) and B.3 (CLI polish) can run in parallel with B.1.

Phase C (Trollz)      — start after A complete + B.1.1 done.
                        C.1 → C.2 → C.3 are sequential (each builds on the last).

Phase D (Distribution) — start after A + B + C complete.
                        D.1.1 is manual (npm token) — do it immediately, doesn't block anything.

Estimated grid refs: 43 total substeps.
Critical path: A.1.1 → A.1.2 → A.1.3 → B.1.1 → B.1.6 → C.1.1 → C.3.2 → D.1.2
```

---

*Last updated: 2026-04-27 | Canonical: /home/tom/Projects/gently-cc/PLAN.md*
