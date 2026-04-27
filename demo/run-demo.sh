#!/usr/bin/env bash
# gently-cc demo: three sessions, three themes, one anomaly bus
#
# What you'll see:
#   Pane 0 [CRIMSON]  gently-cc       — normal prompt, clean compression
#   Pane 1 [COBALT]   trollz-fun      — duplicate prompt trips 🟡◉-D
#   Pane 2 [SLATE]    ide-workspace   — foreign path trips 🔴◉-S
#   Pane 3            Organizer       — tail -f ~/.gently/anomalies.jsonl
#
# Requires: tmux, node, jq (for pretty anomaly output)
# Usage: bash demo/run-demo.sh

set -e

HOOK="node $(cd "$(dirname "$0")/.." && pwd)/hooks/codie-compress.js"
ANOMALIES="$HOME/.gently/anomalies.jsonl"
GENTLY="$HOME/.gently"

# Ensure dirs + clear anomalies for a clean demo run
mkdir -p "$GENTLY"
> "$ANOMALIES"

# Fake identity writer — gives each pane its own project scope
write_identity() {
  local project="$1" handle="$2" theme="$3" project_id="$4"
  local path="$GENTLY/agent-identity-${project}.json"
  cat > "$path" <<JSON
{
  "name": "Claudius-${handle}",
  "handle": "${handle}",
  "theme": "${theme}",
  "project": "${project}",
  "project_id": "${project_id}",
  "project_path": "/home/tom/Projects/${project}",
  "created_at": $(date +%s)000
}
JSON
  echo "$path"
}

ID_GENTLY=$(write_identity "gently-cc"     "GREG"  "Crimson" "a3f91bc2e47d58f0")
ID_TROLLZ=$(write_identity "trollz-fun"    "TRLZ"  "Cobalt"  "7d2e8f1a3c5b9047")
ID_IDE=$(write_identity    "ide-workspace" "IDEV"  "Slate"   "4c1a7f3e2b8d60f5")

# Script runner: pipes a prompt through the hook with a given identity active
run_prompt() {
  local identity_file="$1"
  local prompt="$2"
  # Temporarily swap the identity
  cp "$identity_file" "$GENTLY/agent-identity.json"
  echo "{\"prompt\": $(jq -Rs . <<< "$prompt")}" | node "$(dirname "$0")/../hooks/codie-compress.js" > /dev/null
}

# ── Scripted prompt sequences ─────────────────────────────────

demo_gently() {
  echo ""
  echo "═══ [CRIMSON] gently-cc — clean session ═══"
  echo ""
  sleep 1
  run_prompt "$ID_GENTLY" \
    "fetch the user from the database, if not found return an error, otherwise return a session token with compression stats"
  sleep 1
  run_prompt "$ID_GENTLY" \
    "define the authentication middleware, bind the session variable, loop over pending requests and return the result"
  sleep 1
  echo "✓ gently-cc: 2 clean prompts compressed"
}

demo_trollz_duplicate() {
  echo ""
  echo "═══ [COBALT] trollz-fun — duplicate detection ═══"
  echo ""
  sleep 1
  local prompt="fetch all agent listings from the marketplace and return them sorted by stake weight descending"
  run_prompt "$ID_TROLLZ" "$prompt"
  sleep 1
  # Same prompt again — should trip 🟡◉-D
  run_prompt "$ID_TROLLZ" "$prompt"
  sleep 1
  echo "✓ trollz-fun: duplicate detected → 🟡◉-D written to anomalies.jsonl"
}

demo_ide_foreign_path() {
  echo ""
  echo "═══ [SLATE] ide-workspace — foreign scope ═══"
  echo ""
  sleep 1
  # Prompt referencing a foreign project path — should trip 🔴◉-S
  run_prompt "$ID_IDE" \
    "look at /home/tom/Projects/trollz-fun/apps/api/src/routes/agents.rs and refactor the handler to match the gently-cc codec pin format"
  sleep 1
  echo "✓ ide-workspace: foreign path detected → 🔴◉-S written to anomalies.jsonl"
}

