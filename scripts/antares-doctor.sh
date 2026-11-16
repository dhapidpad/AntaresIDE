#!/usr/bin/env bash
set -euo pipefail
STATIC_ONLY=0
[[ "${1:-}" == "--static" ]] && STATIC_ONLY=1
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; fi
cd "$ROOT"

section() { printf '\n--- %s ---\n' "$1"; }
section "Repository"
printf 'Root: %s\n' "$ROOT"
printf 'Branch: %s\n' "$(git branch --show-current 2>/dev/null || true)"
python3 - <<'PY'
import subprocess
from urllib.parse import urlsplit
try:
    remotes = subprocess.check_output(["git", "remote"], text=True).splitlines()
except Exception:
    remotes = []
for name in remotes:
    try:
        raw = subprocess.check_output(["git", "remote", "get-url", name], text=True).strip()
        if raw.startswith("git@") and ":" in raw:
            host, path = raw[4:].split(":", 1)
        else:
            u = urlsplit(raw)
            host, path = u.hostname or "unknown-host", u.path.lstrip("/")
        print(f"remote {name}: {host}/{path.removesuffix('.git')}")
    except Exception:
        print(f"remote {name}: unreadable")
PY
if command -v gh >/dev/null 2>&1; then
  printf '\nGitHub CLI: %s\n' "$(gh --version | head -1)"
  if [[ "$STATIC_ONLY" == "0" ]]; then
    gh auth status || true
    current="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null || true)"
    if [[ -n "$current" ]]; then
      printf 'GitHub repo: %s\n' "$current"
      gh api "repos/$current" --jq '"fork=" + (.fork|tostring) + " parent=" + ((.parent.full_name // "none"))' 2>/dev/null || true
    fi
  fi
fi
section "Toolchain"
for cmd in rustc cargo rustup python3 git gh; do
  if command -v "$cmd" >/dev/null 2>&1; then
    case "$cmd" in
      rustup) printf '%-8s %s\n' "$cmd" "$(rustup show active-toolchain 2>/dev/null || true)" ;;
      *) printf '%-8s %s\n' "$cmd" "$("$cmd" --version 2>/dev/null | head -1)" ;;
    esac
  else
    printf '%-8s MISSING\n' "$cmd"
  fi
done
section "Antares files"
for path in AGENTS.antares.md GEMINI.antares.md antares-agent/Cargo.toml docs/STATUS.md docs/02-ROADMAP.md docs/12-AGENT-MASTER-PROMPT.md .devcontainer/antares/devcontainer.json .github/workflows/antares-ci.yml; do
  [[ -e "$path" ]] && printf '[OK] %s\n' "$path" || printf '[MISSING] %s\n' "$path"
done
if command -v cargo >/dev/null 2>&1 && [[ "$STATIC_ONLY" == "0" ]]; then
  section "Cargo metadata"
  cargo metadata --no-deps --format-version 1 >/dev/null && echo "Cargo metadata: OK"
fi
printf '\nDoctor complete. No credentials or environment secrets were printed.\n'
