# Workflow: Phase 0 source audit

1. Read all root Antares instructions and rules.
2. Verify repository parent, remotes, branch, status, and source SHA.
3. Inspect Lapce workspace packages, app startup, editor, panel, Floem, LSP, proxy, and plugin seams.
4. Inspect Octomind 0.72.0 release docs for ACP, sessions, process invocation, MCP injection, auth, and sandbox semantics.
5. Run baseline commands; record actual exit codes and missing dependencies.
6. Create `docs/PHASE-0-AUDIT.md` with file paths and verified API details.
7. Update status and stop. Do not implement P1 or UI in the audit task.
