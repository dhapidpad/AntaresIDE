# Context and patch rules

- Context snapshots must declare workspace root, active file URI, selection range, language, diagnostics source and timestamp.
- Canonicalize paths and prove containment under the active workspace. Reject traversal, symlink escape and non-regular files.
- Enforce per-file and per-request byte/token budgets. Exclude secrets, generated directories, build output and ignored files by default.
- Redact credentials and common secret patterns before sending context to a provider; never upload `.env`, private keys or credential stores.
- Represent proposed edits as structured patch hunks with path, base revision/hash and expected old text.
- Revalidate the base revision immediately before apply. Reject stale/conflicting patches; preserve undo support.
- Do not call a patch "applied" until Lapce confirms the buffer/document update.
