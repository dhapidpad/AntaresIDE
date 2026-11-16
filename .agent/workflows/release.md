# Workflow: build a desktop release

1. Read `docs/08-UPSTREAM-MAINTENANCE.md`, `docs/SOURCE-REFERENCES.md`, and `THIRD_PARTY_LICENSES.md`.
2. Verify the current Antares branch is clean and the locked Antares test gate is green.
3. Confirm the release version and whether this release is an artifact preview or a public release.
4. Build/test Linux, Windows, and macOS using `.github/workflows/antares-release.yml`.
5. Inspect each artifact and its metadata. Confirm binary architecture and runtime dependencies.
6. Preserve Lapce and Octomind notices. Do not bundle Octomind unless its license/attribution and checksum are included.
7. Do not claim signing, notarization, installer packaging, auto-update, or full desktop validation unless those features exist and passed tests.
8. Use tags `antares-vX.Y.Z` only after explicit release approval.
