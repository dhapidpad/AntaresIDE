# Security model and trust boundaries

## Assets to protect

- Source files and uncommitted work.
- Credentials, `.env` files, SSH keys, cloud credentials and provider tokens.
- User approval decisions and command execution context.
- Octomind session history, prompts and local workspace metadata.

## Trust boundaries

1. **LLM output is untrusted.** Treat all model text, tool arguments, file paths and suggested commands as hostile input.
2. **Octomind is a powerful external process.** It can access tools enabled for its session. Do not infer safety from the fact that it is launched by Antares.
3. **ACP is a protocol boundary, not a sandbox.** Validate JSON-RPC framing, size, method and session IDs.
4. **MCP bridge is a capability boundary.** Expose minimum tools; validate every argument; do not expose a generic filesystem root or arbitrary shell.
5. **Lapce apply is the write boundary.** Revalidate paths and revisions and require an explicit accept action for patches.

## Default profile

- Read-only workspace context tools only.
- No raw filesystem-write or arbitrary shell capability.
- `auto_capabilities = false` in Antares-owned Octomind settings.
- Guardrails block known write/exec capabilities as defense in depth; verify exact capability names against runtime.
- No background commands. A command request is a visible pending item; execution requires a separate user action.
- No automatic application of generated patches.

## Path and patch validation

- Canonical workspace root and target path.
- Reject absolute paths, parent traversal, symlink escape, special files and unsupported encodings.
- Check the path is inside the canonical root before reading and again immediately before writing.
- Enforce file, hunk, total-patch and context size budgets.
- Check base revision/hash before application; stale proposals must be rejected or regenerated.
- Preserve buffer undo/redo semantics.

## Secrets and privacy

- Exclude `.env`, `.env.*`, key/certificate files, credential stores, build folders, VCS internals and ignored files from context by default.
- Redact likely tokens/secrets before provider calls; make redaction visible in context metadata.
- Store keys in OS secure storage or environment variables. Never persist them in JSON/TOML repo config.
- Do not log prompts, source content, environment values or keys at info level.
- Add no analytics or telemetry by default. If an upstream component has independent telemetry controls, verify and disable them where supported.

## Fail-closed behavior

If the runtime reports unknown tools, policy config fails validation, context path checks fail, or the sidecar protocol is out of contract, disable the affected operation and show a readable reason. Do not silently fall back to direct write/exec.