demo_cluster() {
  echo ""
  echo "═══ [CRIMSON] gently-cc — stitched paste ═══"
  echo ""
  sleep 1
  # Long multi-paragraph stitched prompt — should trip 🟡◉-C
  run_prompt "$ID_GENTLY" \
"Fix the auth middleware in the API layer.

The current implementation stores session tokens in plain text in the database.
This violates the compliance requirements flagged by legal last quarter.

Also the marketplace listing endpoint is returning 500 errors on null agent stakes.
The nullable field is causing a panic in the sorting comparator.

Additionally we need to update the Sui contract to add the new capability field.
The guild.move module needs a migration before Thursday's release cut.

Finally the frontend SDF renderer is flickering on mobile Safari. The WebGL
context is being lost on background tab switches and not recovering."
  sleep 1
  echo "✓ gently-cc: stitched paste detected → 🟡◉-C written to anomalies.jsonl"
}

# ── tmux layout ───────────────────────────────────────────────

if [ "${1:-}" = "--no-tmux" ]; then
  # CI / headless mode: run all sequences inline
  demo_gently
  demo_trollz_duplicate
  demo_ide_foreign_path
  demo_cluster
  echo ""
  echo "═══ Anomaly bus ═══"
  if command -v jq &>/dev/null; then
    cat "$ANOMALIES" | jq -r '"\(.ts | strftime("%H:%M:%S"))  \(.label)  \(.project)  \(.reason)"' 2>/dev/null || cat "$ANOMALIES"
  else
    cat "$ANOMALIES"
  fi
  exit 0
fi

# tmux: four panes
SESSION="gently-demo"
tmux kill-session -t "$SESSION" 2>/dev/null || true
tmux new-session -d -s "$SESSION" -x 220 -y 50

# Pane 0: gently-cc (CRIMSON) — left top
tmux send-keys -t "$SESSION:0" "
echo 'Pane 0: [CRIMSON] gently-cc' && bash '$(realpath "$0")' --pane gently
" Enter

# Pane 1: trollz-fun (COBALT) — right top
tmux split-window -h -t "$SESSION:0"
tmux send-keys -t "$SESSION:0.1" "
echo 'Pane 1: [COBALT] trollz-fun' && bash '$(realpath "$0")' --pane trollz
" Enter

# Pane 2: ide-workspace (SLATE) — left bottom
tmux split-window -v -t "$SESSION:0.0"
tmux send-keys -t "$SESSION:0.2" "
echo 'Pane 2: [SLATE] ide-workspace' && bash '$(realpath "$0")' --pane ide
" Enter

# Pane 3: Organizer — right bottom
tmux split-window -v -t "$SESSION:0.1"
tmux send-keys -t "$SESSION:0.3" "
echo 'Pane 3: Organizer — watching anomaly bus'
echo '────────────────────────────────────────'
touch '$ANOMALIES'
tail -f '$ANOMALIES' | while IFS= read -r line; do
  ts=\$(echo \"\$line\" | jq -r '.ts // empty' 2>/dev/null | xargs -I{} date -d @\$(({}/1000)) '+%H:%M:%S' 2>/dev/null || echo '??:??:??')
  label=\$(echo \"\$line\" | jq -r '.label // \"?\"' 2>/dev/null)
  proj=\$(echo \"\$line\" | jq -r '.project // \"?\"' 2>/dev/null)
  reason=\$(echo \"\$line\" | jq -r '.reason // \"?\"' 2>/dev/null)
  echo \"\$ts  \$label  [\$proj]  \$reason\"
done
" Enter

# Individual pane entry points
if [ "${1:-}" = "--pane" ]; then
  case "${2:-}" in
    gently) demo_gently; demo_cluster ;;
    trollz) sleep 2; demo_trollz_duplicate ;;
    ide)    sleep 3; demo_ide_foreign_path ;;
  esac
  exit 0
fi

tmux attach -t "$SESSION"
