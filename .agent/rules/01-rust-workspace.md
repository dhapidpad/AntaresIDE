# Rust workspace rules

- Inherit `edition`, `rust-version`, `version`, and `license` from Lapce workspace metadata. Current upstream values are read from root `Cargo.toml`; do not hard-code stale values.
- Check `cargo metadata --no-deps` before relying on package or binary names.
- Lapce root package owns `lapce` and `lapce-proxy` binaries. `lapce-app` is a library package; verify all target names before editing CI/release.
- Use `thiserror` for new library error types if the dependency is already approved; otherwise use the simplest typed error model.
- Avoid production `unwrap()`/`expect()`. Use `?`, explicit errors and context.
- Do not block the UI thread. Process IO, parsing, network and file scans run on background tasks/threads.
- Keep public APIs small and documented. Do not expose Floem types from `antares-*` crates.
- Add tests next to code; add integration tests under the crate's `tests/` directory.
- Add dependencies only with a reason and license check. Preserve `Cargo.lock`; use `--locked` in CI.
- Format only Antares-owned packages. Never run workspace-wide formatting to clean up unrelated Lapce changes.
