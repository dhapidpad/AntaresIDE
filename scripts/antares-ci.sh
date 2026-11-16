#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "Run inside the Antares/Lapce Git repository." >&2; exit 1; }
cd "$ROOT"
MODE="${1:-fast}"
run() { printf '\n==> %s\n' "$*"; "$@"; }

run bash scripts/scaffold-check.sh
run git diff --check
run cargo fmt -p antares-agent -- --check

case "$MODE" in
  fast)
    # Deliberately not --locked on the first bootstrap run; Cargo must add the new member to Cargo.lock.
    run cargo check -p antares-agent
    run cargo test -p antares-agent
    run cargo clippy -p antares-agent --all-targets -- -D warnings
    ;;
  locked)
    run cargo metadata --locked --no-deps --format-version 1
    run cargo check --locked -p antares-agent
    run cargo test --locked -p antares-agent
    run cargo clippy --locked -p antares-agent --all-targets -- -D warnings
    ;;
  full)
    run cargo check --workspace
    run cargo test --workspace
    run cargo clippy --workspace --all-targets -- -D warnings
    ;;
  full-locked)
    run cargo check --workspace --locked
    run cargo test --workspace --locked
    run cargo clippy --workspace --all-targets --locked -- -D warnings
    ;;
  *) echo "Usage: $0 {fast|locked|full|full-locked}" >&2; exit 2 ;;
esac
printf '\nAntares CI mode "%s" passed.\n' "$MODE"
