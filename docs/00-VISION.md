# Vision — Antares IDE

Antares IDE is a native Rust code editor with an editor-aware AI development partner.

## Product formula

**Lapce editor foundation + Antares integration layer + Octomind agent runtime.**

Lapce remains responsible for editing, buffers/undo, LSP, syntax, terminal, remote workspace support, and the Floem UI foundation. Antares adds a polished agent experience and controlled workspace tools. Octomind provides planning, model routing, tools, specialist roles, sessions, supervisor, skills and MCP integration as a separately versioned process.

## User-visible goals

- Familiar native editor: file explorer, tabs/splits, code navigation, diagnostics, completion, search, terminal, remote workspace and extensions inherited from Lapce.
- AI workspace: chat, streaming progress, session list/resume, context attachments, specialist selection, usage details and cancellation.
- Editor-aware actions: explain selection, diagnose errors, propose patch, compare hunks, accept/reject, and preserve undo.
- Controlled tools: scoped MCP context, proposal queue, explicit command requests, visible policy denials, and no background command execution without user action.
- Reliable development: reproducible build, CI checks, release artifacts, crash/timeout handling and clear logs.

## Principles

1. Native editor performance comes first; agent latency must not affect typing or scrolling.
2. User intent and control come first; proposed edits and command requests are visible.
3. Use any supported Octomind model/provider; the implementation model in Antigravity is separate from in-app configuration.
4. Keep the Lapce fork maintainable with small, documented upstream patches.
5. Local-first defaults: no analytics added by Antares; no prompt/file/key logging by default.
6. Evidence over claims: a feature is complete only after tests and an actual user-flow smoke test.

## Explicit non-goals for the first release

- Training or hosting a foundation model.
- Replacing Octomind's agent runtime or cloning all of its internals.
- Collaborative cloud workspaces or account synchronization.
- VS Code extension-host compatibility.
- Automatically running unreviewed agent-generated shell commands.

## Definition of a full first product

All existing Lapce core editing workflows remain intact. Antares adds ACP session management, a right-side AI panel, context injection, streaming status, read-only MCP bridge, patch proposal/preview/apply with undo, command request/approval, guardrail visibility, provider settings, session persistence and release packaging. Advanced specialist/tap/MCP administration is delivered only after core session and safety flows are stable.
