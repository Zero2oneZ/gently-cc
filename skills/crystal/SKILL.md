# /crystal — Fork Session as Crystal Node
<!-- grim_hash: sha256:{grim:crystal} | orc: forge | kind: state -->

Checkpoint or seal the current session as an immutable crystal node in the Alexandria DAG.
Crystals are lineage-tracked, forkable, and SYNTH-bearing.

## Triggers
- `/crystal` — checkpoint (non-terminal, session continues)
- `/crystal seal` — terminal crystallize (ends agent lifespan)
- `/crystal fork` — fork current crystal into new branch
- `/crystal show` — show current crystal lineage
- `/crystal list` — list all crystals in this project

## Crystal structure
```json
{
  "id": "sha256:{final_state_hash}",
  "parent_id": "sha256:{previous_crystal}",
  "session_id": "{cc_session_id}",
  "project_path": "{abs_path}",
  "synth_earned": 0.0042,
  "interaction_count": 47,
  "quality_score": 0.87,
  "codie_stats": {
    "tokens_original": 12480,
    "tokens_encoded": 661,
    "pct_saved": 94.7
  },
  "timestamp": 1714089600000,
  "phase": "checkpoint | terminal",
  "codie_archive": "spaces://trollz-agents/crystals/{id}/codie.bb"
}
```

## Checkpoint vs Seal
- **Checkpoint** (`/crystal`): Saves state. Session continues. Crystal node added to DAG. Agent stays ACTIVE.
- **Seal** (`/crystal seal`): Terminal. Agent transitions to Crystal phase. No more interactions. SYNTH distributed. Crystal NFT minted on Sui.

## Fork
`/crystal fork` creates a new session branching from the current crystal:
- New crystal gets `parent_id` = current crystal `id`
- Both branches track independently
- Lineage is always reconstructable from the DAG

## SYNTH on crystallize
```
synth_earned = api_cost_usd × rarity_multiplier × log10(interactions + 1) × quality_score × SYNTH_USD_RATE
```
Shown at crystal seal: `Crystal sealed. SYNTH earned: 0.0042 SYNTH (QS: 0.87 | turns: 47)`

## CODIE expression
```
pug CRYSTAL
├── bark state ← @runtime/session_state
├── bark hash ← SHA256(serialize(state))
├── anchor → .gently/crystals/{hash}.json
├── bark parent ← .gently/crystals/latest
├── bark synth ← synth_calc(state)
├── if seal
│   ├── bark tx ← sui.mint_crystal(hash, synth)
│   └── bone → terminal (no more transitions)
├── if fork
│   └── bark new_session ← branch(hash)
└── biz → display_crystal_info
```
