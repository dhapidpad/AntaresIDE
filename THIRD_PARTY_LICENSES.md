# Third-party notices and distribution checklist

| Component | Use | License | Required handling |
|---|---|---|---|
| Lapce | Main editor source and UI foundation | Apache-2.0 | Preserve root `LICENSE`, relevant source headers and upstream notices. Keep a clear record of modified upstream files. |
| Floem | UI dependency used by Lapce | Follow the license in its pinned upstream repository/revision | Do not assume the license from a package name; verify the exact pinned revision and keep its notices as required. |
| Octomind | Separate agent-runtime executable | Apache-2.0 | Keep it external and version-pinned. If bundling its binary, include its LICENSE and required notices in the package. |
| Rust crates | Build/runtime dependencies | Per crate | Run the repository's existing `cargo-deny` policy and resolve every license/source exception explicitly. |

Do not replace or delete Lapce's license. Do not claim Antares owns upstream components. Keep the original license text when redistributing source or binaries as required by each license.
