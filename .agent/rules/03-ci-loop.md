# GitHub Actions loop

- Work on `agy/<phase>-<topic>`; do not commit directly to `main`.
- Run `make -f Makefile.antares antares-gate` and `git diff --check` before committing.
- For UI or cross-crate changes, also run the narrowest relevant workspace check (`cargo check --workspace --locked` when practical).
- Commit with Conventional Commits; push the branch; identify the Actions run matching the exact pushed commit SHA.
- Use `gh run watch <run-id> --exit-status`; on failure inspect `gh run view <run-id> --log-failed` before changing code.
- Never assume the newest run belongs to the current commit. Never report CI green without a completed successful run URL.
- Never use `[skip ci]`, `continue-on-error` as a workaround, or force-push.
