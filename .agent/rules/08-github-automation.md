# GitHub and Codespaces safety

- Keep `origin` on the personal Antares fork and `upstream` on `lapce/lapce`.
- Never force-push or rewrite published history.
- Do not merge upstream or create a PR to Lapce automatically.
- Keep Antares workflow names prefixed with `antares-`; do not replace upstream workflows.
- Do not replace `.devcontainer/devcontainer.json`; use `.devcontainer/antares/devcontainer.json`.
- Explain quota/billing before starting a Codespace and wait for the user's explicit confirmation.
- Inspect staged diffs before every commit. Never commit secrets, local state, build outputs, or downloaded Octomind binaries.
