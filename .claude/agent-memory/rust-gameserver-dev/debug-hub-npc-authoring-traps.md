---
name: debug-hub-npc-authoring-traps
description: Traps hit building the stasis-room debug hub (templates 300-304): Vendor interaction was never set, set 4 is not harmless, system_message is a stub, new dialogs need DIALOG_OVERRIDES plus pinned-id test edits, cell-methods has no sqlx.
metadata:
  type: project
---

Found 2026-09-26 building the Castle_CellBlock stasis-room debug hub (PR on branch `content/stasis-debug-hub`, doc `docs/content/debug-hub.md`).

- **`NpcInteractionType::Vendor` was never set in production.** The store-open arm of `handle_interact` was dead; template 25 "worked" only because `try_open_trainer` answers first. Fixed by `static_interaction_for_flags` in `cell-world/.../space_manager/spawn.rs` (any `INT_Vendor*` bit on the template). It runs at spawn only: a `set_interaction_type` vendor bit changes the cursor, not the route, and the respawn tick never restores `interaction_type` (only the flags).
- **Ability set 4 is NOT the "zero-damage melee"** the NA43 comment implies: its primary is 584 (30 m ranged, 25 dmg). A harmless faction-10 target needs set 6 (`[710]` only). An empty set falls back to 592 Pistol Shot. NEUTRAL/FRIENDLY aggression never stops fighting back once hit.
- **Loot only rolls on death** (`loot_drop.rs`); a "loot container" must be a killable faction-10 mob. Use probability 1 rows or an empty roll leaves an unclickable corpse.
- **`system_message` is log-only**; visible chain feedback = `npc_bark` + a never-displayed holder dialog whose screen carries the text (loaded into `dialog_screen_text` at startup).
- **New Cimmeria dialogs**: seed rows + `DIALOG_OVERRIDES` entry (append at END; tests read `[0]`), and edit the id list pinned in `resources/src/base/resources/tests/dialog_overrides.rs` and the 3995/3996-only `shipped_overrides_carry_no_buttons`. Use screen ids >= 200000 (PAK screens reach 120383). Generic button type 4, not Accept 2.
- **`cimmeria-cell-methods` has no `sqlx` dep**: a live-DB test there cannot name `PgPool`; load via a local `macro_rules!` that calls the `spawner::load_*` fns on the `require_db_or_skip!()` value.

Related: [[content-chain-authoring-traps]], [[seed-name-id-and-asset-naming]], [[npc-range-gate-and-weapon-range-columns]].
