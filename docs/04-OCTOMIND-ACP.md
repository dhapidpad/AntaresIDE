# Octomind integration contract

## Verified upstream surface

The current Octomind repository documents an ACP mode launched as `octomind acp`. ACP uses JSON-RPC over stdio and supports initialization, session creation/loading, prompt streaming, cancellation, session commands, and editor-side MCP server injection. Octomind's docs also describe WebSocket server mode, but Antares defers that transport.

The lockfile currently pins Octomind `0.72.0`. Confirm protocol docs and asset hashes against that exact tag before implementation; current `master` docs may move independently.

## Primary command

The P1 process manager should launch a command equivalent to:

```text
octomind acp --name <stable-session-name>
```

Pass the workspace directory as the session CWD using the ACP session request supported by the pinned release. Add `--sandbox` only on platforms where Octomind documents support and after testing the resulting workspace boundary. Never assume a flag applies identically across platforms.

## Required lifecycle

1. Spawn child with piped stdin/stdout/stderr.
2. Send ACP `initialize` request with supported protocol version and client capabilities.
3. Handle the response and agent capabilities; preserve extension metadata.
4. Create or load a session with workspace CWD and injected `mcpServers` if supported by the pinned version.
5. Send `session/prompt` with text/resource blocks for user request and selected context.
6. Consume `session/update` notifications, including text and tool activity; keep unknown update variants visible as safe generic events instead of crashing.
7. Cancel through `session/cancel` when supported; report cancellation distinctly from success.
8. Close stdin and continue reading stdout until EOF during graceful shutdown, per upstream ACP docs.

## IO invariants

- stdout is JSON-RPC only; stdout debug prints will corrupt the protocol.
- stderr is captured separately and bounded. Never expose secrets in UI/logs.
- Use one writer task to serialize JSON-RPC lines and one reader task for stdout frames.
- Apply frame size limits, request IDs, timeout handling and bounded event queues.
- Handle partial lines, malformed JSON, child exit, pipe closure, broken writer, no response and duplicate notifications.
- Never wait for a model response on the Floem/UI thread.

## MCP injection and policy

The host may inject MCP servers into session creation. The ACP integration docs confirm stdio/HTTP injection, but the injected stdio `env` and HTTP `headers` fields are not copied into Octomind’s server config. Do not rely on per-server injected credentials; pass provider credentials through the sidecar process environment or a separately configured server. Contract-test the exact injected server payload against the pinned ACP release before implementation. Build a local Antares-owned server with a very small surface:

- `list_workspace_files` — filtered and bounded listing.
- `search_workspace` — bounded search, excludes ignore patterns and secrets.
- `read_workspace_file` — canonical path under workspace root, bounded UTF-8 text.
- `get_editor_context` — active file/selection/diagnostic snapshot supplied by the host.
- `propose_patch` — records a patch proposal only; no write side effect.
- `request_command` — creates a pending command request only; no process launch side effect.

After the server is implemented, add the example role from `config-templates/roles.antares.example.toml` to Octomind’s `roles.toml`, replacing the allowlist with the exact tool names returned by the real server. Disable automatic capability activation and inspect the effective tool set after startup. Important: Octomind documents that role grants are not an OS sandbox; interactive monitor tools, runtime activation and project-local tools may still be present. The Antares app must apply a separate policy layer and OS sandbox where supported. Fail closed if an unexpected tool surface is available.

## Provider configuration

Octomind's current provider guide includes a Google AI Studio provider prefix `google-studio` and credential `GOOGLE_STUDIO_API_KEY`. Google's current Gemini 3.8 Flash stable model ID is `gemini-3.8-flash`. The example config fragment uses:

```toml
[model]
name = "google-studio:gemini-3.8-flash"

[supervisor.model]
name = "google-studio:gemini-3.8-flash"

[compression.model]
name = "google-studio:gemini-3.8-flash"
```

Merge these entries into an Octomind-generated config; do not replace the full generated config with a fragment. Configure the key as a Codespaces secret or OS environment variable. Validate with `octomind config --validate` and a short smoke request before enabling in-app settings.

The Gemini model selected in Antigravity is a separate authentication path; Antares must not copy Antigravity credentials or session tokens.

## Version pin and updates

- Current scaffold pin: `0.72.0`.
- Verify platform archive digest before extraction and execution.
- Store the installed binary under an Antares-owned app-data directory.
- Upgrade only in a dedicated PR: update lock file, read release notes, review protocol changes, run mock contract tests and manual ACP smoke test.
- Do not install by piping a remote shell script directly into a shell from the application.
