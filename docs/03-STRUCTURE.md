# Antares IDE project structure

The starter ZIP is an overlay, not a replacement Lapce source tree. The automation forks `lapce/lapce`, then applies Antares-owned files while preserving the upstream editor and its build/configuration entrypoints.

```text
AntaresIDE/
├── Cargo.toml                         # upstream Lapce workspace; bootstrap adds antares-agent
├── Cargo.lock                         # preserved, resolved/updated by Cargo
├── Makefile                           # upstream; includes Makefile.antares
├── Makefile.antares                   # separate Antares targets
├── rust-toolchain.toml                # pinned Rust 1.98.0 for this audited Lapce revision
├── README.md                          # Antares project entry; prior Lapce README is preserved in docs/antares/upstream/
├── LICENSE                            # upstream Lapce license; preserve
├── THIRD_PARTY_LICENSES.md
├── NOTICE-ANTARES.md
├── AGENTS.md / GEMINI.md              # short entrypoint pointers
├── AGENTS.antares.md / GEMINI.antares.md # full Antares coding rules
├── .agent/rules/                      # Antigravity project rules
├── .agent/workflows/                  # repeatable agent workflows
├── .agents/guardrails.toml            # Octomind project guardrails; defense in depth only
├── .devcontainer/devcontainer.json    # upstream configuration, untouched
├── .devcontainer/antares/devcontainer.json # Antares Codespaces alternative
├── .github/workflows/                 # upstream workflows plus antares-*.yml workflows
├── .github/ISSUE_TEMPLATE/            # Antares issue form
├── config-templates/antares/          # non-secret Octomind examples and pinned release metadata
├── scripts/                           # one-click setup, overlay, bootstrap, CI, diagnostics, installer
├── docs/                              # product, architecture, roadmap, security, setup, and prompt docs
├── antares-agent/                     # initial Antares-owned crate
│   ├── src/lib.rs                     # P0 typed sidecar discovery boundary + unit tests
│   └── tests/                          # P1+ ACP/process/security integration-test plan and fixtures
├── lapce-app/ lapce-core/             # upstream editor UI/core; modify only at verified extension points
├── lapce-proxy/ lapce-rpc/            # upstream proxy and RPC crates
└── defaults/ extra/ icons/            # upstream assets/configuration, preserved
```

## Antares-owned modules planned for later phases

```text
lapce-app/src/antares/     # only after inspecting the current panel/view patterns
antares-agent/             # process, ACP, session lifecycle
antares-context/           # bounded workspace/editor context and redaction
antares-bridge/            # narrowly scoped host-owned MCP operations
antares-patch/             # diff validation, preview, approval, apply/rollback
antares-settings/          # config schema, migrations, validation
```

Do not create every planned crate as an empty P0 placeholder. Add a crate when it has a stable responsibility, meaningful tests, and a real consumer. Keep generic Antares crates independent of `lapce-app` and Floem; the app layer adapts Lapce/Floem state into the generic services.
