# /team — Multi-Agent Coordination via Trollz Routing
<!-- grim_hash: sha256:{grim:team} | orc: worg | kind: orchestration -->

Spin up N specialized agents on a shared task. Agents are routed via trollz-api, capability-gated by Sui scope, and coordinated through the Alexandria crystal DAG.

## Triggers
- `/team <N> <task>` — N agents on task
- `/team <task>` — auto-determine N from task complexity
- `team: <task>` — same
- `team pipeline: <task>` — explicit staged pipeline

## Stages (Pipeline)
```
plan → prd → exec[N] → verify → fix? → complete
```

1. **plan** — Decompose task into N parallel subtasks, write `.gently/team-{id}/plan.md`
2. **prd** — Write acceptance criteria + test spec, `.gently/team-{id}/prd.md`
3. **exec** — N agents work in parallel on assigned subtasks
4. **verify** — Check all subtasks complete + meet acceptance criteria
5. **fix** — If verify fails, route failed tasks back to exec
6. **complete** — All verified. Crystal checkpoint. SYNTH calculated.

## Agent Routing (trollz-api)
Each subtask routes to the right agent type based on capability:
- Code changes → `executor` agent (capability: `code:write`)
- Reviews → `reviewer` agent (capability: `code:read`)
- Tests → `test-engineer` agent (capability: `test:write`)
- DB changes → slog orc (capability: `db:migrate`)
- API routes → zug orc (capability: `api:write`)

Capability check: `POST /scope/{scope_id}/capabilities/{cap}` before each dispatch.

## State (.gently/team-{id}/)
```
team-{id}/
├── plan.md          task decomposition
├── prd.md           acceptance criteria
├── tasks/
│   ├── task-1.md    {status: pending|in_progress|done|failed}
│   └── task-N.md
├── workers/
│   └── worker-K/
│       ├── inbox.md      assigned tasks
│       ├── outbox.jsonl  results log
│       └── heartbeat     liveness
└── events.jsonl     full event timeline (append-only)
```

## SYNTH accounting
Team sessions earn SYNTH per successful task × rarity multiplier.
Crystal checkpoint fired on pipeline complete — logs `synth_total` to bb-runtime.

## CODIE expression
```
pug TEAM
├── pin task ← user_input
├── bark n ← complexity_score(task) or user_arg
├── bark subtasks ← decompose(task, n)
├── anchor → .gently/team-{id}/plan.md
├── bark prd ← write_acceptance_criteria(subtasks)
├── fork n workers
│   └── spin worker.execute(subtask)
├── bark verified ← verify_all(subtasks)
├── if not verified → fix(failed) → verify again
└── biz → crystal_checkpoint + synth_calc
```
