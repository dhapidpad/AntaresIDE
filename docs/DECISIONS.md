# Decisions log

| ID | Date | Decision | Evidence / source |
|---|---|---|---|
| ADR-001 | 2026-11-16 | Use Lapce as editor foundation; keep Antares logic in separate crates and a thin Floem adapter. | `lapce/lapce` root `Cargo.toml`, `README.md`, `docs/building-from-source.md` |
| ADR-002 | 2026-11-16 | Use Octomind as an external process, not vendored code. | `Muvon/octomind` README and Apache-2.0 LICENSE |
| ADR-003 | 2026-11-16 | ACP over stdio is first transport; defer WebSocket. | `Muvon/octomind/doc/integration/02-acp-protocol.md` |
| ADR-004 | 2026-11-16 | Use host-injected MCP context/proposal tools; keep direct write/exec disabled by default. | Octomind roles, MCP and guardrails docs; Antares security model |
| ADR-005 | 2026-11-16 | Pin Octomind 0.72.0 and its published asset digests in the scaffold. | GitHub release metadata for `Muvon/octomind` tag `0.72.0` |
| ADR-006 | 2026-11-16 | Use Gemini 3.8 Flash in Antigravity; in-app model choice remains independent. | Google AI Studio model docs; Octomind provider docs |

Append an entry whenever a source/API decision changes. Include the exact upstream path, commit/tag, and what was verified.
