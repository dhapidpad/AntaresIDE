#!/usr/bin/env bash
# Fork Lapce -> clone as AntaresIDE -> apply safe overlay -> validate -> commit/push -> optional Codespace.
set -euo pipefail

usage() {
  cat <<'EOF'
Antares IDE one-click GitHub bootstrap

Usage:
  bash scripts/one-click-setup.sh [options]

Options:
  --repo-name NAME       Fork name (default: AntaresIDE)
  --target DIR           Clone destination (default: ../AntaresIDE)
  --branch NAME          Setup branch (default: agy/antares-bootstrap)
  --no-push              Create local commit but do not push
  --no-commit            Apply and test only; do not commit or push
  --create-codespace     Ask before creating a Codespace from the setup branch
  --yes                  Do not ask before commit/push; still asks before Codespace billing
  -h, --help             Show this help

Requires: gh (authenticated), git, python3. Rust/Cargo are optional here; GitHub Actions
will validate the Antares crate if Cargo is absent. The first Codespace run updates Cargo.lock.
EOF
}

REPO_NAME="${ANTARES_REPO_NAME:-AntaresIDE}"
TARGET=""
BRANCH="${ANTARES_SETUP_BRANCH:-agy/antares-bootstrap}"
PUSH=1
COMMIT=1
CREATE_CODESPACE=0
YES=0
while (($#)); do
  case "$1" in
    --repo-name) REPO_NAME="${2:?missing repo name}"; shift 2 ;;
    --target) TARGET="${2:?missing target path}"; shift 2 ;;
    --branch) BRANCH="${2:?missing branch}"; shift 2 ;;
    --no-push) PUSH=0; shift ;;
    --no-commit) COMMIT=0; PUSH=0; shift ;;
    --create-codespace) CREATE_CODESPACE=1; shift ;;
    --yes) YES=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

SOURCE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -z "$TARGET" ]]; then TARGET="$(cd "$SOURCE_ROOT/.." && pwd)/AntaresIDE"; fi
mkdir -p "$(dirname "$TARGET")"
TARGET="$(cd "$(dirname "$TARGET")" && pwd)/$(basename "$TARGET")"

for cmd in gh git python3; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "ERROR: required command '$cmd' is missing." >&2; exit 1; }
done

echo "[1/9] Checking GitHub CLI authentication..."
gh auth status >/dev/null
gh auth setup-git >/dev/null
OWNER="$(gh api user --jq .login)"
[[ -n "$OWNER" ]] || { echo "Could not determine GitHub username." >&2; exit 1; }

if [[ ! "$REPO_NAME" =~ ^[A-Za-z0-9_.-]{1,100}$ ]]; then
  echo "Invalid GitHub repository name: $REPO_NAME" >&2; exit 1
fi
FULL_REPO="$OWNER/$REPO_NAME"

repo_json=""
if repo_json="$(gh api "repos/$FULL_REPO" 2>/dev/null)"; then
  parent="$(python3 -c 'import json,sys; d=json.load(sys.stdin); print((d.get("parent") or {}).get("full_name", ""))' <<<"$repo_json")"
  is_fork="$(python3 -c 'import json,sys; print(str(json.load(sys.stdin).get("fork", False)).lower())' <<<"$repo_json")"
  if [[ "$is_fork" != "true" || "$parent" != "lapce/lapce" ]]; then
    echo "ERROR: $FULL_REPO already exists but is not a fork of lapce/lapce (parent='$parent', fork='$is_fork')." >&2
    echo "Choose another --repo-name; no repository was modified." >&2
    exit 1
  fi
  echo "[2/9] Reusing existing verified fork: $FULL_REPO"
else
  echo "[2/9] Creating fork lapce/lapce -> $FULL_REPO ..."
  gh repo fork lapce/lapce --fork-name "$REPO_NAME" --clone=false
  ready=0
  for _ in $(seq 1 30); do
    if gh api "repos/$FULL_REPO" >/dev/null 2>&1; then ready=1; break; fi
    sleep 2
  done
  if [[ "$ready" != "1" ]]; then
    echo "Fork creation was requested, but GitHub has not made $FULL_REPO visible yet." >&2
    echo "Re-run this script; it will reuse the fork if it is ready." >&2
    exit 1
  fi
fi

if [[ -e "$TARGET" && ! -d "$TARGET/.git" ]]; then
  echo "ERROR: target path exists but is not a Git repository: $TARGET" >&2; exit 1
fi
if [[ ! -d "$TARGET/.git" ]]; then
  mkdir -p "$(dirname "$TARGET")"
  echo "[3/9] Cloning $FULL_REPO to $TARGET ..."
  gh repo clone "$FULL_REPO" "$TARGET"
else
  echo "[3/9] Reusing existing clone: $TARGET"
fi

cd "$TARGET"
# Do not switch branches or stage files when the user has local modifications,
# including untracked files. This check intentionally runs before any branch operation.
if [[ -n "$(git status --porcelain)" ]]; then
  echo "ERROR: existing checkout has modified, staged, or untracked files." >&2
  echo "Commit/stash them or choose a new --target directory. Nothing was overwritten." >&2
  git status --short
  exit 1
