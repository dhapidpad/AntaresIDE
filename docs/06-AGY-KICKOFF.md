# Antigravity kickoff — Gemini 3.8 Flash

Use this as a concise entrypoint. The complete instructions are in `docs/12-AGENT-MASTER-PROMPT.md`.

## Before any code edits

1. Read `GEMINI.md`, `GEMINI.antares.md`, `AGENTS.md`, `AGENTS.antares.md`, all `.agent/rules/`, `docs/STATUS.md`, `docs/02-ROADMAP.md`, `docs/DECISIONS.md`, `docs/07-SECURITY-MODEL.md`, and `docs/10-TEST-PLAN.md`.
2. Run `git status --short --branch`, `bash scripts/antares-doctor.sh`, and `bash scripts/scaffold-check.sh`.
3. Verify that the current GitHub repository is a fork of `lapce/lapce`, with `origin` on the user's fork and `upstream` on Lapce.
4. Inspect the exact Lapce/Floem APIs and Octomind 0.72.0 ACP docs before relying on any API.
5. Keep Codespaces headless: use it for builds/tests, not as proof that the native GUI works.

## First task — Phase 0 audit only

Paste `docs/12-AGENT-MASTER-PROMPT.md` and complete only its Phase 0 instructions:

- Map the actual Lapce crate/module responsibilities and editor extension points.
- Verify the pinned Octomind ACP contract and process invocation details.
- Run baseline static/build/test commands and record real outcomes.
- Create `docs/PHASE-0-AUDIT.md` with exact files, versions, risks, and a P0 plan.
- Update `docs/STATUS.md` with evidence.

**Stop after the audit. Do not build an AI panel, ACP implementation, MCP bridge, or patch writer in this first task.** Wait for the user to approve Phase 1.

## Constraints

- Lapce remains the editor engine; keep Antares-specific logic isolated.
- Octomind remains an external sidecar; do not vendor its entire source.
- AI changes are proposals only. No direct agent file writes or arbitrary shell by default.
- Every write/command requires host-side validation and explicit user approval.
- Do not replace upstream Cargo metadata, devcontainer, workflows, README history, or license without a reviewed migration plan.
- Do not mark a test as passed unless it actually ran and exited successfully.
