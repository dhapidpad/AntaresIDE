# Test plan

## P0 bootstrap

- `cargo metadata --no-deps --format-version 1` succeeds.
- `cargo fmt -p antares-agent -- --check` succeeds.
- `cargo clippy -p antares-agent --all-targets -- -D warnings` succeeds.
- `cargo test -p antares-agent` succeeds.
- `cargo check --workspace --locked` succeeds in GitHub Actions.

## P1 ACP/process tests

Use a deterministic fake child process; CI must not call a real model or require API secrets.

- Binary missing / invalid path.
- Child starts, initializes and exits cleanly.
- Stdout JSON-RPC frames split across reads and multiple frames in one read.
- Malformed JSON, unexpected method, unknown extension metadata.
- Large frames and bounded queue backpressure.
- Stderr flood does not block stdout or memory growth.
- Child exits unexpectedly, closes pipe, startup timeout, response timeout.
- Session create/load, prompt, update notifications and cancel.
- Graceful shutdown drains stdout to EOF.

## P2 UI state tests

- Every state transition is tested without a native window.
- Empty, loading, ready, streaming, cancelling, auth-required, disconnected and error states render honestly.
- Cancel and workspace switch cancel stale work.
- Long transcript updates do not rebuild all existing messages.

## P3 context tests

- Valid workspace file accepted.
- Parent traversal and absolute path rejected.
- Symlink escape rejected.
- `.env`, keys, credential files and ignored paths excluded.
- Large file truncation is explicit and bounded.
- Secret redaction marks what was removed.
- Workspace switch invalidates previous context.

## P4 MCP/diff tests

- MCP tool input schema rejects malformed arguments.
- Proposal tool creates no filesystem side effect.
- Patch outside root, stale base hash, symlink escape and unsupported file type rejected.
- Accept applies exactly the accepted hunk(s); reject applies nothing.
- Apply operation is undoable.
- Command request is pending only; no process starts until explicit UI action.
- Unknown tools or capability surfaces fail closed.

## P5 performance checks

Record baseline vs Antares-enabled results for typing latency, scroll frame time, idle memory, event queue depth, context build time and cancellation. Set numeric budgets from measured Lapce baseline before claiming a performance improvement.

## Desktop smoke tests

Codespaces does not provide a GUI. Download a CI/release artifact and test on actual Windows/macOS/Linux desktop systems: first launch, window rendering, GPU fallback, keyboard input, editor, LSP, terminal, AI session, disconnect/reconnect, diff accept/reject and clean shutdown.
