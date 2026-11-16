# Quality and performance

- Keep editor typing, completion and scrolling independent of agent response latency.
- Use bounded channels, debounce, cancellation tokens and explicit backpressure. Do not spawn unbounded tasks.
- Track startup time, RSS, idle CPU, context-build duration, ACP latency, event queue depth and cancel latency.
- Test malformed JSON-RPC, sidecar crash, stderr flood, stdout EOF, timeout, cancel, resume, workspace switch and oversized output.
- For visual changes, add a small manual smoke-test checklist; Codespaces cannot display the native window.
