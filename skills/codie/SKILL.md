# /codie — Live CODIE Compression Engine
<!-- grim_hash: sha256:{grim:codie} | orc: codie | kind: util -->

Local binary. Zero network. 44-keyword glyph table. BLAKE3 hashes.

## Triggers

| Trigger | Action |
|---------|--------|
| `/codie` | Session stats from binary |
| `/codie <text>` | Compress text — bench output + hash |
| `/codie loop` | Activate continuous compression mode |
| `/codie table` | Full 44-keyword glyph table |
| `/codie hash <text>` | BLAKE3 content address |
| `/codie decompress <glyph>` | Reverse glyph → keyword |

---

## Binary Resolution

Always resolve binary in this order:
1. `./target/release/codie` (project root — preferred, freshest build)
2. `codie` (on PATH — installed globally)
3. Fallback: manual keyword substitution using the table in CLAUDE.md

Detect which with:
```bash
command -v ./target/release/codie 2>/dev/null || command -v codie 2>/dev/null
```

---

## Behaviour

### `/codie` — Session Stats

Run:
```bash
./target/release/codie stats
```

Display exactly as returned. Then append:
```
Hook status: active (UserPromptSubmit → codie hook)
Binary     : ./target/release/codie
Build      : cargo build --release -p codie
```

### `/codie <text>` — Compress + Bench

Run:
```bash
./target/release/codie bench "<text>"
```

Display the full bench output (original / compressed / hash / OpenClaw vs CODIE line).

Then run:
```bash
./target/release/codie decompress "<compressed_output>"
```

Show the decompressed form so Claude can verify round-trip fidelity.

### `/codie loop` — Continuous Compression Mode

Activate loop mode. On every subsequent user message:

1. Run `./target/release/codie compress --stdin` with the message text
2. Show: `⚡ CODIE {pct}% · {orig}→{comp} tokens`
3. Work from the compressed form internally
4. At session end, run `./target/release/codie stats`

Loop structure:
```
pug CODIE_LOOP
├── pin mode ← active
├── spin each user_message
│   ├── bark compressed ← codie compress <message>
│   ├── if pct_saved >= 10
│   │   ├── anchor stats ← codie stats
│   │   └── biz → work_from_compressed
│   └── else biz → work_from_original
└── on session_end → codie stats
```

Never disable loop mode until user says `/codie stop`.

### `/codie table` — Glyph Table

Run:
```bash
./target/release/codie table
```

Display as returned.

### `/codie hash <text>` — Content Address

Run:
```bash
./target/release/codie hash "<text>"
```

Output: `sha256:{blake3_hex}` — 64-char GRIM-format hash.

### `/codie decompress <glyph>` — Reverse

Run:
```bash
./target/release/codie decompress "<glyph>"
```

Show reconstructed keyword form.

---

## Live Stats Format

The binary emits this to stderr on every compression (also shown in hook output):
```
⚡ CODIE 44.7% · 38→21 tokens  [session: 18 saved · 3 turns · 16.9% avg]
```

When showing stats, always include this line verbatim from the binary output.

---

## CODIE Self-Description

```
pug CODIE
├── bark binary ← resolve(./target/release/codie, codie)
├── fence
│   └── bone NOT: call trollz.fun API (local binary only)
├── spin trigger
│   ├── if stats → bark session ← codie stats
│   ├── if text  → bark bench ← codie bench <text>
│   │              bark decomp ← codie decompress <compressed>
│   ├── if loop  → activate compression_mode
│   ├── if table → bark glyphs ← codie table
│   ├── if hash  → bark h ← codie hash <text>
│   └── if decompress → bark kw ← codie decompress <glyph>
└── biz → display_output
```

---

## Build Reference

If binary missing or stale:
```bash
cd /home/tom/Projects/gently-cc
cargo build --release -p codie
# Binary: ./target/release/codie
```

Tests:
```bash
cargo test -p codie
# Expected: 4 passed
```
