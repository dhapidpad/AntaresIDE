# Workflow: implement one roadmap phase

1. Read the master prompt and the matching phase from `docs/02-ROADMAP.md`.
2. Check `docs/STATUS.md`, `docs/DECISIONS.md`, `docs/PATCHES.md`, and current Git status.
3. Trace real source paths and tests before editing.
4. Write a short plan with target files, invariants, risks, and acceptance tests.
5. Implement one vertically complete slice; avoid broad refactors.
6. Add tests for success, error, cancellation, malformed input, and permission denial as relevant.
7. Run the exact targeted tests, fmt, clippy, scaffold check, and diff check.
8. Review the full diff; remove dead code and debug traces.
9. Update docs and commit only after checks pass.
10. Report what is actually implemented and what remains.
