# /gently — GentlyOS Substrate Commands
<!-- grim_hash: sha256:{grim:gently} | orc: lurk | kind: meta -->

Access GentlyOS substrate primitives directly. Memory, knowledge graph, foam storage, ops events.

## Triggers
- `/gently mem <concept>` — search Alexandria + BARF for concept
- `/gently align <from> → <to>` — record Alexandria edge
- `/gently pin <content>` — pin to IPFS, get [MEM:key:CID] token
- `/gently recall <key>` — recall by ternary key or CID
- `/gently ops` — show ops_events queue status
- `/gently status` — full substrate health check
- `/gently tree` — show GRIM manifest as ternary address tree

## Memory (Alexandria + BS-Artisan)

`/gently mem <concept>`:
1. `trollz_mem_search(concept)` → BARF XOR-distance search over foam
2. `forward_search(concept)` → Alexandria: what it IS
3. `rewind_search(concept)` → Alexandria: what leads HERE
4. Display ranked results with trustworthiness scores

`/gently align <from> → <to>`:
1. `trollz_mem_align({from, to, direction: "→"})` — record directed edge
2. Both concepts become torus nodes in foam if not already present
3. Edge appears in Alexandria graph + boosts BARF retrieval

`/gently pin <content>`:
1. `trollz_mem_pin({content})` → IPFS CID
2. Returns `[MEM:{ternary_key}:{CID}]` — embeddable token
3. Content retrievable forever via `/gently recall <key>`

## Ops Events
`/gently ops` calls `trollz_ops_poll` and shows pending events:
```
Ops Queue: {n} pending events
─────────────────────────────
[{id}] session.started   2m ago   session:{sid}
[{id}] test.failed       4m ago   cargo test (3 failures)
[{id}] crystal.sealed    1h ago   sha256:{hash} +0.0042 SYNTH
```

Acknowledge: `/gently ops ack {id}` → `trollz_ops_ack({id})`

## Substrate health
`/gently status`:
```
GentlyOS Substrate
──────────────────────────────────────────
trollz-api     ✓  http://localhost:8080/health
trollz-mcp     ✓  stdio transport
Alexandria     ✓  {n} concepts, {m} edges
BS-Artisan     ✓  {k} tori in foam
GRIM manifest  ✓  {f} functions, {e} entries
Sui network    ✓  testnet | {block}
CODIE engine   ✓  v{version} | 44 keywords
SYNTH balance  ✓  {balance} SYNTH
```

## CODIE expression
```
pug GENTLY
├── if mem → bark results ← barf(concept) + alexandria(concept)
├── if align → bark tx ← trollz_mem_align(from, to)
├── if pin → bark token ← trollz_mem_pin(content)
├── if recall → bark content ← trollz_mem_recall(key)
├── if ops → bark events ← trollz_ops_poll()
├── if status → bark health ← trollz_ops_status()
└── biz → display_result
```
