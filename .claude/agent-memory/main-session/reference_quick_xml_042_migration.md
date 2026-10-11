# quick-xml 0.42 migration (2026-10-07)

- `quick-xml` 0.42 uses `&str` and `Cow<str>` for event names, attribute keys and values, text, CDATA, and comments. XML readers in auth, cell-world, defs, minigame, resources, `tools/ContentEditor`, and patchset tests previously converted these from bytes.
- The cooked-dialog parser keeps `Attribute::value` raw and escaped for parse/emit fidelity. Its `round_trip_preserves_escaped_text_verbatim` test guards that contract.
- The root workspace CI excludes GUI crates, so dependency updates need separate checks of `sgw-launcher` and `cimmeria-content-editor`. The content editor Tauri macro requires `tools/ContentEditor/ui/dist` from `npm run build` before `cargo check`.

Sources: `Cargo.toml`, `crates/resources/src/base/dialog_overrides/parse.rs`, `crates/resources/src/base/dialog_overrides/parse.rs` tests, and the excluded-crate list in `CLAUDE.md`.

- 2026-10-11: `sgw-launcher` was deleted in LX-18, so CI's excluded crates are now `cimmeria-app`, `cimmeria-content-editor`, `cimmeria-scene-editor`, `cimmeria-client-telemetry`, `cimmeria-client-patches` and `cimmeria-lab`. The desktop launcher is its own workspace (`crates/launcher/desktop/`), checked by `launcher-desktop.yml`; check it separately after a dependency update it shares.
