# AGENTS.md — Antares IDE master engineering rules

You are implementing **Antares IDE** in a fork of `lapce/lapce`. This file is the source of truth for Antigravity, Gemini, and other coding agents. Read it in full before every task, then read `docs/STATUS.md` and the active phase of `docs/02-ROADMAP.md`.

## 1. Product ground truth

- **Lapce is the editor foundation.** Current upstream is a Rust workspace using edition 2024, Rust `1.98.0` MSRV, Floem UI, wgpu rendering, LSP, terminal, remote development, and WASI plugins. Verify current source before changing any API; never rely on memory of an older Lapce version.
- **Octomind is an external AI-agent runtime.** Pin a release, spawn it as a child process, and integrate through ACP over stdio first. Do not vendor or fork Octomind in this repository.
- **Antares is the integration layer.** Keep the editor responsive; keep Floem-specific code in `lapce-app/src/antares/`; keep transport, session, policy, context and patch models in independent `antares-*` crates.
- **Gemini 3.8 Flash** is the requested implementation model in Antigravity. This is separate from the model configured inside the shipped Antares IDE.
- The scaffold is not a finished IDE. Never claim a feature exists until its code, tests, and user-visible state are present.

## 2. Non-negotiable rules

1. **Inspect before editing.** Read the relevant upstream modules and nearby tests. Before UI work, inspect the current panel kind, panel registration, command, config, and theme patterns in Lapce.
2. **Small upstream diff.** New logic goes in `antares-*` crates or a dedicated `lapce-app/src/antares/` module. Patch upstream files minimally. Record every upstream-file change in `docs/PATCHES.md` with file, reason, and re-apply note.
3. **No guessed protocols.** ACP messages, session lifecycle, `_meta`, cancellation, capabilities, and Octomind configuration must be based on the pinned release docs/source. Keep protocol fixtures and contract tests. Do not invent WebSocket or ACP field names.
4. **No UI blocking.** Child process IO, parsing, MCP work, file scans and LSP/context collection run off the UI thread. Send bounded events to the UI. Batch token updates; target no more than 60 UI updates per second.
5. **No silent writes or command execution.** The initial profile is read-only. Expose only Antares-owned MCP tools in the first version. Changes are proposed as patches; the editor validates and displays a diff; only an explicit user action applies it. Commands require a separate, explicit run action. Do not claim Octomind's role allowlist is an OS sandbox.
6. **Layered safety.** Use role/tool allowlists, `auto_capabilities = false`, `.agents/guardrails.toml`, path canonicalization, size limits, a scoped MCP bridge, and OS sandboxing where supported. Verify effective runtime tools; an empty allowlist is not automatically proof of deny-all.
7. **Secrets stay out of Git.** Never commit API keys, access tokens, `.env`, private workspace content, or session logs. Never print secrets in diagnostics. Prefer the OS key store; in Codespaces use repository/user secrets.
8. **No fake success.** Unsupported functionality returns a typed error or remains disabled. No placeholder buttons that appear to work.
9. **Error handling.** Use typed errors. Avoid `unwrap()` / `expect()` in production code. Handle child exit, malformed messages, broken pipes, cancellation, timeouts, stale sessions, and restart paths.
10. **Keep dependencies intentional.** Check workspace dependencies and licenses before adding a crate. Add the smallest dependency set and explain why in the PR.
11. **No repo-wide formatting of upstream.** Format only Antares-owned packages/files. Do not run broad `cargo fmt` if it changes upstream files.
12. **Test headlessly in Codespaces.** Never try to open the native GUI in the Codespace. Use `fmt`, `clippy`, unit/integration tests, `cargo check`, and GitHub Actions. GUI/manual UX tests run on a desktop using a built artifact.
13. **Do not hide failed CI.** Never use `[skip ci]`, `continue-on-error` to mask failures, deleted tests, or relaxed lints as a shortcut.
14. **Never force-push `main`.** Work on `agy/<phase>-<topic>` branches. Push commits to the feature branch and use PRs or a verified fast-forward.

## 3. Architecture boundaries

### Lapce-owned
- editor buffers, undo/redo, LSP, syntax, workspace, terminal UI, panel registration, themes, command palette, proxy RPC.
- Antares Floem views and interaction wiring in `lapce-app/src/antares/`.

### Antares-owned crates (planned)
- `antares-agent`: Octomind binary resolution, process lifecycle, ACP JSON-RPC transport, session lifecycle.
- `antares-protocol`: typed ACP/Antares event boundary and versioned internal event types.
- `antares-context`: scoped workspace/editor context snapshots, redaction, size budgeting.
- `antares-mcp`: editor-hosted MCP tools for read-only context and edit/command proposals.
- `antares-diff`: patch parsing, path validation, preview model, accepted patch application contract.
- `antares-policy`: permission decisions, approval state, deny reasons, capability registry.
- `antares-settings`: settings schema, provider metadata, migrations and validation.

