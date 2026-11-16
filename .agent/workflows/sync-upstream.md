# Workflow: sync Lapce upstream

```bash
git remote get-url upstream || git remote add upstream https://github.com/lapce/lapce.git
git fetch upstream --prune
git remote show upstream
# Confirm the default branch from remote metadata before choosing it.
git switch -c agy/sync-upstream-YYYYMMDD main
git merge upstream/master
```

Resolve conflicts in a dedicated branch. Reapply only the entries in `docs/PATCHES.md`, inspect the full diff, run the Antares gate and `cargo check --workspace --locked`, then push and open a PR. Do not assume the upstream branch is permanently named `master`; verify it each time. Never force-push `main`.
