# One-click GitHub fork, Antares setup, and Codespaces

This procedure is designed to start from the extracted Antares starter ZIP and end with a personal GitHub fork, an Antares setup branch, a validated overlay, a pushed commit, and optionally a new Codespace.

## 1. Before running

Install GitHub CLI (`gh`), Git, and Python 3.11+. Sign in once:

```bash
gh auth login
gh auth status
```

The account must be allowed to create a fork. No GitHub token should be typed into a script or committed to a file. `gh` handles the authenticated API calls and Git credential setup.

## 2. One-command path

Run from the root of the extracted starter folder, not from a different repository:

```bash
bash scripts/one-click-setup.sh --create-codespace
```

Defaults:

| Setting | Default |
|---|---|
| Upstream | `lapce/lapce` |
| Fork name | `AntaresIDE` |
| Clone path | sibling folder `AntaresIDE` next to the extracted starter |
| Working branch | `agy/antares-bootstrap` |
| Commit | yes, if staged changes exist |
| Push | yes, after the commit |
| Codespace | optional, only after an explicit confirmation |

Override values:

```bash
bash scripts/one-click-setup.sh \
  --repo-name AntaresIDE \
  --target ../AntaresIDE \
  --branch agy/antares-bootstrap \
  --create-codespace
```

The script is idempotent with respect to the fork and overlay: an existing repository is reused only if GitHub confirms it is a fork of `lapce/lapce`; an unrelated repository with the same name causes a safe stop. Existing upstream files are preserved. It does not force-push, rewrite Git history, change repository visibility, enable billing, merge a PR, or submit a PR to Lapce upstream.

Available safety switches:

```bash
# Fork/clone/apply/validate/commit, but do not push
bash scripts/one-click-setup.sh --no-push

# Apply and stage changes only; no commit and no push
bash scripts/one-click-setup.sh --no-commit
```

If the script stops after creating the fork because GitHub has not made the fork visible yet, rerun the same command. It checks and reuses the existing verified fork.

## 3. What the automation changes

The overlay uses dedicated names and directories:

- `AGENTS.antares.md`, `GEMINI.antares.md` plus small root entry-point pointers.
- `.agent/rules/` and `.agent/workflows/` for Antigravity operating rules.
- `.agents/guardrails.toml` for Octomind's project guardrails, subject to verification against the pinned runtime.
- `antares-agent/` and `Makefile.antares` for the first Antares-owned crate.
- `docs/` for the architecture, phases, source notes, test plan, and prompts.
- `config-templates/antares/` for non-secret example configuration.
- `.github/workflows/antares-*.yml` for Antares CI/release/upstream-watch workflows.
- `.devcontainer/antares/devcontainer.json` for the Antares Codespaces environment.
- `scripts/` for bootstrap, diagnostics, validation, CI, and upstream-report helpers.

The overlay leaves the original Lapce `LICENSE`, root `.devcontainer/devcontainer.json`, original GitHub workflows, and original build packages in place. It preserves the former root `README.md` in `docs/antares/upstream/lapce-README.md` before installing the Antares README.

## 4. Codespace creation

The script asks before creating a Codespace because compute usage is subject to the account's included quota and any applicable billing settings. Review the machine size and account allowance before confirming.

To create it manually later:

```bash
gh codespace create \
  --repo YOUR_GITHUB_LOGIN/AntaresIDE \
  --branch agy/antares-bootstrap \
  --devcontainer-path .devcontainer/antares/devcontainer.json \
  --display-name "Antares IDE"
```

Do not assume a particular machine ID is available. GitHub will offer machine choices appropriate to the account and repository. The configuration requests a minimum of 4 CPUs, 8 GB memory, and 32 GB storage where available; those are host requirements, not a guarantee of free compute.

## 5. First boot in Codespaces

`post-create.sh` performs:

1. Installs the documented Linux build dependencies used by Lapce.
2. Installs/activates Rust 1.98.0 with rustfmt and clippy.
3. Adds `~/.local/bin` to the interactive shell path.
4. Installs Octomind 0.72.0 from its pinned release asset and verifies the SHA-256 digest before installation.
5. Validates the Antares scaffold.
6. Runs `cargo check -p antares-agent` to resolve the new workspace member and update the lockfile.
7. Runs Antares fmt/check/test/clippy checks.

If a step fails, read the first failing command and fix that step before moving forward. Do not skip a failed step and then claim the Codespace is ready.

## 6. First manual verification

```bash
bash scripts/antares-doctor.sh
bash scripts/scaffold-check.sh
bash scripts/antares-ci.sh fast
git diff --check
git status --short
```

Inspect and commit the generated lockfile when it changed:

```bash
git diff -- Cargo.lock
git add Cargo.lock
git commit -m "chore: lock Antares bootstrap dependencies"
git push
```

After the lockfile has been committed, run the locked checks:

```bash
bash scripts/antares-ci.sh locked
```

The full Lapce workspace is larger than the starter crate. Use the manual full workflow only when needed:

```bash
bash scripts/antares-ci.sh full
```

## 7. Antigravity working arrangement

Antigravity is the implementation agent; Codespaces/GitHub Actions provide a reproducible Linux build and test environment. If Antigravity is running on the local desktop and cannot attach directly to the Codespace filesystem:

1. Clone `YOUR_GITHUB_LOGIN/AntaresIDE` locally on the same branch.
2. Open that local clone in Antigravity.
3. Select Gemini 3.8 Flash and paste `docs/12-AGENT-MASTER-PROMPT.md`.
4. Keep local and Codespace branches synchronized with ordinary Git pull/push.
5. Run build/tests in Codespaces and inspect the resulting GitHub Actions checks.

Do not let two environments edit and push the same branch simultaneously. Before switching between local Antigravity and Codespaces, commit or stash the active environment's changes and pull in the other.

## 8. Recovery

| Problem | Recovery |
|---|---|
| `gh auth status` fails | Run `gh auth login`, then rerun the setup script. |
| Fork name already belongs to an unrelated repository | Choose a different `--repo-name`. The script will not overwrite it. |
| Existing target path is not a Git clone | Choose another `--target` or move the unrelated folder yourself. |
| Worktree is dirty | Review, commit, or stash changes before rerunning. |
| Cargo missing | Create/use the Antares Codespace, then run `bash scripts/antares-ci.sh fast`. |
| Octomind checksum mismatch | Stop; do not bypass checksum verification. Verify the pinned release metadata and update the installer deliberately. |
| CI fails | Open the failing job log, reproduce locally/Codespaces, fix the root cause, rerun the same checks. |
| Upstream has moved | Run `bash scripts/antares-update-upstream.sh`, review the diff, and sync manually. |

## 9. What is intentionally not automated

- Repository visibility changes.
- Billing settings or paid machine selection.
- Automatic merge of any pull request.
- Automatic merge/rebase from Lapce upstream.
- Upload of API keys, OAuth credentials, or secrets.
- Release publication without a reviewed `antares-v*` tag / manual workflow dispatch.
- Automatic application of AI-generated patches.
