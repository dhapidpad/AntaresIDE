# Lapce upstream maintenance

## Source of truth

Use the user's Antares fork as `origin` and `lapce/lapce` as `upstream`. Current GitHub metadata reports `master` as Lapce's default branch; always verify the remote default branch before syncing.

## Rules

- Preserve upstream code, copyright headers, assets, docs and license.
- Keep each upstream patch minimal and list it in `docs/PATCHES.md`.
- Do not reformat entire upstream files as part of an Antares feature.
- Avoid conflicts by adding `lapce-app/src/antares/` modules rather than distributing unrelated changes across editor internals.
- When integration requires changing commands, panel IDs, config keys, RPC enums or theme definitions, record the exact affected source paths and tests.

## Sync procedure

1. Fetch `upstream` and confirm its default branch.
2. Create a dedicated sync branch from current `main`.
3. Merge the confirmed upstream default branch; do not rebase `main`.
4. Resolve conflicts and re-apply only the documented Antares patches.
5. Run Antares crate gate and full workspace checks.
6. Push branch, wait for matching CI, open PR and review the entire diff.
7. Update `docs/PATCHES.md` and `docs/STATUS.md`.

## Rebrand strategy

Do not rename targets, app IDs, updater endpoints, config paths, file associations or icon asset names while ACP integration is still changing. Do all branding in a dedicated phase with explicit build/package tests for all OS targets.