fi

remote_slug() {
  local remote_url="$1"
  python3 - "$remote_url" <<'PYREMOTE'
import sys
from urllib.parse import urlsplit
raw = sys.argv[1].strip()
if raw.startswith("git@") and ":" in raw:
    host, path = raw[4:].split(":", 1)
elif raw.startswith("ssh://"):
    parsed = urlsplit(raw)
    host, path = parsed.hostname or "", parsed.path.lstrip("/")
else:
    parsed = urlsplit(raw)
    host, path = parsed.hostname or "", parsed.path.lstrip("/")
if host.lower() != "github.com":
    print("")
else:
    print(path.removesuffix(".git").strip("/"))
PYREMOTE
}

if ! git remote get-url origin >/dev/null 2>&1; then
  git remote add origin "https://github.com/$FULL_REPO.git"
fi
origin_slug="$(remote_slug "$(git remote get-url origin)")"
if [[ "${origin_slug,,}" != "${FULL_REPO,,}" ]]; then
  echo "ERROR: local origin points to '$origin_slug', expected '$FULL_REPO'. Refusing to push to another repository." >&2
  exit 1
fi
if ! git remote get-url upstream >/dev/null 2>&1; then
  git remote add upstream https://github.com/lapce/lapce.git
fi
upstream_slug="$(remote_slug "$(git remote get-url upstream)")"
if [[ "${upstream_slug,,}" != "lapce/lapce" ]]; then
  echo "ERROR: local upstream points to '$upstream_slug', expected 'lapce/lapce'. Fix remotes before continuing." >&2
  exit 1
fi

if git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  git switch "$BRANCH"
elif git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
  git fetch origin "$BRANCH"
  git switch --track -c "$BRANCH" "origin/$BRANCH"
else
  git switch -c "$BRANCH"
fi

echo "[4/9] Applying Antares overlay while preserving upstream files..."
python3 "$SOURCE_ROOT/scripts/apply-overlay.py" --source "$SOURCE_ROOT" --target "$TARGET"

# The overlay script is now in the fork; all subsequent commands run from that copy.
echo "[5/9] Applying idempotent Rust workspace and ignore integration..."
if [[ -f scripts/bootstrap.sh ]]; then
  if command -v cargo >/dev/null 2>&1; then
    bash scripts/bootstrap.sh
  else
    ANTARES_SKIP_CARGO=1 bash scripts/bootstrap.sh
  fi
else
  echo "ERROR: scripts/bootstrap.sh was not installed." >&2; exit 1
fi

bash scripts/scaffold-check.sh
bash scripts/antares-doctor.sh --static

echo "[6/9] Running Rust checks when Cargo is available..."
if command -v cargo >/dev/null 2>&1; then
  bash scripts/antares-ci.sh fast
else
  echo "Cargo is not installed on this host; Rust compile/test is deferred to the Codespace and GitHub Actions."
fi

git diff --check

echo "[7/9] Reviewing changed files..."
git status --short

git add -A
if git diff --cached --quiet; then
  echo "No new changes to commit."
else
  if [[ "$COMMIT" == "1" ]]; then
    if [[ -z "$(git config user.name || true)" ]]; then git config user.name "$OWNER"; fi
    if [[ -z "$(git config user.email || true)" ]]; then
      GH_ID="$(gh api user --jq .id)"
      git config user.email "${GH_ID}+${OWNER}@users.noreply.github.com"
    fi
    git commit -m "chore(antares): bootstrap Antares IDE foundation"
  else
    echo "Commit skipped by --no-commit. Changes are staged."
  fi
fi

if [[ "$PUSH" == "1" && "$COMMIT" == "1" ]]; then
  echo "[8/9] Pushing setup branch to GitHub..."
  git push -u origin "$BRANCH"
else
  echo "Push skipped. To push later: git push -u origin $BRANCH"
fi

if [[ "$CREATE_CODESPACE" == "1" && "$PUSH" == "1" && "$COMMIT" == "1" ]]; then
  echo "[9/9] Codespaces can consume account quota or incur charges after any included allowance."
  answer="n"
  if [[ "$YES" == "1" ]]; then
    echo "--yes does not bypass Codespaces billing confirmation."
  fi
  read -r -p "Create a Codespace for $FULL_REPO on $BRANCH now? [y/N] " answer
  if [[ "$answer" =~ ^[Yy]$ ]]; then
    gh codespace create \
      --repo "$FULL_REPO" \
      --branch "$BRANCH" \
      --devcontainer-path .devcontainer/antares/devcontainer.json \
      --display-name "Antares IDE"
  else
    echo "Codespace creation skipped. Create it later with the command in docs/13-ONE-CLICK-SETUP.md."
  fi
else
  echo "[9/9] Codespace creation not requested."
fi

printf '\nSUCCESS: Antares scaffold is installed in %s\n' "$TARGET"
printf 'Repository: https://github.com/%s\n' "$FULL_REPO"
printf 'Branch: %s\n' "$BRANCH"
printf 'Next: open the branch in Antigravity and paste docs/12-AGENT-MASTER-PROMPT.md\n'
