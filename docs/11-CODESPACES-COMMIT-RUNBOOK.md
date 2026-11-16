# Codespaces + GitHub commit/test runbook

## Preferred one-time setup

Run the automated workflow described in `docs/13-ONE-CLICK-SETUP.md` from the extracted starter folder:

```bash
gh auth login
gh auth status
bash scripts/one-click-setup.sh --create-codespace
```

This creates/reuses a verified fork named `AntaresIDE`, applies the Antares overlay without overwriting upstream CI/devcontainer/license files, creates `agy/antares-bootstrap`, runs available checks, commits and pushes, then asks before creating a Codespace. For what it changes and the safety switches, read the complete one-click setup guide.

## First Codespace commands

The Antares devcontainer is `.devcontainer/antares/devcontainer.json`; do not replace Lapce's upstream `.devcontainer/devcontainer.json`.

After the container finishes its post-create setup:

```bash
bash scripts/antares-doctor.sh
bash scripts/scaffold-check.sh
bash scripts/antares-ci.sh fast
git status --short
```

The first cargo check may update `Cargo.lock` for the new workspace member. Inspect it and commit only the lockfile change after review:

```bash
git diff -- Cargo.lock
git add Cargo.lock
git commit -m "chore: lock Antares bootstrap dependencies"
git push
```

Then use the locked checks:

```bash
bash scripts/antares-ci.sh locked
```

Run the full Lapce workspace check only when a task touches upstream editor crates or when explicitly validating the full build:

```bash
bash scripts/antares-ci.sh full
```

## GitHub Actions

```bash
gh run list --branch "$(git branch --show-current)" --limit 10
```

Open the failing job, copy the first real compiler/test error, and reproduce it in Codespaces. Never delete tests or weaken CI just to get green.

## Antigravity task loop

1. Open the same fork branch in Antigravity and select Gemini 3.8 Flash.
2. Read `AGENTS.md`, `AGENTS.antares.md`, `GEMINI.md`, `GEMINI.antares.md`, `.agent/rules/`, and the current roadmap/status.
3. Paste `docs/12-AGENT-MASTER-PROMPT.md`; execute Phase 0 audit only.
4. Keep local Antigravity and Codespaces clones on the same branch, committing/pulling before switching environments.
5. Give the agent one bounded phase at a time. Run tests in Codespaces and inspect GitHub Actions.

## Desktop UI reality

Codespaces is a headless Linux development environment. Compile and test Rust code there; do not assume a native Floem window can be shown in the browser. Download the CI artifact and smoke-test it on a supported desktop OS. The forwarded port preview is not a substitute for a native desktop GUI.
