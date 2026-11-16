#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "Run inside the repository." >&2; exit 1; }
cd "$ROOT"

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

run cargo fmt --manifest-path antares-agent/Cargo.toml -- --check
run cargo check --locked -p antares-agent
run cargo test --locked -p antares-agent
run cargo clippy --locked -p antares-agent --all-targets -- -D warnings

if [[ "${ANTARES_FULL_WORKSPACE:-0}" == "1" ]]; then
  run cargo check --locked --workspace
fi

printf '\nAntares CI loop passed. Inspect git diff and GitHub Actions before merging.\n'
