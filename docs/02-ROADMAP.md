# Roadmap — one verified vertical slice at a time

Every phase ends with: tests pass, CI green for the phase commit, `docs/STATUS.md` updated, and acceptance evidence recorded. A phase can be split into smaller PRs.

## P0 — Foundation and CI

- Fork Lapce; apply this scaffold; add `antares-agent` to the existing workspace.
- Verify workspace metadata, lockfile, formatting, Clippy, crate tests, and full `cargo check --workspace` in Actions.
- Keep all upstream behavior unchanged.

**Exit:** first green CI; no Antares feature falsely represented as implemented.

## P1 — Octomind installation and ACP process

- Read Octomind 0.72.0 ACP, session, provider, role, MCP and environment-variable docs.
- Implement verified installer using pinned archive + SHA-256 for supported platforms.
- Isolate Octomind data/config/session storage under Antares app data.
- Implement child process supervisor, JSON-RPC framing, serialized writer, independent stdout/stderr readers, timeouts, EOF/exit handling and graceful shutdown.
- Contract tests with a deterministic mock ACP child process. Do not require an LLM or secret in CI.

**Exit:** headless test can initialize, create a mock session, stream updates, cancel and shut down cleanly.

## P2 — AI panel and session basics

- Add Antares module to Lapce using existing panel/command registration patterns.
- Build reducer/state for idle, starting, ready, streaming, cancelling, auth-required, disconnected and error.
- Render message stream, tool activity, cancel, session selector and session error states.
- Keep Floem code thin; add headless reducer tests.

**Exit:** real Octomind ACP session can answer a text-only prompt from a desktop build; no file write tools enabled.

## P3 — Editor context

- Read active file, selected range and diagnostics through Lapce-owned APIs.
- Add explicit context picker and `@file` resource references.
- Add canonical path checks, ignore rules, redaction and token/byte budgets.
- Add fixtures for secret files, symlink escapes, huge files and workspace switching.

**Exit:** user can ask about selected code/current file and see exactly which context was attached.

## P4 — MCP bridge and controlled changes

- Implement host-owned `antares-workspace` MCP server.
- Expose read/search/context tools and patch-proposal ingress only.
- Parse/validate proposal, show unified diff, accept/reject by file/hunk, revalidate base revision, apply through Lapce buffer/proxy with undo.
- Add command request as a separate pending action; command runs only after explicit user interaction.
- Confirm effective Octomind role/tool permissions after startup and fail closed on unknown tool surfaces.

**Exit:** no direct agent write/exec path in default profile; accepted edits are undoable; rejected requests have no workspace side effects.

## P5 — Agent capabilities

- Session persistence/resume, specialist/role picker, provider configuration, budget/usage view, model selection, MCP server management and background task panel.
- Add capability-level policy explanations and safe enable/disable states.
- Each feature is enabled only if the pinned runtime exposes a documented and tested API.

**Exit:** each advertised control works end-to-end or is hidden/disabled with explanation.

## P6 — UI and performance hardening

- Refine visual system, keyboard navigation, focus behavior, panel resizing, empty/loading/error states and accessibility.
- Add instrumentation for typing latency, scroll frame time, event queue depth, idle memory, context build time and ACP cancellation latency.
- Optimize streaming and long transcripts; add stress tests.

**Exit:** no measurable typing/scroll regression in defined manual benchmark; no unbounded queue/task growth.

## P7 — Antares branding and release

- Dedicated app name, IDs, icons, defaults, URLs, about dialog and updater/release metadata.
- Build binaries for Linux, Windows and macOS; include correct third-party notices.
- Add installation/first-run screens and Octomind setup flow.
- Do not rename upstream targets until every build/package script has been updated and tested.

**Exit:** desktop artifacts install and launch on all supported OS targets; first-run can detect/configure the agent.

## P8 — Reliability and public release readiness

- Crash recovery, interrupted-turn behavior, logs/privacy controls, security audit, dependency/license review, upstream sync dry run, upgrade/migration tests and release checklist.
- Validate sidecar archive digest and version on every supported architecture.
- Review threat model and documented limitations.

**Exit:** reproducible CI, platform smoke tests, release notes, notices and upgrade path.
