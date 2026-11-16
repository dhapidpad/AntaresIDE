# GitHub automation and repository lifecycle

## Repository model

```text
lapce/lapce (upstream)
       |
       | GitHub fork operation
       v
YOUR_ACCOUNT/AntaresIDE (origin)
       |
       +-- master (upstream-derived default branch)
       +-- agy/antares-bootstrap (initial scaffold branch)
       +-- agy/p0-foundation
       +-- feature/<bounded-feature>
```

Keep the fork relationship intact. Do not create an unrelated repository and copy source without retaining the upstream relationship and license history.

## Automation responsibilities

| Component | Responsibility | Trigger |
|---|---|---|
| `scripts/one-click-setup.sh` | Fork/reuse fork, clone, overlay, run checks, commit, push, optional Codespace creation | Human invoked |
| `scripts/apply-overlay.py` | Copy only Antares-owned files and preserve upstream config | Setup script |
| `scripts/bootstrap.sh` | Idempotently add workspace member, Makefile include, ignore rules, upstream remote | Setup / Codespaces |
| `scripts/scaffold-check.sh` | Verify required file set, parse JSON/TOML, shell/Python syntax | Local + CI |
| `scripts/antares-doctor.sh` | Diagnose repo, tools, remotes, scaffold without exposing secrets | Local / support |
| `scripts/antares-ci.sh` | Fast, locked, or full quality gate | Developer / CI |
| `scripts/antares-update-upstream.sh` | Fetch upstream and report divergence only | Human invoked |
| `antares-ci.yml` | Fast crate and scaffold checks | Push / PR / manual |
| `antares-release.yml` | Cross-platform desktop artifacts | Manual or `antares-v*` tag |
| `antares-upstream-watch.yml` | Weekly report of upstream drift; never merges | Weekly / manual |

## Branch and commit policy

- Never implement directly on `master`/default branch.
- Branch format: `agy/<phase>-<short-description>` or `feature/<short-description>`.
- Use conventional, descriptive commits, for example `feat(agent): add ACP session manager`.
- One commit should represent one coherent, reviewable change.
- No force push, rebase of shared branches, or automatic merge.
- Push only after reviewing `git status`, `git diff --check`, and `git diff --cached`.

## CI layers

### Fast checks

Run for every branch push and PR:

- Scaffold and config validation.
- Rust formatting for Antares-owned crate.
- Antares crate check, tests, and clippy.

The initial bootstrap workflow intentionally permits Cargo to update `Cargo.lock` on its CI runner because the new workspace member may not yet be represented in the inherited lockfile. After the first Codespace resolves and commits `Cargo.lock`, use `bash scripts/antares-ci.sh locked` for reproducible local verification.

### Full workspace checks

Manual `workflow_dispatch` only. This compiles the whole Lapce + Antares workspace and installs the native Linux packages Lapce documents for source builds. It is more expensive than the fast check and should not run on every small documentation-only commit.

### Release workflow

The release workflow builds the native `lapce` and `lapce-proxy` executables for Linux, Windows, and macOS. It uploads artifacts. The workflow does not claim to produce an installer or signed/notarized app package unless those packaging/signing steps are implemented and tested separately.

## Secrets and permissions

- CI uses `contents: read` unless a workflow has a narrowly justified write action.
- No PAT is required for normal Actions checkout/build/test.
- Use Codespaces Secrets or the supported provider credential flow for API keys.
- Never echo env values, dump all env variables, upload `.env`, or print process arguments that may contain credentials.
- Avoid `pull_request_target` for code execution from untrusted forks.
- Do not add broad `contents: write` to CI.

## Upstream maintenance

`antares-upstream-watch.yml` reports divergence. It deliberately does not merge. Before syncing:

1. Fetch upstream.
2. Read `docs/PATCHES.md` and identify Antares modifications overlapping upstream changes.
3. Compare commits and inspect the full diff.
4. Create a dedicated sync branch.
5. Merge/rebase only after reviewing conflicts and tests.
6. Run Antares tests and relevant Lapce package/workspace checks.
7. Document the upstream SHA and changed paths.

## Codespaces

The fork includes `.devcontainer/antares/devcontainer.json` as an alternative environment. The upstream `.devcontainer/devcontainer.json` stays untouched. Use `gh codespace create --devcontainer-path .devcontainer/antares/devcontainer.json` or select that path in the Codespaces UI.

The requested 4 CPU / 8 GB / 32 GB settings are requirements, not free-tier guarantees. Let GitHub present the available machine options. Check account quotas and any billing implications before creating a Codespace.
