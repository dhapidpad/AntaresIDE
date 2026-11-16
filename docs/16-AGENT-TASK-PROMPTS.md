# Antares IDE — staged prompts for Gemini 3.8 Flash

Use these prompts one at a time in Antigravity. The Master Prompt (`docs/12-AGENT-MASTER-PROMPT.md`) remains the highest-level contract.

## Prompt A — Phase 0 audit only

```text
Read GEMINI.md, GEMINI.antares.md, AGENTS.md, AGENTS.antares.md, all .agent/rules/, docs/12-AGENT-MASTER-PROMPT.md, docs/STATUS.md, docs/02-ROADMAP.md, docs/DECISIONS.md, docs/07-SECURITY-MODEL.md, and docs/10-TEST-PLAN.md.

Execute Phase 0 from docs/12-AGENT-MASTER-PROMPT.md only. Inspect the checked-out Lapce source and the pinned Octomind 0.72.0 ACP documentation. Do not implement UI, process manager, or MCP features yet. Run the baseline diagnostics and tests. Create docs/PHASE-0-AUDIT.md with concrete file paths, verified APIs, exact versions, command outputs, architecture seams, risks, and a P0 plan. Update docs/STATUS.md only with facts established by the audit. Stop after the audit and report any tests not run.
```

## Prompt B — P0 foundation implementation

```text
Read the Master Prompt and current Phase 0 audit. Implement only P0. Confirm the workspace/package metadata and keep Antares code isolated. Make antares-agent a clean, documented Rust library boundary with typed configuration, lifecycle states, errors, and headless tests only where the Phase 0 audit proves they are needed. Do not add an unverified ACP dependency and do not start Octomind from the UI yet. Ensure the fork's upstream files are preserved, Cargo.lock is resolved, and Antares CI passes. Run fmt/check/test/clippy, scaffold validation, and diff checks. Update docs/STATUS.md, docs/DECISIONS.md, and docs/PATCHES.md with the exact result.
```

## Prompt C — P1 Octomind sidecar and ACP

```text
Implement P1 only, using the exact ACP contract of Octomind 0.72.0. First reread the pinned protocol docs and verify every method/field against the release. Add a testable process supervisor with stdout JSON-RPC framing, stderr diagnostics, bounded channels, request correlation, cancellation, timeout, shutdown, child-exit handling, and recoverable session state. Build fake ACP child fixtures so CI requires no provider key and no network. Add explicit sandbox launch options and safe executable discovery. Do not implement patch writes or unrestricted shell tools. Do not add a decorative UI before lifecycle tests pass. Run targeted tests and update the docs.
```

## Prompt D — P2 context builder and read-only workspace bridge

```text
Implement P2 only. Inspect Lapce workspace, editor selection, document, diagnostics, and file APIs. Build a bounded context assembly service and host-owned read-only tools. Canonicalize paths, reject traversal and symlink escapes, respect ignore files, exclude secrets/private files/binaries/build outputs, cap line/file/token sizes, and show the user what context is sent. Add unit tests for path containment, ignore behavior, binary files, oversized files, selection snapshots, and context budget. No file-write tool and no auto-indexing of the entire workspace.
```

## Prompt E — P3 patch proposals and approval

```text
Implement P3 only. Create a proposal data model with session/proposal IDs, relative paths, base hashes, patch content, rationale, and validation status. Parse unified diffs safely. Reject absolute paths, traversal, symlink escape, protected/secret paths, malformed diffs, stale hashes, and concurrent edits. Display proposals as reviewable diffs. Reject/close must leave buffers and disk untouched. Approval must be an explicit human action and apply through Lapce editor APIs so undo/redo and dirty state work. Add rollback or explicit recoverable transaction behavior for multi-file failure. Add security tests before adding more features.
```

## Prompt F — P4 native AI panel

```text
Implement P4 only. Use the exact Floem and Lapce UI patterns discovered in Phase 0. Add a native Antares AI panel with sessions, streaming conversation, composer, send/cancel, tool-call progress cards, context summary, errors/retry, settings, and patch-review navigation. All state transitions must be driven by tested domain state; no fake buttons or mock agent responses in release code. No blocking process/model/filesystem work on the UI thread. Add empty/loading/error/cancelled/reconnecting states, keyboard focus/accessibility, theme support, long-history bounds, and panel persistence. Run tests and a manual UI checklist.
```

## Prompt G — P5 hardening and release

```text
Implement P5 stabilization only. Add repeated start/stop, large output, cancellation, child crash, multi-session isolation, stale notification, memory bound, and shutdown tests. Profile startup, typing latency, AI panel updates, and memory before optimizing. Verify Linux/Windows/macOS process launching and Octomind installation paths. Build desktop artifacts using GitHub Actions. Review third-party notices, artifact metadata, failure reporting, and upgrade procedure. Do not claim signed/notarized installers unless implemented. Update release documentation and docs/STATUS.md.
```

## Prompt H — CI failure repair

```text
Read the failed GitHub Actions job log and reproduce the failing command in the Codespace. Classify the failure as toolchain, dependency resolution, source compile, test, lint, native system package, or workflow config. Fix the smallest root cause. Do not weaken tests, remove clippy warnings globally, or mark failing jobs as continue-on-error. Rerun the exact failing command, then the affected Antares gate and git diff --check. Report the original failure, root cause, patch, and actual verification evidence.
```

## Prompt I — safe upstream sync

```text
Run scripts/antares-update-upstream.sh and review its output. Do not merge or rebase automatically. Read docs/PATCHES.md, fetch the explicit upstream base, inspect overlapping changes, create a dedicated sync branch, and report the proposed merge plan before changing source. Preserve all Antares-specific code and keep the upstream license/history intact. Run Antares crate tests and relevant Lapce/workspace tests after the sync.
```
