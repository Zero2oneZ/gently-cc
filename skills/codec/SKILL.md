# /codec — CCP Pin Grid

Manage the hierarchical pin grid (GLOBAL → PROJECT → USER → CHAT).

## Commands

- `/codec ref <name>` — look up a pin and show its scope level
- `/codec list` — show all active pins for current project
- `/codec context` — dump compact project pin table (inject into new context)
- `/codec crystal` — seal current chat scope as a Crystal checkpoint
- `/codec load <prefix>` — resume from a Crystal checkpoint by hash prefix
- `/codec conflicts` — show pins defined at multiple scope levels with different labels

## What it does

Pins compress repeated concepts across the conversation.
`[*auth: authentication and session management]` → every future `*auth` saves ~4 tokens.

Scope hierarchy: CHAT → USER → PROJECT → GLOBAL. Resolution walks up — first hit wins.
Promotion: chat pin with ≥5 refs → user scope. User pin across ≥3 sessions → project scope.

## Pin syntax

```
[*name: label text]   — declare pin in current scope
*name                 — reference (compressed on wire, expanded for model)
[*a → *b]             — directed edge between pins
[*a ≡ *b]             — merge: a and b are the same referent
[*a: ~]               — tombstone (retire) pin a
```

## Usage examples

```
/codec ref auth
/codec list
/codec context
/codec crystal
/codec load a3f2
/codec conflicts
```

## Integration

Pins are stored at:
- `~/.gently/global/pins.json`
- `~/.gently/projects/{project_id}/pins.json`
- `~/.gently/users/{user_id}/pins.json`
- `~/.gently/sessions/{session_id}/pins.json`

Crystal checkpoints are sealed by `codec crystal` and loaded by `codec load <prefix>`.
Session-start auto-injects the project pin table via `codec context`.
