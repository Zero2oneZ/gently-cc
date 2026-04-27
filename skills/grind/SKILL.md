# /grind — GRIM Hash-Addressed Dispatch
<!-- grim_hash: sha256:{grim:grind} | orc: grim | kind: runtime -->

Execute a function by its GRIM hash. The only execution path in the GentlyOS runtime.
Unknown hash → None → stop. No name-based dispatch. No injection possible.

## Triggers
- `/grind <hash>` — execute function by SHA-256 hash
- `/grind scan` — scan current project and show manifest
- `/grind verify` — verify all skill hashes against manifest
- `/grind show <hash>` — show descriptor for hash without executing
- `/grind list` — list all entry-point hashes in this project
- `grind: <hash>` — shorthand execute

## Hash format
```
sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```
Always prefixed `sha256:`. 64-char lowercase hex.

## Execution
```
grind(hash, ctx) →
  1. manifest.resolve(hash) → FunctionDescriptor or None
  2. if None → "Unknown hash. Stop." (no fallback, no guess)
  3. check ctx.orc_filter matches descriptor.orc
  4. if orc mismatch → "Cross-orc execution forbidden."
  5. execute function with context payload
  6. read descriptor.calls[0] → next_hash
  7. if next_hash → grind(next_hash, ctx) (chain)
  8. else → return result
```

## Scan output
`/grind scan` runs `grim-scanner scan` on current directory and shows:
```
GRIM Scan — {project_name}
Files scanned  : {n}
Functions found: {m}
Entry points   : {k}
Cross-orc edges: {x} (should be 0)
Unassigned fns : {u} (should be 0)

Entry points:
  sha256:abc123... | route_handler        | zug  | apps/api/routers/auth.rs
  sha256:def456... | mint_agent           | chain| contracts/sources/agent_nft.move
  sha256:789abc... | compress             | codie| packages/codie-engine/src/compress.rs
```

## CODIE expression
```
pug GRIND
├── pin hash ← user_arg
├── bark descriptor ← manifest.resolve(hash)
├── fence → if None { biz "Unknown hash. Stop." }
├── fence → if orc_mismatch { biz "Cross-orc forbidden." }
├── bark result ← execute(descriptor, ctx)
├── bark next ← descriptor.calls[0]
└── if next → grind(next, ctx)
    else biz → result
```
