---
name: launcher-preferences-schema-change-fanout
description: Changing the desktop launcher's Preferences shape breaks frontend fixtures (strict decode), embedded migration/adoption records, and the adoption Plan intent digest; migrate in serde try_from, digest through digest_form
metadata:
  type: project
---

Desktop launcher `Preferences` (`crates/launcher/desktop/engine/src/storage/mod.rs`) went to schema 2 in LX-02 (2026-10-10). Three non-obvious fan-outs:

- The frontend decodes `NativeSnapshot` with `onExcessProperty: "error"` and a literal `schema_version`, and every status type (install, launch, migration, adoption, game update) nests it. Any field change means updating the `preferences:{schema_version:...}` literals in about ten `src/*.test.ts` and `*-uat.mjs` fixtures; a regex over `preferences\s*:\s*\{[^{}]*\}` finds them all.
- Legacy-import and adoption records embed whole `Preferences` values (`before`/`after`) and compare them with the live file. Migrating in `#[serde(try_from = "StoredPreferences")]` upgrades every embedded copy the same way, so those structural comparisons still match; migrating only in `DesktopState::open` would not.
- Structural equality is not enough where a record is **digested**: the adoption `Plan::digest()` (SHA-256 of its JSON) is the operation's `intent_digest`. A plan journaled by the old build re-serializes in the new shape and stops matching, so an adoption in flight across the upgrade reads as Corrupt. Fix: `Plan::digest()` serializes preferences through `Preferences::digest_form()`, which emits the frozen v1 shape while the v2 fields hold their defaults and the full value otherwise. Guard: `adoption/digest_tests.rs` (a hand-written v1 plan JSON must hash to its own bytes). The migration `Record` is compared, not digested, so it needs nothing.

**Why:** a schema bump that only touches the Rust struct passes engine tests and fails `npm test`, recovery comparisons, or in-flight digests.
**How to apply:** for the next preferences field, add it to `StoredPreferences` as `Option`, decide the per-version rule in `TryFrom`, extend `digest_form` so values at the new field's default still digest in the older shape, then sweep the TS fixtures. Adoption file reads are macOS-only (file identity checks), so the persisted-recovery half of a digest test only runs on macOS. Related: [[launcher-state-key-and-egui-wake]].
