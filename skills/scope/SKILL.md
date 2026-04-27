# /scope — Manage Org→Team→Project Hierarchy (Sui)
<!-- grim_hash: sha256:{grim:scope} | orc: chain | kind: governance -->

Create and manage scopes. Every scope is a Sui Move object (scope.move).
Capability_set is LOCKED at creation. Equity is ratchet-only (never decreases).

## Triggers
- `/scope` — show current scope context
- `/scope create <type> <name>` — create Org/Team/Project/App/Folder
- `/scope status` — show scope winding level + decision status
- `/scope add <wallet> <role> <equity_bps>` — add principal
- `/scope capability <name>` — check if capability exists in current scope
- `/scope decisions` — show governance decision status
- `/scope activate` — activate scope (requires all decisions APPROVED)

## Scope types
```
0 = Org      — top level, owns teams
1 = Team     — owns projects
2 = Project  — owns apps + folders
3 = App      — deployed artifact
4 = Folder   — file namespace boundary
```

## Winding levels
```
1 DRAFT       → just created, not usable
2 STRUCTURED  → metadata complete
3 REFINED     → decisions pending review
4 REVIEWED    → audit complete
5 DEPLOYED    → live in production
6 ARCHIVED    → immutable historical record
```

## Activation requirements
ALL governance decisions must be APPROVED before scope can activate:
- `charter` — mission statement (grade ≤5 plain language)
- `attribution` — contributor equity split agreed
- `fork_policy` — what happens on disagreement
- Any custom decisions added at creation

## Capabilities (never expandable after creation)
Common capability set for a Project scope:
```json
["code:write", "code:read", "test:write", "api:write", "db:migrate",
 "discord", "telegram", "social:read", "social:post", "agents:read"]
```

To check a capability: `POST /scope/{id}/capabilities/{name}` → 200 or 403

## Principal stake
```
role 0 = creator  (first principal, equity locked)
role 1 = lead     (decision voting rights)
role 2 = member   (contribution tracked)
role 3 = advisor  (reduced equity, no votes)
role 4 = viewer   (read-only)

equity_bps     = current equity (0–10000 = 0–100%)
locked_equity  = ratchet floor (only ever increases)
share_floor    = Saverin clause (anti-rug minimum)
```

## CODIE expression
```
pug SCOPE
├── bark current ← @runtime/scope_context
├── if create
│   ├── pin type ← {org|team|project|app|folder}
│   ├── pin name ← user_arg
│   ├── bark tx ← sui.create_scope(type, name, capability_set)
│   └── biz → scope_id
├── if add_principal
│   ├── bark tx ← sui.add_principal(scope_id, wallet, role, equity_bps)
│   └── fence → locked_equity only increases
├── if activate
│   ├── bark decisions ← scope.decisions.all_approved?
│   ├── fence → if not all_approved { biz "Missing approvals" }
│   └── bark tx ← sui.activate_scope(scope_id)
└── biz → display_scope_state
```
