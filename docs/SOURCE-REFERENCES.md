# Source references verified on 2026-11-16

## Lapce
- Repository: https://github.com/lapce/lapce
- README: pure Rust editor using Floem UI, Rope Science, wgpu; LSP, modal editing, remote dev, WASI plugins and terminal.
- Root `Cargo.toml`: workspace members `lapce-app`, `lapce-proxy`, `lapce-rpc`, `lapce-core`; root package defines the `lapce` and `lapce-proxy` binaries; workspace edition `2024`, MSRV `1.98.0`, Apache-2.0.
- Build guide: https://github.com/lapce/lapce/blob/master/docs/building-from-source.md
- License: https://github.com/lapce/lapce/blob/master/LICENSE

## Octomind
- Repository: https://github.com/Muvon/octomind
- Release pinned in this starter: [0.72.0](https://github.com/Muvon/octomind/releases/tag/0.72.0), published 2026-11-15.
- ACP integration: https://github.com/Muvon/octomind/blob/master/doc/integration/02-acp-protocol.md
- WebSocket integration: https://github.com/Muvon/octomind/blob/master/doc/integration/01-websocket-server.md
- Provider configuration: https://github.com/Muvon/octomind/blob/master/doc/usage/04-providers.md
- Roles and permissions: https://github.com/Muvon/octomind/blob/master/doc/usage/06-roles.md
- Guardrails: https://github.com/Muvon/octomind/blob/master/doc/usage/18-guardrails.md
- License: https://github.com/Muvon/octomind/blob/master/LICENSE

## Coding model
- Gemini 3.8 Flash model documentation: https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash
- Stable model id requested for Antigravity coding: `gemini-3.8-flash`. The model used inside Antares/Octomind is a separate setting.

## Important distinction
This starter does not copy either upstream source tree. It gives Antares integration scaffolding, a pinned sidecar install path for Linux Codespaces, and a development plan. Re-check upstream files, licenses, and release digests before every intentional upgrade.
