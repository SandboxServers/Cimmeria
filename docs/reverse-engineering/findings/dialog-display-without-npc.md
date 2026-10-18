# Dialog display without an NPC present (radio beats)

> Type: explanation.
> Status: static evidence only (DK-06a, 2026-10-18). No live client, no x64dbg, no new Ghidra session. Ghidra addresses are quoted from the existing findings, not re-decompiled here.
> Packet: `docs/analysis/dakara-e1-rebuild/packets/wave-1.md` (DK-06a). Feeds the D-DK10 ruling and DK-06b.

## The question

Dialogs 5815 and 5816 (1570 step 4905, "Use your radio to contact Gen. Hammond") have screens spoken by speaker 2609 (Hammond) and by speaker 0 (the player). Nobody with speaker 2609 stands on Dakara. The later radio beats have the same shape:

| Step | Mission | Radio dialog(s) | First screen speaker | Source of the step text |
|---|---|---|---|---|
| 4905 | 1570 SG-18 | 5815, 5816 | 2609 (Hammond) | `mission_steps.sql:2376` |
| 4916 | 1647 Enemy at the Gates | 5824 | 945 (Moh'katan) | `mission_steps.sql:2787` |
| 4921 | 1649 Moh'katan | 5833 | 2179 (Teal'c per the audit) on every NPC screen (0, 2, 4, 6) | `mission_steps.sql:2803` |
| 4937 | 1653 Superweapon | 5848, 5849 (the audit does not map the step to one of them) | 2323 (Bra'tac) | `mission_steps.sql:2847` |

Dialog ids come from `docs/analysis/dakara-e1-rebuild/audit.md` (line 114 names 5815/5816, 5824 and 5833; line 97 names 5848/5849 for 1653).

How can the server show such a dialog, server-authoritative and with no client patch?

## Current server behaviour

Speaker resolution lives in `display()` in `crates/cell-content/src/cell/content/executor/dialog/mod.rs` (lines 55-114). For a dialog that is not a monologue (`space_mgr.monologue_dialog_ids`, line 66):

1. `params["target_entity_id"]` (line 88), stamped only by `fire_interact_tag` / `fire_interact_template`.
2. The player's `last_interaction_target` pin (line 94).
3. Otherwise `tracing::warn!` and `return` (lines 100-111). Nothing is sent.

None of the radio dialogs is a monologue: their first screen has a non-zero speaker. Two consequences follow from the code:

- A non-interact chain (a `dialog_choice`, a mission step, a minigame victory) has no `target_entity_id`. Its only source is the pin, which may be absent (fresh login) or stale (an NPC from an earlier interact). A stale pin binds the wrong portrait, the trap described in `.claude/agent-memory/mission-systems-advisor/dialog-chain-authoring-rules.md` section 1.
- No seeded chain in `db/resources/Content/Seed/` fires 5815, 5816, 5824, 5833, 5848 or 5849 today (grep, 2026-10-18). The radio steps have no display row in this tree yet, so the observable behaviour is not something this tree can show. Per the code above, a display from a non-interact chain with no pin is dropped with a `warn`.

## Wire send

`send_dialog_display` (`crates/cell-content/src/cell/interactions/dialog.rs` lines 35-41, payload lines 73-78) sends `onDialogDisplay` (client method 105) with `EntityId` = the id passed in, `DialogID`, `MissionFlags = 0`, `IsImmediate = 1`, `aMissionId = 0`. Method 105 is `INT32 EntityId, INT32 DialogID, INT32 MissionFlags, UINT8 IsImmediate, INT32 aMissionId` (`docs/protocol/client-method-dispatch-table.md:274`). The `.def` is `entities/defs/SGWPlayer.def:1150-1156`.

## Client evidence

### Q1. What the client does with an unexpected EntityId

The client reads the five fields by name from the Mercury stream (`dialog-controller-wire-flow.md`, table at lines 29-40). The handler `FUN_00d25900` (true wire handler) builds an `AvailableDialog`, asks the cache for the cooked dialog, and only when `IsImmediate != 0` pins GameEntityManager slot `0x11` (`DialogSpeaker`) to the `EntityId` (`FUN_00c67bd0`).

When the cooked entry is ready, `FUN_00d25310` matches the entity and calls `FUN_00d25200`, which calls the display core `FUN_00d24f10` (`dialog-portrait-lookup.md` lines 101-113; `dialog-controller-wire-flow.md` lines 49-66). The display core:

- calls `FUN_00d22c90(dialog, 0x1b58)` (player-side slot) and `FUN_00d22c90(dialog, 0x1bbc)` (NPC-side slot). Each calls `LookupEntityListenerEntry(GameEntityManager, entityId, ...)` and pins the entity only when the lookup returns a non-zero entry (`dialog-portrait-lookup.md` line 143);
- emits `Event_UI_DialogDisplay` (`FUN_00d27b80`, carrying `DialogID` only) unconditionally after the pins (`dialog-portrait-lookup.md` line 112).

The portrait is rendered only from a non-zero slot-`0x11` entity (`dialog-portrait-lookup.md` line 96; `Event_UI_UnitMappingChanged` fires for non-zero entities). The portrait then goes through `FUN_00e6a430`, which repeats `LookupEntityListenerEntry` (`dialog-portrait-lookup.md` lines 160-164).

The speaker name is not keyed by the EntityId. The Lua side reads the current screen's `SpeakerID` and looks the name up in the cooked `speakers` table; an empty name falls back to the player's name (`dialog-portrait-lookup.md` lines 195-218, 274-277). The `dialog-controller-wire-flow.md` document disputes that name track (its header note), so treat the name path as MEDIUM.

Applied to the three cases in the question:

| EntityId sent | Window and text | Name label | Portrait |
|---|---|---|---|
| Id of an NPC not in the player's AoI listener table | Shown (unconditional emit) | From `SpeakerID`, Hammond | Blank: the lookup misses, so no pin reaches slot `0x11` |
| The player's own entity id | Shown | From `SpeakerID`, Hammond | Renders the player's head if the player's entity is in its own listener table. The server already sends the player's id for every monologue dialog (`display()` line 85, about 42% of authored screens per its comment) and intends the player portrait there, so the client accepts this id. Not verified for a screen named for Hammond; MEDIUM |
| `0` | Shown | From `SpeakerID`, Hammond | Blank. The slot is pinned only for a non-zero entity (`dialog-portrait-lookup.md` line 96), so no portrait is raised. MEDIUM |

**Answer to Q1 (confidence MEDIUM overall):** the client shows the window and the text for any EntityId, including `0` and an id it cannot find. The EntityId only decides the portrait. The speaker name comes from the screen's `SpeakerID`, so Hammond's name shows without Hammond present. The risky case is the player's own id: the portrait would show the player for Hammond's lines.

### Q2. Any client-side radio, comm or remote-speaker path?

- **Radio type 4 (`RadioType`)** exists: `Dialog.RadioType = 4` (`dialog-controller-wire-flow.md` line 89). It registers to `DialogWin` and `DialogMod.initDialog`, the same window as types 2 and 5 (table at lines 110-117). Its only extra effect is a **radio icon**, a lure: a non-immediate dialog raises `Event_UI_DialogAvailable` and waits for the player to click the icon (lines 66, 119). The lure is not a remote-speaker path. It does not change the portrait, the name or the window.
- **The type comes from the client's cooked data, not from the server seed.** The client reads `CookedDataDialogs.pak` (`dialog-controller-wire-flow.md` line 19). `db/resources/Dialogs/Seed/dialogs.sql` has `ui_screen_type` = `DUIST_DefaultDialog` on 4287 of its 5418 dialog rows (the rest: 988 `DUIST_DefaultBlurb`, 128 `DUIST_None`, 15 `DUIST_DefaultTutorial`) and `DUIST_DefaultRadio` (the enum value; its mapping to client type 4 is not verified here) on none. All six radio dialogs (5815, 5816, 5824, 5833, 5848, 5849) are `DUIST_DefaultDialog`, type 2. A server-side seed edit cannot change what the client shows. Changing the type needs a cooked override, which is a different pipeline (`crates/resources/src/base/dialog_overrides/`) with a documented crash risk (`docs/reverse-engineering/findings/cooked-dialog-override-crash.md`).
- **Non-modal line:** the server's `NpcBark` action (`crates/cell-content/src/cell/content/executor/bark.rs`, header lines 1-12) sends a non-modal companion line in the chat window. The docs say it is the only non-modal text route the client honours today. It is not a dialog: no window, no buttons, no portrait, no `dialogButtonChoice`.
- **No remote-speaker event-set or `dialog_flags` bit was found.** The `dialog_flags` column (`dialogs.sql`) is not decoded in any finding. Not traced here; a flag that the client reads would need a Ghidra session, which this packet does not run.

**Answer to Q2 (confidence HIGH that no radio window or remote-speaker frame exists in the traced code; LOW that no flag does):** the only server-triggerable radio-like path is the type-4 lure, and that type is fixed by cooked data.

### Client question left open

Whether the player's own entity is in its own AoI listener table (decides the player-id row above), and whether `dialog_flags` gates a remote-speaker frame. Both need one Ghidra session on `LookupEntityListenerEntry` callers and on `FUN_00d24f10`'s flag reads. Not run here.

## Options for the server

Each option is classed **free** (server-only: code, chain params, spawn or template seed rows, no client file) or **client patch** (needs a client UI file or a cooked override).

| # | Option | How | Class | Effect | Confidence |
|---|---|---|---|---|---|
| A | Display with the player as the speaker entity | `display()` passes `entity_id` for the radio dialog, like the monologue branch (lines 66-85) | free | Window, text, Hammond name. Player's face may show beside Hammond's name | MEDIUM |
| B | Speaker-0 fallback: send EntityId `0` | New branch in `display()`: a chain flag (e.g. `params["speaker"] = "none"`) sets `npc_entity_id = 0` and skips the pin lookup | free | Window, text, Hammond name. No portrait. No false face | MEDIUM |
| C | Pin a hidden mission-scoped NPC | Spawn an invisible entity with a Hammond-like template in the player's AoI and pin its id as `target_entity_id` | free (template and spawn rows) | Window, text, name, and a portrait from a real entity | MEDIUM; needs a template with a Hammond model and a spawn that is invisible and not interactable |
| D | Non-modal radio frame | New client window or remote-speaker frame | client patch | Real radio UI | HIGH that it is a client patch |
| E | Cooked override: set type 4 or add a speaker-less record | Cooked override pipeline | cooked override (not free: the documented crash risk) | Lure icon or changed record; no portrait change | HIGH that the type is cooked-only |
| F | Chat line (`NpcBark`-style) instead of a dialog | Existing `NpcBark` action | free | Non-modal text only. Not a dialog, no buttons. Fallback when no dialog is wanted | HIGH |

## Recommendation for D-DK10

**Option B.** It is free, it reuses the one display function, and it avoids the two wrong pictures: the player's face (option A) and a hidden entity (option C). Hammond's name and the dialog text come from the dialog rows, and the portrait stays blank. Option C is the fallback if the owner wants Hammond's face: it needs a template and a spawn row.

The server change for DK-06b:

- **File:** `crates/cell-content/src/cell/content/executor/dialog/mod.rs`.
- **Function:** `display()`, lines 55-114.
- **Change:** before the monologue test, read `params.get("speaker")`. When it equals `"none"`, set `npc_entity_id = 0` and skip the `target_entity_id` and pin lookup, so no warn fires. `send_dialog_display` passes the id to `npc_name`, which converts it with `u32::try_from` (`crates/cell-content/src/cell/interactions/dialog.rs` lines 113-117). Id 0 is passed through to `entity_names(0)`, which returns `EntityNames::default()` for an id with no entity (`crates/cell-world/src/cell/space_manager/entity_labels.rs:109-113`), so it cannot panic; only the log line depends on it. What the client does with id 0 is the unverified part (Q1 row 3).
- **Seed:** the radio chain rows (`db/resources/Content/Seed/`, a DK-06b file) must carry `params` `{"speaker": "none"}` on their `display_dialog` action. The seed change is DK-06b's, not this packet's.
- **Tests:** a unit test in `executor/dialog/tests.rs` that a `speaker = none` display sends EntityId 0 and does not warn. The live-DB replay is the reviewer's call.

The architect decides whether the radio flag belongs in chain params or in a dialog-level set (`radio_dialog_ids`, as `monologue_dialog_ids` already is). Params are simpler and avoid a spawner change.

## Open items

- The player-own-id portrait (Q1 row 2) and the `dialog_flags` question (Q2): both need one Ghidra session. Not run.
- The 4937 step maps to 5848 or 5849; the audit lists both.
- Whether the 1570 radio beat is fired from an interact-tag (a stale-pin risk) is a DK-06b question; no chain rows exist yet.
- **Owner decision if option B is rejected:** options D and E are client patch or cooked override. The architect must not pick them without the owner.
