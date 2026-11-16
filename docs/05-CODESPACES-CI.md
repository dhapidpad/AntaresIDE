# GitHub Codespaces and GitHub Actions

## Codespaces setup

1. Fork `lapce/lapce` and rename your fork `antares-ide`.
2. Open a Codespace on the fork's `main` branch.
3. Choose 4-core/8-GB as a practical baseline; choose 8-core/16-GB or higher when available for faster full workspace builds. Codespaces capacity depends on the account/organization quota.
4. Extract this scaffold into the repository root and run **Codespaces: Rebuild Container** so the committed `.devcontainer` configuration and `rust-toolchain.toml` are applied.
5. Run `bash scripts/bootstrap.sh` and inspect its changes before proceeding.

## Secrets

For the in-app Google AI Studio provider, create `GOOGLE_STUDIO_API_KEY` as a Codespaces secret and expose it only to the Antares Codespace. Alternatively, configure OctoHub or another provider supported by Octomind. Do not commit `.env`, `.env.*`, provider credentials, auth files or session logs.

Antigravity's Gemini 3.8 Flash login is managed by Antigravity; it is not copied into the repo and is not the same as the IDE's provider API key. The root `rust-toolchain.toml` pins Rust `1.98.0`, matching the Lapce workspace metadata observed during scaffold preparation.

## CI stages

- `antares-fast`: format, Clippy and unit tests for Antares-owned crates.
- `workspace-check`: compile the complete Lapce + Antares workspace with required Linux build libraries.
- Release workflow: build root `lapce` and `lapce-proxy` binaries for Linux, Windows and macOS.

The root Cargo package owns the `lapce` and `lapce-proxy` binary targets. Do not use `cargo build -p lapce-app` as a binary build command.

## Local commands

```bash
# Quick Antares-only gate
make -f Makefile.antares antares-gate

# Full workspace check (slower)
cargo check --workspace --locked

# Verify metadata and binary target names
cargo metadata --no-deps --format-version 1
cargo build --release --locked --bin lapce --bin lapce-proxy

# Review before commit
git diff --check
git status --short
```

## Commit and Actions loop

```bash
git switch -c agy/p0-bootstrap
git add -A
git commit -m "chore: add Antares IDE scaffold"
git push -u origin HEAD
gh run list --branch "$(git branch --show-current)" --limit 5
```

Choose the run whose `headSha` matches `git rev-parse HEAD`, then:

```bash
gh run watch <run-id> --exit-status
gh run view <run-id> --log-failed
```

Use `scripts/ci-loop.sh` for the same process after later commits. A green check must be from the current commit SHA.

## What Codespaces cannot verify

Codespaces is headless. Do not attempt to launch the native Floem/wgpu window there. Use Codespaces for formatting, linting, tests, and compile checks. Download the GitHub Actions artifact and smoke-test the GUI on a desktop for window rendering, input, panel behavior, terminal, fonts, GPU backend and OS packaging.

Codespaces and Actions usage is metered; stop the Codespace when not actively working.
