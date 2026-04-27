# /ralph — Persistence Mode (Won't Stop Until Done)
<!-- grim_hash: sha256:{grim:ralph} | orc: forge | kind: orchestration -->

Ralph mode: self-referential execution loop. Will not stop, summarise, or ask for permission until the task is verified complete. Named after the PERMANENT lifespan agent archetype.

## Triggers
- `ralph: <task>` — activate for specific task
- `/ralph <task>` — same
- `ralph mode` — activate for the current task in context

## Behaviour

On activation:
1. Acknowledge: `⚡ RALPH MODE — I will not stop until this is done and verified.`
2. CODIE-compress the task specification
3. Decompose into subtasks (max 8) — write to `.gently/ralph-tasks.md`
4. Execute subtasks sequentially, ticking each on completion
5. After all subtasks: run verification (build, test, or functional check — whatever fits)
6. If verification fails: loop back to failed subtask, fix, re-verify
7. Only stop when verification passes

## Loop structure
```
while not verified:
    for task in pending_tasks:
        execute(task)
        tick(task)
    verify()
    if verify.failed:
        reopen(failed_tasks)
```

## State persistence
- Writes `.gently/ralph-tasks.md` on every tick
- Format: `- [x] task text` / `- [ ] task text`
- Survives compaction — hook reads this file on session resume

## Constraints
- Never says "I've completed X, should I continue?"
- Never summarises progress mid-task
- Never asks for clarification after start (ask before activating)
- Never marks complete until verification passes

## CODIE expression
```
pug RALPH
├── pin task ← user_input
├── bark subtasks ← decompose(task, max=8)
├── anchor → .gently/ralph-tasks.md
├── spin subtask in subtasks
│   ├── bark result ← execute(subtask)
│   ├── anchor tick(subtask)
│   └── if result.failed → requeue(subtask)
├── bark verified ← verify()
└── if not verified → spin again
    else biz → "✓ RALPH COMPLETE — verified"
```
