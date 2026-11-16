#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$ROOT" ]]; then ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; fi
cd "$ROOT"

python3 - <<'PY'
from pathlib import Path
import json, tomllib

required = [
    "README.md", "AGENTS.md", "AGENTS.antares.md", "GEMINI.md", "GEMINI.antares.md", "THIRD_PARTY_LICENSES.md",
    "Makefile.antares", "rust-toolchain.toml", ".agent/rules/00-core.md",
    ".agent/rules/01-rust-workspace.md", ".agent/workflows/new-feature.md",
    ".devcontainer/antares/devcontainer.json", ".devcontainer/antares/post-create.sh",
    "antares-agent/Cargo.toml", "antares-agent/src/lib.rs", "antares-agent/tests/README.md", "antares-agent/tests/fixtures/acp/README.md", "antares-agent/tests/fixtures/fake-agent/README.md", "antares-agent/tests/README.md", "antares-agent/tests/fixtures/acp/README.md", "antares-agent/tests/fixtures/fake-agent/README.md",
    "docs/01-ARCHITECTURE.md", "docs/04-OCTOMIND-ACP.md", "docs/06-AGY-KICKOFF.md",
    "docs/07-SECURITY-MODEL.md", "docs/10-TEST-PLAN.md",
    "docs/12-AGENT-MASTER-PROMPT.md", "docs/13-ONE-CLICK-SETUP.md",
    "docs/14-GITHUB-AUTOMATION.md", "docs/15-TEST-MATRIX.md",
    "docs/16-AGENT-TASK-PROMPTS.md", "scripts/bootstrap.sh", "scripts/ci-loop.sh",
    "scripts/one-click-setup.sh", "scripts/apply-overlay.py", "scripts/antares-ci.sh",
    "scripts/antares-doctor.sh", "scripts/antares-update-upstream.sh",
    "scripts/install-octomind.sh",
]
missing = [p for p in required if not Path(p).is_file()]
if missing:
    raise SystemExit("Missing required Antares files: " + ", ".join(missing))

# The starter ZIP stores workflows under a non-active folder to avoid shadowing upstream files;
# after application, apply-overlay.py copies them into the active .github/workflows directory.
workflow_candidates = [Path(".github/workflows/antares-ci.yml"), Path(".github/workflows-antares/antares-ci.yml")]
if not any(p.is_file() for p in workflow_candidates):
    raise SystemExit("Missing Antares CI workflow template")

json.loads(Path(".devcontainer/antares/devcontainer.json").read_text(encoding="utf-8"))

candidates = {
    "guardrails": [Path(".agents/guardrails.toml")],
    "lock": [Path("config-templates/antares/octomind.lock"), Path("config-templates/octomind.lock")],
    "role": [Path("config-templates/antares/roles.antares.example.toml"), Path("config-templates/roles.antares.example.toml")],
    "profiles": [Path("config-templates/antares/octomind-google-studio-profiles.example.toml"), Path("config-templates/octomind-google-studio-profiles.example.toml")],
}
for label, paths in candidates.items():
    found = next((p for p in paths if p.is_file()), None)
    if found is not None:
        tomllib.loads(found.read_text(encoding="utf-8"))
    elif label in ("guardrails", "lock", "role"):
        raise SystemExit(f"Missing required TOML: {label}")

tomllib.loads(Path("rust-toolchain.toml").read_text(encoding="utf-8"))
tomllib.loads(Path("antares-agent/Cargo.toml").read_text(encoding="utf-8"))
print(f"Antares scaffold validation passed ({len(required)} required paths; JSON/TOML parsed).")
PY

for script in scripts/*.sh .devcontainer/antares/*.sh; do bash -n "$script"; done
python3 -m py_compile scripts/apply-overlay.py
rm -rf scripts/__pycache__
printf 'Shell/Python syntax checks passed.\n'
