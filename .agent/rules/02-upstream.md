# Lapce upstream maintenance

- `origin` is the user's Antares fork. `upstream` is `https://github.com/lapce/lapce.git`.
- Confirm the upstream default branch before syncing; current upstream metadata reports `master`, but re-check before every sync.
- Never rebase or force-push `main`. Sync upstream into a dedicated `agy/sync-upstream-*` branch and merge through a reviewed PR.
- Every edit to an upstream file must be recorded in `docs/PATCHES.md` with rationale and a re-apply hint.
- Prefer small isolated extension points. Do not rewrite `lapce-app/src/app.rs` wholesale or replace an entire large module for a panel.
- Defer rebranding of app IDs, icons, updater URLs, binary names and settings namespace to a dedicated release phase.
