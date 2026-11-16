# Antares IDE

**Antares IDE** is a long-term native Rust IDE project built on the Lapce editor foundation, with Octomind used as an independently versioned AI-agent runtime.

This repository is created by forking `lapce/lapce` and then applying an Antares-owned overlay. The overlay is deliberately isolated: it does not replace Lapce's original Cargo packages, license, devcontainer, or existing GitHub workflows.

## One-command setup

### Requirements

- A GitHub account with permission to create a fork.
- GitHub CLI (`gh`) installed and authenticated (`gh auth login`).
- Git, Python 3.11+, and network access. On Windows, run the Bash script from Git Bash or WSL; PowerShell does not execute this `.sh` file directly.
- Rust/Cargo is recommended. If absent, GitHub Actions performs the first Antares crate checks and the Codespace installs the pinned toolchain.

### Run from the extracted starter folder

Extract this starter ZIP into a clean folder, open a terminal in that folder, then run:

```bash
 gh auth status
 bash scripts/one-click-setup.sh --create-codespace
```

The script:

1. Authenticates through `gh` and discovers the signed-in GitHub account.
2. Creates or safely reuses a fork named `AntaresIDE` from `lapce/lapce`.
3. Clones that fork and adds `upstream=https://github.com/lapce/lapce.git`.
4. Creates or switches to `agy/antares-bootstrap`.
5. Applies the Antares overlay without replacing upstream CI, README history, license, or devcontainer.
6. Integrates `antares-agent` into the existing Cargo workspace and Makefile idempotently.
7. Runs static checks, and Rust checks if Cargo is installed.
8. Creates a local commit and pushes it to your fork.
9. Asks before creating a Codespace. Codespaces may consume included quota or incur charges beyond the allowance; the script never bypasses this confirmation.

To prepare the repo without pushing, use `--no-push`. To apply and stage only, use `--no-commit`.

### Create the Codespace later

```bash
gh codespace create \
  --repo YOUR_GITHUB_LOGIN/AntaresIDE \
  --branch agy/antares-bootstrap \
  --devcontainer-path .devcontainer/antares/devcontainer.json \
  --display-name "Antares IDE"
```

Choose a machine size that is available to your account and check your Codespaces allowance before confirming. The configured development container uses a Rust image with the native Linux packages Lapce documents for building.

## First Codespace commands

The Antares post-create script installs the pinned Rust toolchain and Octomind sidecar, validates the scaffold, resolves `Cargo.lock`, and runs the fast Antares checks. When the Codespace is ready:

```bash
bash scripts/antares-doctor.sh
bash scripts/scaffold-check.sh
bash scripts/antares-ci.sh fast
git status --short
```

If Cargo changed `Cargo.lock`, inspect it and commit the lockfile:

```bash
git diff -- Cargo.lock
git add Cargo.lock
git commit -m "chore: lock Antares bootstrap dependencies"
git push
```

Then inspect the GitHub Actions run:

```bash
gh run list --branch "$(git branch --show-current)" --limit 10
```

Run the full Lapce + Antares workspace checks only when needed:

```bash
bash scripts/antares-ci.sh full
```

## Antigravity / Gemini 3.8 Flash

Open the fork's working branch in Antigravity and select **Gemini 3.8 Flash**. Start with:

- `GEMINI.md`
- `AGENTS.md` and `AGENTS.antares.md`
- `.agent/rules/`
- `docs/12-AGENT-MASTER-PROMPT.md`
- `docs/STATUS.md` and `docs/02-ROADMAP.md`

Paste `docs/12-AGENT-MASTER-PROMPT.md` as the master instruction. Then ask the agent to execute **Phase 0 only** and return its audit before asking it to implement Phase 1. This prevents a fast model from attempting a huge, unverified rewrite in one pass.

Antigravity and Codespaces can be separate environments: use Antigravity for code changes and agent orchestration, and use the fork/Codespace for reproducible Linux builds, tests, and GitHub Actions. Keep both on the same branch and commit/push deliberately.

## Architecture boundary

- **Lapce remains the editor core:** Rust, Floem, wgpu, LSP, terminal, proxy/remote support, and existing editor systems.
- **Antares-owned code stays in separate modules/crates:** AI panel, ACP process manager, editor/agent bridge, context builder, patch review, settings, and tests.
- **Octomind runs as a child process:** ACP JSON-RPC over stdio is the first integration transport. Do not vendor all Octomind source into Lapce.
- **Antares owns patch application:** the model proposes a patch; the IDE validates and shows the diff; a person approves; only then does the editor apply it.
- **Codespaces is headless build/test infrastructure:** compiling a native GUI there does not automatically display the desktop window. Publish desktop artifacts through GitHub Actions and run them on a supported desktop OS.

## Keep upstream healthy

```bash
bash scripts/antares-update-upstream.sh
git status --short
```

The helper only fetches and reports divergence. It never merges or rebases automatically. Review `docs/PATCHES.md` and `docs/08-UPSTREAM-MAINTENANCE.md` before syncing upstream.

## Current status

The starter is an integration scaffold, not a finished IDE. It does not claim that the AI panel, ACP client, patch UI, or desktop product is already implemented. The agent must progress through the roadmap, run the tests, and update `docs/STATUS.md` with evidence.

## Upstream and licensing

- Lapce source: https://github.com/lapce/lapce — Apache-2.0.
- Octomind source/releases: https://github.com/Muvon/octomind — Apache-2.0. Octomind is installed as a pinned sidecar, not copied wholesale into this repository.

Preserve the fork's `LICENSE`, upstream copyright headers, and any applicable `NOTICE` files. See `THIRD_PARTY_LICENSES.md` and `NOTICE-ANTARES.md` before redistributing binaries.
