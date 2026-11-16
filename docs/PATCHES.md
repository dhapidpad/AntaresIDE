# Patches to Lapce upstream

No Lapce source files have been changed by the starter ZIP itself. Bootstrap will add the Antares crate and Makefile include to the upstream repository metadata/build files.

| File | Change | Why | Re-apply hint | Status |
|---|---|---|---|---|
| root `Cargo.toml` | Add `antares-agent` to `[workspace].members` | Compile Antares sidecar boundary in same workspace | `scripts/bootstrap.sh` performs an idempotent members-array edit | Pending |
| root `Cargo.lock` | Record the new workspace package after `cargo check -p antares-agent` | Keep `--locked` CI reproducible | Let Cargo update it; never hand-edit lock data | Pending |
| root `Makefile` | Add `-include Makefile.antares` | Expose Antares local gate without replacing upstream targets | `scripts/bootstrap.sh` appends the include once | Pending |
| root `.gitignore` | Append Antares secret/local build ignores | Avoid committing credentials and generated output | `scripts/bootstrap.sh` appends missing lines only | Pending |
