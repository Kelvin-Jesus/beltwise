#!/usr/bin/env bash
# Claude Code PreToolUse hook (Bash): before an agent's `git push`, run what CI runs and
# block the push if anything fails. Every push to main deploys, so this is the gate.
# Reads the hook payload on stdin; exit 2 blocks the tool call and shows stderr to Claude.
set -uo pipefail
cmd=$(jq -r '.tool_input.command // ""')
[[ "$cmd" =~ (^|[;&|[:space:]])git[[:space:]]+push ]] || exit 0
cd "$(git rev-parse --show-toplevel)" || exit 2
run() { "$@" > /tmp/beltwise-pre-push.log 2>&1 || { echo "Push blocked: '$*' failed (see /tmp/beltwise-pre-push.log)." >&2; tail -20 /tmp/beltwise-pre-push.log >&2; exit 2; }; }
run cargo fmt --all --check
run cargo clippy -p engine --all-targets --locked -- -D warnings
run cargo test -p engine --locked
run mise x -- npm run typecheck
run mise x -- npm run build
run mise x -- npm run check
exit 0