Create crates only when a phase needs them; do not create empty placeholder crates in P0.

### Required data path

`Lapce editor state` → `Antares context snapshot` → `Octomind ACP session` → `typed stream events` → `pure reducer/state` → `Floem view`.

For edits: `agent proposal` → `MCP proposal store` → `diff validation` → `user accepts` → `Lapce buffer/proxy apply` → `undo-capable buffer update`.

For commands: `agent command request` → `risk/policy check` → `human run action` → `terminal/proxy execution` → bounded output result.

## 4. ACP process contract

- Primary integration: launch `octomind acp` with a project-scoped working directory and a stable session name; use the ACP methods supported by the pinned Octomind docs.
- Keep stdout exclusively for JSON-RPC. Drain stderr independently to a bounded diagnostic log. Never parse user-visible debug text as protocol data.
- Use one serialized writer and one reader task; do not block either on UI work. Apply message-size limits and backpressure.
- Implement initialize → new/load session → prompt/update stream → cancel → graceful shutdown. Preserve returned session IDs and only resume when the protocol reports a valid existing session.
- Capture child PID, exit status, startup timeout, protocol errors and restart reason. Never spawn duplicate sidecars for one session.
- ACP `_meta` fields are extensions; preserve unknown values safely and only interpret keys documented by the pinned runtime.
- WebSocket is not part of the first milestone. Add it only behind the same transport abstraction and only for a specific remote/daemon use case.

## 5. AI tools and safety boundary

- P0/P1 default is read-only. The first MCP bridge should provide only bounded operations such as list/search/read active context and diagnostics.
- The bridge may accept `propose_patch` and `request_command`, but these operations create pending requests; they must not write files or execute commands.
- Never expose raw filesystem-write or arbitrary shell capabilities to the initial agent role. Verify the effective tool list at runtime after each Octomind upgrade.
- Guardrails are a backstop, not the only permission system. Consult the pinned Octomind `doc/usage/06-roles.md`, `doc/usage/07-mcp-tools.md`, and `doc/usage/18-guardrails.md` before changing policy.
- For workspace paths: canonicalize, verify containment under workspace root, reject traversal and symlink escapes, check file type/size, and revalidate immediately before applying a patch.
- Render agent text as text/Markdown with safe links; never interpret agent output as raw HTML or automatically execute code blocks.

## 6. Performance rules

- Never perform disk reads, process IO, LLM calls, diff generation, or project scans synchronously in a Floem view callback.
- Debounce search/context refresh. Cache file metadata and invalidate via existing workspace/file watcher events.
- Bound chat history, logs, event queues and tool output. Do not rebuild all transcript nodes on every token; append to the active message.
- Cancel stale tasks when workspace/session changes. Avoid holding Lapce buffer locks while waiting for sidecar IO.
- Define and measure: editor typing latency, scroll frame time, ACP event-to-render delay, memory with idle agent, and cancellation latency.

## 7. Required task loop

1. `git status --short`; read status/roadmap; identify one task.
2. Write a plan of no more than 10 lines in the task response.
3. Inspect exact upstream APIs and record sources/decisions.
4. Add or update a failing test first where practical; implement one vertical slice.
5. Run `make -f Makefile.antares antares-gate`, `git diff --check`, and relevant workspace checks.
6. Review `git diff` for unrelated upstream churn, secrets, generated files and placeholder behavior.
7. Commit with Conventional Commits; push the feature branch; watch the matching GitHub Actions run.
8. On green CI, update `docs/STATUS.md` and `docs/PATCHES.md` if required. Commit docs separately when useful.
9. On red CI, follow `.agent/workflows/fix-ci.md`. After five genuine root-cause attempts, stop and record a blocker instead of looping blindly.

## 8. Commit convention

`type(scope): imperative summary` — types: `feat`, `fix`, `refactor`, `test`, `docs`, `ci`, `build`, `chore`; scopes: `agent`, `protocol`, `context`, `mcp`, `diff`, `policy`, `ui`, `ci`, `deps`, `rebrand`. Keep the subject concise (about 72 characters or fewer).

## 9. Definition of Done

A task is complete only when its acceptance criteria are met; implementation is covered by appropriate tests; relevant checks pass; the UI has honest loading/empty/error/cancel states where applicable; docs/status are updated; and CI for the commit is green. A green compile alone is not proof that a feature works.

## 10. Final report format

- **Changed files:** exact paths.
- **Behavior:** what now works and what remains disabled.
- **Tests:** exact commands and pass/fail results.
- **CI:** GitHub Actions run URL or explicit “not run”.
- **Risks/blockers:** only concrete, reproducible items.
- **Next task:** one roadmap item.
