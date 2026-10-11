---
name: launcher-preferences-schema-change-fanout
description: Changing the desktop launcher's Preferences shape breaks every frontend fixture (strict decode) and embedded migration/adoption records; migrate in serde try_from
metadata:
  type: project
---

Desktop launcher `Preferences` (`crates/launcher/desktop/engine/src/storage/mod.rs`) went to schema 2 in LX-02 (2026-10-10). Two non-obvious fan-outs:

- The frontend decodes `NativeSnapshot` with `onExcessProperty: "error"` and a literal `schema_version`, and every status type (install, launch, migration, adoption, game update) nests it. Any field change means updating the `preferences:{schema_version:...}` literals in about ten `src/*.test.ts` and `*-uat.mjs` fixtures; a regex over `preferences\s*:\s*\{[^{}]*\}` finds them all.
- Legacy-import and adoption records embed whole `Preferences` values (`before`/`after`) and compare them with the live file. Migrating in `#[serde(try_from = "StoredPreferences")]` upgrades every embedded copy the same way, so those comparisons still match; migrating only in `DesktopState::open` would not.

**Why:** a schema bump that only touches the Rust struct passes engine tests and fails `npm test` and recovery paths.
**How to apply:** for the next preferences field, add it to `StoredPreferences` as `Option`, decide the per-version rule in `TryFrom`, then sweep the TS fixtures. Related: [[launcher-state-key-and-egui-wake]].
