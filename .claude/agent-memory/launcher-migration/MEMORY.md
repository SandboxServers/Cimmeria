# Launcher migration findings

- 2026-10-04: `crates/launcher/src/config.rs`, `identity.rs`, `state.rs` define three distinct legacy sources: executable-adjacent config and identity, plus game-root historical ledger. Identity metadata and ordered patch claims must be preserved; historical telemetry `enabled` is not consent.
- 2026-10-04: Desktop `installed-content.json` requires signed release/operation ownership; a legacy ledger cannot be promoted to it. `engine/src/storage/migration/` stores a separate bounded recoverable import record, with exact source text and explicit native game-folder mapping. See `crates/launcher/desktop/docs/migration.md`.
