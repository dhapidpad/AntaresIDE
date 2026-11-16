#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "ERROR: run this script from inside a Git repository." >&2
  exit 1
}
cd "$ROOT"

required=(Cargo.toml Makefile .gitignore .gitignore.antares LICENSE lapce-app/Cargo.toml lapce-core lapce-proxy lapce-rpc antares-agent/Cargo.toml AGENTS.md AGENTS.antares.md .agent .agents)
for path in "${required[@]}"; do
  if [[ ! -e "$path" ]]; then
    echo "ERROR: '$path' is missing. Extract the Antares starter into the root of a Lapce fork first." >&2
    exit 1
  fi
done

python3 - <<'PY'
from pathlib import Path
import re, tomllib

root = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))
workspace = root.get("workspace", {})
package = workspace.get("package", {})
for key in ("version", "edition", "rust-version", "license", "authors", "homepage"):
    if key not in package:
        raise SystemExit(f"Lapce workspace.package.{key} is missing; inspect the upstream version before bootstrapping.")
if package["edition"] != "2024":
    raise SystemExit(f"Expected Lapce edition 2024, found {package['edition']!r}. Review this upstream change before continuing.")

cargo = Path("Cargo.toml")
text = cargo.read_text(encoding="utf-8")
match = re.search(r"(?ms)^members\s*=\s*\[(.*?)\]", text)
if not match:
    raise SystemExit("Could not locate [workspace].members array in root Cargo.toml; inspect upstream manually.")
items = match.group(1)
if '"antares-agent"' not in items and "'antares-agent'" not in items:
    updated = items.rstrip()
    if updated and not updated.rstrip().endswith(","):
        updated += ","
    updated += ' "antares-agent" '
    text = text[:match.start(1)] + updated + text[match.end(1):]
    cargo.write_text(text, encoding="utf-8")
    print("Added antares-agent to Cargo workspace members.")
else:
    print("antares-agent already belongs to the workspace.")

makefile = Path("Makefile")
line = "-include Makefile.antares"
contents = makefile.read_text(encoding="utf-8")
if line not in contents.splitlines():
    makefile.write_text(contents.rstrip() + "\n\n" + line + "\n", encoding="utf-8")
    print("Added Antares Makefile include.")

ignore = Path(".gitignore")
contents = ignore.read_text(encoding="utf-8")
extra = [
    line for line in Path(".gitignore.antares").read_text(encoding="utf-8").splitlines()
    if line.strip() and not line.lstrip().startswith("#")
]
missing = [line for line in extra if line not in contents.splitlines()]
if missing:
    ignore.write_text(contents.rstrip() + "\n\n# Antares local files and secrets\n" + "\n".join(missing) + "\n", encoding="utf-8")
    print("Added missing Antares ignore entries.")
PY

if ! git remote get-url upstream >/dev/null 2>&1; then
  git remote add upstream https://github.com/lapce/lapce.git
fi

if [[ "${ANTARES_SKIP_CARGO:-0}" == "1" ]]; then
  echo "Cargo validation skipped by ANTARES_SKIP_CARGO=1; run scripts/antares-ci.sh fast inside Codespaces."
elif command -v cargo >/dev/null 2>&1; then
  cargo metadata --no-deps --format-version 1 >/dev/null
  # This first check resolves the new workspace member and updates Cargo.lock when needed.
  cargo check -p antares-agent
else
  echo "Cargo is not installed; compile checks are deferred to GitHub Actions/Codespaces."
fi
printf '\nBootstrap complete. Review the diff, then run: bash scripts/antares-ci.sh fast\n'
printf 'Upstream remote: %s\n' "$(git remote get-url upstream)"
