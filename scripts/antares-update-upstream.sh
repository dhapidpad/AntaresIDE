#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "Run inside the fork." >&2; exit 1; }
cd "$ROOT"
if ! git remote get-url upstream >/dev/null 2>&1; then
  git remote add upstream https://github.com/lapce/lapce.git
fi

echo "Fetching upstream refs (no merge/rebase/reset will be performed)..."
git fetch upstream --prune
BASE="$(gh repo view --json defaultBranchRef --jq .defaultBranchRef.name 2>/dev/null || git symbolic-ref refs/remotes/upstream/HEAD 2>/dev/null | sed 's#^refs/remotes/upstream/##' || true)"
BASE="${BASE:-master}"
CURRENT="$(git branch --show-current)"
printf 'Upstream base: %s\nCurrent branch: %s\n' "$BASE" "$CURRENT"
git log --oneline --left-right "HEAD...upstream/$BASE" | head -80 || true
printf '\nReview the output and docs/PATCHES.md. This helper never merges upstream automatically.\n'
