# Testing and evidence

- A command is “passed” only when it actually ran and exited 0.
- Distinguish scaffold/static checks, Antares-crate checks, full workspace checks, desktop smoke tests, and manual UI checks.
- Do not hide or skip failed tests to make CI green.
- Reproduce failures from the exact command and job log.
- First bootstrap may use unlocked Cargo resolution until `Cargo.lock` includes the new workspace member; after committing it, run locked checks.
- If a command is unavailable, report it as not run and give the missing prerequisite.
