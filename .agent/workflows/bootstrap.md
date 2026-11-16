# Workflow: Antares bootstrap and initial repository setup

1. Confirm the user extracted the starter into its own folder and has authenticated `gh`.
2. Read `docs/13-ONE-CLICK-SETUP.md`.
3. Run `gh auth status`; never ask the user to paste a token.
4. Review target/branch parameters before executing `scripts/one-click-setup.sh`.
5. Explain that fork creation and push change the user's GitHub account; Codespaces may use quota or incur charges. Require a direct confirmation before starting Codespaces.
6. Run the setup script. Do not bypass a fork-parent mismatch, dirty worktree check, checksum error, or test failure.
7. Review `git status`, `git diff --check`, and changed paths.
8. Wait for GitHub Actions result and inspect failed logs if needed.
9. Do not create or merge a PR to Lapce upstream.
10. Report fork URL, working branch, commit SHA, Codespace status, CI status, and remaining manual work.
