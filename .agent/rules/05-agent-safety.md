# Octomind / agent safety rules

- Integrate Octomind through ACP over stdio first; read the docs matching the pinned version before implementing.
- Keep stdout reserved for JSON-RPC and drain stderr separately. Bound queues, logs, message sizes and tool output.
- The first role is read-only and exposes only the Antares context bridge. Never assume `allowed_tools` is an OS sandbox.
- Disable automatic capability activation in the Antares runtime profile and verify the effective tool list after startup.
- `.agents/guardrails.toml` is defense in depth, not the only policy. Match targets must use exact capabilities resolved by Octomind.
- Agent writes become proposal records; Antares validates and previews a diff. Only a user accept action may apply it.
- Agent command requests become pending approvals; no command is launched before an explicit user action.
- Do not render tool output as HTML or auto-open untrusted links. Redact secrets from prompt context and diagnostics.
