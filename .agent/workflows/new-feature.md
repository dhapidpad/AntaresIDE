# Workflow: implement a feature

1. Read `AGENTS.md`, `docs/STATUS.md`, and the exact roadmap item.
2. Inspect the current Lapce/Octomind implementation and cite the source paths in `docs/DECISIONS.md` if they determine an interface.
3. Write a <=10-line plan and define acceptance criteria.
4. Create or reuse `agy/<phase>-<topic>`; do not work on `main`.
5. Add a test/fixture for the behavior or protocol contract before implementation where practical.
6. Implement one vertical slice; keep UI glue and domain logic separated.
7. Run `make -f Makefile.antares antares-gate`, `git diff --check`, and targeted workspace checks.
8. Review diff for unrelated upstream changes, placeholders, secrets and unreviewed file/command paths.
9. Commit, push, watch CI for the exact SHA, and report the Actions URL.
10. Update `docs/STATUS.md`; update `docs/PATCHES.md` if upstream code changed.
