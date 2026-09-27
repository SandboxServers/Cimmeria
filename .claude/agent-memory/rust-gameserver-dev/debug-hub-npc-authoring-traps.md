---
name: debug-hub-npc-authoring-traps
description: Traps hit building the stasis-room debug hub (templates 300-304): Vendor interaction was never set, set 4 is not harmless, system_message is a stub, new dialogs need DIALOG_OVERRIDES plus pinned-id test edits, cell-methods has no sqlx, the DebugHub_ tag prefix is counted, vendor purchases land in bag 1 unstacked.
metadata:
  type: project
---

Found 2026-09-26 building the Castle_CellBlock stasis-room debug hub (PR on branch `content/stasis-debug-hub`, doc `docs/content/debug-hub.md`).

- **`NpcInteractionType::Vendor` was never set in production.** The store-open arm of `handle_interact` was dead; template 25 "worked" only because `try_open_trainer` answers first. Fixed by `static_interaction_for_flags` in `cell-world/.../space_manager/spawn.rs` (any `INT_Vendor*` bit on the template). It runs at spawn only: a `set_interaction_type` vendor bit changes the cursor, not the route, and the respawn tick never restores `interaction_type` (only the flags).
- **Ability set 4 is NOT the "zero-damage melee"** the NA43 comment implies: its primary is 584 (30 m ranged, 25 dmg). A harmless faction-10 target needs set 6 (`[710]` only). An empty set falls back to 592 Pistol Shot. NEUTRAL/FRIENDLY aggression never stops fighting back once hit.
- **Loot only rolls on death** (`loot_drop.rs`); a "loot container" must be a killable faction-10 mob. Use probability 1 rows or an empty roll leaves an unclickable corpse.
- **`system_message` is log-only**; visible chain feedback = `npc_bark` + a never-displayed holder dialog whose screen carries the text (loaded into `dialog_screen_text` at startup).
- **New Cimmeria dialogs**: seed rows + `DIALOG_OVERRIDES` entry (append at END; tests read `[0]`), and edit the id list pinned in `resources/src/base/resources/tests/dialog_overrides.rs` and the 3995/3996-only `shipped_overrides_carry_no_buttons`. Use screen ids >= 200000 (PAK screens reach 120383). Generic button type 4, not Accept 2.
- **The stasis room's A-B hub line is full** (2026-09-27, SS-U3): its only open slot is 5.07 from the respawner and kept empty on purpose. New hub NPCs go on the B-C wall (Banker 470 mid-wall, mail clerk 490 at 13 along from B) or elsewhere; CR-11 owns the D-A wall. `debug_hub_dispatch_tests` pins the count of `DebugHub_*` tags, so a new hub spawn breaks it until the count and a click row are added.
- **A content action that writes base-owned data** (mail, persistence) sends one `CellToBaseMsg` and lets the base do one transaction: `cell-content` cannot depend on `base-methods`. Per-player limits on re-triggerable chains (dialog buttons) need durable state: `sgw_player_content_cooldown`, claimed in the same tx (SS-U3 `mail/content.rs`).
- **`cimmeria-cell-methods` has no `sqlx` dep**: a live-DB test there cannot name `PgPool`; load via a local `macro_rules!` that calls the `spawner::load_*` fns on the `require_db_or_skip!()` value.
- **Hub placement (2026-09-27, BV-04):** the A-B line is full to corner B (pet trainer 450 is 2.1 from the B-C wall), crafting's `CraftHub_*` take the A-D side, the C-D wall is the exit. The Banker 470 went mid B-C wall, 3 in. `staged_hub` counts `DebugHub_*` spawns: 7 after BV-04.
- **`sgw_player.player_name` is UNIQUE** (`sgw_player_player_name_key`): a name-based GM lookup has no ambiguous case, and a fixture cannot insert two characters with one name.
- **The `DebugHub_` tag prefix is counted** (added 2026-09-27, crafting CR-11): `debug_hub_dispatch_tests::staged_hub` spawns every `DebugHub_*` spawnlist tag and asserts the count (5 on main, 6 on the pets branch). A campaign adding hub NPCs either bumps that count (and conflicts with every other campaign doing the same) or uses its own prefix; the crafting corner uses `CraftHub_*`.
- **Hub placement is crowded:** A-B wall = 400-404 + pet trainer 450; D-A wall = crafting 410-414; black-market 238 sits past the C-D side. Check open branches' stasis-room rows before placing more.
- **A vendor purchase always lands in bag 1** and ignores `max_stack_size` (13 of a stack-1 item become one stack of 13). A GM/crafting grant through `first_player_container` instead lands `{17,15}` items in bag 15.

Related: [[content-chain-authoring-traps]], [[seed-name-id-and-asset-naming]], [[npc-range-gate-and-weapon-range-columns]].
