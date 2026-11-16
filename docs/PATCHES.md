# Patches to Lapce upstream

The Antares IDE integration keeps modifications to upstream Lapce files isolated, minimal, and non-destructive.

| File | Change | Why | Re-apply hint | Status |
|---|---|---|---|---|
| root `Cargo.toml` | Add `antares-agent` to `[workspace].members` | Compile Antares sidecar boundary in same workspace | `scripts/bootstrap.sh` performs an idempotent members-array edit | Applied |
| root `Cargo.lock` | Record the new workspace package after `cargo check -p antares-agent` | Keep `--locked` CI reproducible | Let Cargo update it; never hand-edit lock data | Applied |
| root `Makefile` | Add `-include Makefile.antares` | Expose Antares local gate without replacing upstream targets | `scripts/bootstrap.sh` appends the include once | Applied |
| root `.gitignore` | Append Antares secret/local build ignores | Avoid committing credentials and generated output | `scripts/bootstrap.sh` appends missing lines only | Applied |
| `lapce-app/Cargo.toml` | Add `antares-agent = { path = "../antares-agent" }` | Expose agent domain state/reducer to Lapce UI | Add path dependency under `[dependencies]` | Applied |
| `lapce-app/src/lib.rs` | Add `pub mod antares;` | Expose Antares UI module to Lapce application | Add module declaration | Applied |
| `lapce-app/src/panel/kind.rs` | Add `PanelKind::Antares` | Register Antares AI panel type with icon and default position | Add enum variant and match arms | Applied |
| `lapce-app/src/panel/data.rs` | Add `PanelKind::Antares` to `default_panel_order` | Show Antares panel in `RightTop` position by default | Insert into `RightTop` panel vector | Applied |
| `lapce-app/src/panel/view.rs` | Add `PanelKind::Antares` view dispatch and tooltip | Render `antares_panel` when selected in panel picker | Add match arms in `panel_view` and `panel_picker` | Applied |
| `lapce-app/src/window_tab.rs` | Add `antares_data` field and `PanelKind::Antares` focus check | Wire reactive panel data into window tab lifecycle | Add field to `WindowTabData` and constructor | Applied |
