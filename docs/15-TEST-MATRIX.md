# Antares IDE test matrix

| Area | Test | Expected result | Automation |
|---|---|---|---|
| Scaffold | Required paths exist | No missing mandatory files | `scaffold-check.sh` |
| Config | Devcontainer JSON parses | Valid JSON | `scaffold-check.sh` |
| Config | TOML examples and guardrails parse | Valid TOML | `scaffold-check.sh` |
| Shell | All helper scripts parse | `bash -n` exits 0 | `scaffold-check.sh` |
| Python | Overlay script compiles | `py_compile` exits 0 | `scaffold-check.sh` |
| Overlay | Existing upstream README preserved | Backup exists and original hash matches | Overlay integration test |
| Overlay | Existing `.devcontainer/devcontainer.json` preserved | Byte-for-byte unchanged | Overlay integration test |
| Overlay | Existing workflow names preserved | Upstream files remain unchanged | Overlay integration test |
| Overlay | Re-run overlay | No duplicate files/integration lines | Overlay integration test |
| Workspace | `antares-agent` registered | Cargo metadata lists package | `cargo metadata` |
| Workspace | First package check updates lock | Cargo resolves member | `cargo check -p antares-agent` |
| Rust | Formatting | No rustfmt diff | `cargo fmt -p antares-agent -- --check` |
| Rust | Compile | No compiler errors | `cargo check -p antares-agent` |
| Rust | Unit tests | All pass | `cargo test -p antares-agent` |
| Rust | Lints | No warnings under `-D warnings` | `cargo clippy -p antares-agent --all-targets -- -D warnings` |
| Rust | Reproducible lock | No lock changes | `antares-ci.sh locked` |
| Workspace | Lapce + Antares compile | All workspace members compile | Manual `antares-ci.sh full` |
| Git | Whitespace / conflict markers | No whitespace errors or accidental markers | `git diff --check` + review |
| CI | Fast branch workflow | Green | GitHub Actions |
| CI | Full workspace dispatch | Green on supported runner | Manual workflow |

## ACP integration acceptance matrix (future P1)

| Case | Fixture | Expected behavior |
|---|---|---|
| Executable missing | Invalid Octomind path | Clear diagnostic; editor remains usable |
| Process starts | Fake ACP process | State reaches Initializing/Ready |
| Initialize unsupported | Wrong protocol version | Refuse session; show actionable error |
| Session create | Valid ACP fixture | Store returned session ID |
| Session resume | Known session ID | Resume selected session only |
| Prompt stream | Text/update notifications | Append incrementally in correct order |
| Tool call | ToolCall and ToolCallUpdate | Show tool name/status/result; do not auto-approve writes |
| Cancel | In-flight prompt | Cancel request and end in stable state |
| Child crash | Non-zero exit | Show failure and allow restart; no editor crash |
| Malformed JSON | Invalid frame | Reject frame and log redacted diagnostic |
| stdout contamination | Non-protocol stdout | Treat as protocol error; logs never silently become model output |
| Timeout | No response | Stop waiting; inspect before retrying side-effecting operation |
| Auth required | Runtime auth error | Show sign-in requirement without logging secrets |
| Multiple sessions | Interleaved notifications | Correctly route by session ID |
| Window close | Active child | Cancel/close and terminate child cleanly |

## Patch safety acceptance matrix (future P2/P3)

| Case | Expected behavior |
|---|---|
| Relative path inside workspace | Allowed to preview |
| Absolute path | Rejected |
| `../` traversal | Rejected |
| Symlink points outside workspace | Rejected after canonicalization |
| `.env` / private key / token file | Protected by default |
| `.git` / build cache | Rejected by default |
| Malformed unified diff | Rejected; no writes |
| Stale base hash | Conflict shown; no writes |
| File changed after preview | Revalidate before apply; no blind overwrite |
| User rejects proposal | No buffer/disk change |
| User closes preview | No buffer/disk change |
| User approves valid proposal | Apply through editor APIs with undo support |
| Multi-file partial failure | Roll back or enter explicit recoverable transaction error |

## UI manual test checklist (future P4)

- Panel opens/closes/resizes and persists across restart.
- Focus remains correct between editor and composer.
- Enter/shortcut sends; Escape/cancel stops generation.
- Streaming output remains smooth with long messages.
- Tool cards show pending/running/success/failure states.
- Error, offline, starting, reconnecting, cancelled, and empty states are visible.
- Session switch does not leak messages across sessions.
- Theme changes are respected.
- Keyboard navigation and accessible labels work.
- Large conversation history does not freeze typing or scrolling.
- Closing window cancels or safely detaches the child process.
