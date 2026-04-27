# /autopilot — Full Autonomous Execution
<!-- grim_hash: sha256:{grim:autopilot} | orc: lurk | kind: orchestration -->

Full autonomous from description to working, verified output. CODIE-compressed throughout.
Uses team pipeline internally. Earns SYNTH on completion.

## Triggers
- `autopilot: <description>` — start full autonomous run
- `/autopilot <description>` — same
- `auto: <description>` — shorthand

## Pipeline
```
deep-interview? → codie-compress → scope-check → grind-scan →
team(N) → verify → crystal-checkpoint → done
```

1. **Pre-flight** — Check current scope capabilities. Warn if required caps missing.
2. **CODIE compress** — Compress description. Show token savings.
3. **GRIM scan** — Identify entry-point hashes for this task domain.
4. **Decompose** — Break into N subtasks routed to correct Orcs.
5. **Execute** — team(N) with appropriate agents per subtask.
6. **Verify** — Build, test, functional check. Loop on failure.
7. **Crystal checkpoint** — Save state. SYNTH calculated.

## Asks NO questions mid-run
Ask everything before activating. If description is ambiguous, recommend `/deep-interview` first.

## Output on completion
```
✓ AUTOPILOT COMPLETE
─────────────────────────────────────
Task     : {description}
Subtasks : {n} complete, 0 failed
Turns    : {k}
CODIE    : {pct}% avg reduction · {tokens} tokens saved
SYNTH    : +{synth} earned
Crystal  : sha256:{hash}
Time     : {elapsed}s
─────────────────────────────────────
```

## CODIE expression
```
pug AUTOPILOT
├── pin desc ← user_input
├── bark compressed ← codie.compress(desc)
├── bark scope ← @runtime/current_scope
├── fence → missing_caps { biz "Missing: {caps}" }
├── bark entry_hashes ← grim.entry_points(scope)
├── bark subtasks ← decompose(compressed, entry_hashes)
├── fork team(subtasks)
├── bark verified ← verify_all()
├── spin if not verified → fix → verify
└── biz → crystal_checkpoint + synth_calc + report
```
