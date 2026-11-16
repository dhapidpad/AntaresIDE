# Architecture — Lapce host + Antares services + Octomind

## High-level diagram

```text
┌──────────────────────────── Antares Desktop ───────────────────────────┐
│ Lapce app (Rust + Floem + wgpu)                                        │
│  ├─ Existing editor, LSP, buffers, undo, terminal, remote, WASI         │
│  └─ lapce-app/src/antares/                                             │
│       ├─ AI panel / sessions / context picker                          │
│       ├─ tool activity / command request / approval UI                 │
│       ├─ diff review / hunk accept / settings                          │
│       └─ Floem adapter only                                            │
│                  │ bounded typed events                                │
│     antares-* crates (no Floem dependency)                             │
│       ├─ agent process + ACP transport                                  │
│       ├─ session/event model                                            │
│       ├─ context snapshot + secret redaction                            │
│       ├─ MCP bridge: read context + create proposals                    │
│       ├─ diff validation + policy                                      │
│       └─ settings + persistence                                         │
│                  │ ACP JSON-RPC over stdio                              │
│       ┌──────────▼──────────┐                                           │
│       │ Octomind 0.72.0     │ external, pinned process                  │
│       │ supervisor + roles  │                                           │
│       └──────────┬──────────┘                                           │
│                  │ model/provider, MCP clients                          │
│       Google AI Studio / OctoHub / OpenAI / other supported providers    │
└────────────────────────────────────────────────────────────────────────┘
```

## Key design decisions

### D1 — Fork Lapce, do not recreate an editor

Lapce already supplies the native editor, LSP, remote development, terminal and plugin foundation. Antares should preserve that behavior and add features in a narrow integration layer. Avoid broad rebrand changes until the integration is stable.

### D2 — Octomind is an external process, not a linked crate

Pin the sidecar release in `config-templates/octomind.lock`. Resolve and verify the platform archive before execution. Keep data/config/session files in an Antares-owned directory. Do not modify a user's independent Octomind config.

### D3 — ACP over stdio is the primary transport

Octomind's ACP integration is designed for editor hosts and uses JSON-RPC over stdio. The initial `AgentTransport` implementation should spawn `octomind acp`, initialize capabilities, create/load a session, stream `session/update`, prompt and cancel. Stdout is protocol-only; stderr is diagnostic-only. WebSocket is deferred until remote or daemon use is required.

### D4 — Host-owned MCP bridge

The editor injects a narrowly scoped `antares-workspace` MCP server into the ACP session. Initial tools: list/search/read bounded workspace content, read active selection/diagnostics, and create a patch proposal. Later: create a command request and manage optional MCP servers. Do not grant raw writes or arbitrary shell to the initial role.

### D5 — Proposals are not writes

An AI tool call may create a pending proposal record, but it must not mutate the workspace. Antares validates the path, base revision and patch; shows a diff; then applies only after a user acceptance action through Lapce's buffer/proxy contract. Keep undo support.

### D6 — UI reducer is headless-testable

Each panel uses domain state + events + reducer, then a Floem adapter. Tests should exercise state transitions without creating a native window.

## Process lifecycle

1. Resolve explicit configured Octomind path; otherwise locate the pinned installation in Antares app data.
2. Verify binary version against lockfile and archive digest at installation time.
3. Create an isolated `OCTOMIND_DATA_DIR`; do not reuse a global user data directory.
4. Spawn `octomind acp` with a project working directory; pipe stdin/stdout/stderr.
5. Initialize ACP; handle authentication-required states without leaking provider credentials.
6. Create or load the session with the exact workspace CWD and injected MCP server config supported by the pinned Octomind release.
7. Stream notifications through a bounded queue to a pure state reducer.
8. Cancellation uses ACP `session/cancel`; shutdown closes stdin and drains stdout to EOF per Octomind docs.
9. On child crash: preserve visible error/session ID; do not silently re-run a potentially mutating turn. Allow the user to reconnect/resume.

## Editor context contract

A context snapshot contains:
- workspace root ID and canonical path;
- active file URI and language identifier;
- selected text and exact range, if any;
- diagnostics with source/range/severity;
- requested extra files/resources;
- snapshot timestamp and base document revision/hash;
- redaction and budget metadata.

Context is scoped to the active workspace, filtered by ignore rules, capped by bytes/tokens, and redacted before it leaves the machine. Never attach `.env`, private keys, credential files, build outputs or ignored folders by default.

## Patch proposal contract

Each proposal contains a unique ID, session ID, workspace ID, normalized relative paths, base revision/hash, unified or structured hunks, creation timestamp, and validation results. Reject absolute paths, `..`, symlink escapes, unsupported file types, oversized changes, stale base revisions and writes outside the current workspace. The UI must distinguish pending, accepted, rejected, stale and failed.

## State ownership

- Lapce owns editor buffers, file watching, LSP, terminal and undo.
- `antares-agent` owns process lifecycle and ACP framing.
- `antares-context` owns safe snapshots and redaction.
- `antares-mcp` owns host-provided tools and proposal request ingress.
- `antares-diff` owns validation and diff models.
- `antares-policy` owns permissions and decision reasons.
- Floem views display state; they do not own protocol or filesystem logic.

## Open questions to resolve from source before coding

- Exact ACP version/capability payload accepted by pinned Octomind version.
- Exact `session/new` MCP injection structure and how the injected server process is launched by the ACP host.
- Exact `ToolCall`/`ToolCallUpdate` event mapping and cost metadata fields.
- How to map Lapce buffer revisions and accepted patches into proxy/buffer APIs without losing undo history.
- Effective Octomind default and role tools after automatic capability activation is disabled.

Record evidence and file paths in `docs/DECISIONS.md`; do not solve these by guessing.
