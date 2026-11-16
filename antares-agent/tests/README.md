# Antares agent integration-test layout

P0 currently keeps deterministic unit tests beside `src/lib.rs`; no ACP process, network provider, or API key is required.

Add integration tests here only when the corresponding implementation phase lands:

- `acp/`: JSON-RPC framing, request correlation, session lifecycle, cancellation, shutdown and malformed/unknown messages.
- `process/`: executable discovery, child lifecycle, stdout/stderr separation, restart and timeout behavior.
- `security/`: fail-closed policy, path containment, redaction, tool allowlists and patch safety.
- `fixtures/acp/`: sanitized JSONL request/notification fixtures pinned to Octomind 0.72.0; never store credentials or live user conversations.
- `fixtures/fake-agent/`: deterministic fake child process used by CI; tests must not call a live model.

Every new integration test must document its expected invariant and reproduce the bug/contract it protects. Keep tests deterministic and bounded. Avoid sleep-based synchronization when a pipe, channel, or explicit test barrier can be used.
