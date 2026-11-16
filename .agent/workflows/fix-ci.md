# Workflow: repair failed CI

1. Identify the failing Actions run and exact commit SHA; do not inspect a different/newer run by accident.
2. Read failed job logs with `gh run view <id> --log-failed` and identify the first causal error.
3. Classify the failure: format/lint, compile/API drift, missing system library, lockfile, test regression, timeout/resource issue, or workflow error.
4. Reproduce locally with the same command and toolchain where practical.
5. Fix the root cause. Do not suppress the check, weaken a test, add `continue-on-error`, or change unrelated code.
6. Add a regression test if the failure exposed a behavior bug.
7. Run the local gate, commit `fix(ci|scope): ...`, push, and watch the new run matching the new SHA.
8. After five genuine attempts, stop and document the exact blocker and evidence in `docs/STATUS.md`.
