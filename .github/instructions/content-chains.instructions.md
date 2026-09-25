---
applyTo: "db/resources/Content/Seed/**/*.sql"
---

# Content chain review rules

These chains drive the content engine: triggers (events), conditions (gates), actions (effects). When reviewing a chain SQL change, work through this checklist.

## Interaction-type bits

For any chain that triggers on `interact_tag`, the tagged entity must have its `interaction_type` bit set somewhere — otherwise the client renders it as scenery and never sends the click. Set the bit when the entity becomes interactable; clear it when it's done.

Common masks (see `docs/content/interaction-flags.md` for the full table):

| Mask | Constant | Use for |
|---|---|---|
| `2` | `INT_Banker` | Banker NPC |
| `32` | `INT_RingNetwork` | Ring transporter — usable rings |
| `128` | `INT_Trainer` | Ability trainer |
| `256` | `INT_MinigameLivewire` | Hackable console (Livewire) |
| `512` | `INT_MinigameActivate` | Activate-style minigame |
| `8192`–`2097152` | `INT_Vendor*` | Vendor sub-categories (OR multiple) |
| `8388608` | `INT_AStoryMissionAvaliable` (sic) | "?" main-story available |
| `16777216` | `INT_AStoryMissionActive` | "!" main-story active |
| `33554432` | `INT_AStoryMissionTurnIn` | "?" main-story turn-in |
| `1073741824` | `INT_MissionWorldObject` | Quest item glow |

## Set/clear pairing

Every `op: "|"` (set) needs a matching `op: "~"` (clear) on the chain that completes the work. Forgotten clears leave stale icons; missing sets leave entities unclickable.

For mission progression, also add a `player_loaded`-triggered chain that re-applies the bit for active steps. Interaction flags don't persist on the entity across server restart, so without restoration a relog mid-mission breaks interactivity. Worked example: chains 1045/1046 in `castle_cellblock_chains.sql` restore HackTheRings_Switch's bit based on which step is active.

## Edge-triggered chains and the step-activation race

`enter_region`, `player_loaded` and the cover triggers are **edges**: the
client reports the crossing once. A chain gated on a mission step that is not
yet active sees that edge, fails its gate, and never gets another one. The
2026-09-18 Castle playtest lost objective 2484 to it.

Review rules:

- An `enter_region` chain gated on mission state is covered by the engine.
  The server replays the edge when the step activates with the player already
  inside the volume (`content-engine.md` §3, "Step-activation replay"). No
  second trigger row is needed.
- An `enter_region` chain gated on `world` or `archetype` **only** is not
  covered — those are not mission gates, and the replay refuses the chain
  because re-firing it would not be idempotent. Either give it the
  `step_status` gate it wants, or add a second trigger on the event that
  opens its gate.
- A `player_entered_cover` chain gated on mission state is covered the same
  way: the server replays the enter edge for each cover set the player is
  already in when the step activates. `player_left_cover` and the cover
  duration milestones are not replayed.
- `player_loaded` chains are still not covered. They keep the
  second-trigger rule: add a trigger on the event that opens the gate,
  usually `mission_completed '<id>'` (a second trigger row on the same chain,
  tested through `load_chain_expansions_for_test`, or a paired chain with a
  condition-parity guard).
- A chain that should repaint an offer after an **abandon** uses the
  `mission_abandoned` trigger, not a second `mission_completed` row.

## Mission grants must gate on `not_active`

Every chain whose actions include `accept_mission` must carry a
`mission_status <id> eq not_active` condition (see chain 1001 for the
canonical shape). Since #411 the server also refuses re-accepts
authoritatively (`accept_mission`'s offer guard: already-active,
failed-without-`can_repeat_on_fail`, or completed past `num_repeats`),
so a missing gate no longer corrupts saved mission rows — but it still
fires the chain's *other* actions (dialogs, highlights) spuriously, so
the condition remains a review requirement.

## Dialog buttons and the two hard rules

A `dialog_choice` chain only fires if the client actually sends something. It
sends on a button click, and — **only when the dialog has zero buttons across
all of its screens** — on close, with `button_id = -1`. A dialog that has any
button sends nothing when the player closes it.

Two rules follow. Both are enforceable by reading the seed, and both soft-lock
a real player when broken. Full reasoning, evidence and the window/button
compatibility tables are in
[docs/content/dialog-ui-client-contract.md](../../docs/content/dialog-ui-client-contract.md).

- **A dialog that keys a `dialog_choice` chain has either zero buttons, or a
  button on its FINAL screen.** A button that stops before the last screen
  leaves a player who reads to the end with nothing to click and a Done that
  emits nothing.
- **Never add a button to 2300, 5021, 5020, 2574, 2575, 2577, 2581, 5003,
  5004, 5008 or 5009.** All are button-less today and key chains through the
  `-1` close. Any button stops the close emitting — including one the window
  cannot even draw, since an undrawable button still counts toward the total.

Check the button type against the window as well: `DUIST_DefaultBlurb` can
only draw More Info (1) and Accept (2); `DUIST_DefaultDialog` can only draw
Accept (2) and Generic 1-3 (4, 5, 6); `DUIST_DefaultTutorial` draws none.

`DUIST_DefaultRadio` and `DUIST_DefaultRealization` are the same window as
`DUIST_DefaultDialog` and take the same button types. Two more review points
the tables above do not cover:

- **"Final screen" means the highest `dialog_screens.index`, not the highest
  `screen_id`.** The two agree for every dialog shipping today, which is
  exactly why the wrong one is easy to write.
- **Do not key a `dialog_choice` chain on a Tutorial or a type-0
  (`DUIST_None`) dialog.** Tutorial never draws a cooked button, so the player
  has nothing to click and nothing can fire the chain.

[`crates/content-engine/tests/dialog_button_linter.rs`](../../crates/content-engine/tests/dialog_button_linter.rs)
enforces all of the above against the Castle and Castle_CellBlock chain seeds,
alongside
[`interact_tag_linter.rs`](../../crates/content-engine/tests/interact_tag_linter.rs)
for the interaction-type rule at the top of this file. Both parse the seed
directly and need no database.

## Inventory consumption

`UseInventoryItem` fires `OnItemUse` as a pure event — the base no longer auto-consumes the stack. Chains that need to consume (consumable vials, mission objects) must include an explicit `remove_item` action. This is the correct pattern for `item_use`-triggered chains:

```sql
(chain_id, 'remove_item', <design_id>, NULL, '{"qty": 1}', 0, 0),
```

`Action::RemoveItem` routes through `CellToBaseMsg::RemoveInventoryItemByType`, which resolves the player's first matching stack (ordered by `container_id, slot_id` to prefer the main bag over the bandolier) and applies the full wire-update sequence. Non-consumable items (radios, multi-step "use on target" objectives) simply omit the `remove_item` action.

## Auto-generated `space_*_chains.sql` (chain IDs 5xxx)

These come from a converter that walks the level-script node graph (`python/cell/spaces/*.py`). Known converter bugs:

- **Wrong action verb**: the converter has emitted `accept_mission` where the original Python calls `missions.complete()`. Re-accepts a just-completed mission and loops the player back. Always cross-check the action against the source Python.
- **Duplicate actions within one chain**: 5005 has the same `add_dialog` action 5 times; 5001 has 14 actions including duplicate `accept_mission`/`display_dialog`/`launch_ability`. Each runs once per fire, which usually breaks the mission. Collapse dupes.
- **Duplicate conditions**: 5005 has the same `archetype eq 8` condition 4 times. Collapse.
- **Shadow chains**: 5012/5013 are duplicate auto-generated chains for the same node. Pick one or disable both and keep the curated equivalent.

When a PR regenerates these, diff against the previous version. Disable buggy chains by setting `enabled` to `false` in the `content_chains` row; preserve the row for traceability.

## Sort-order discipline

`sort_order` within a chain's `content_actions` determines execution order and is also used as a deduplication key. When adding actions, increment past the highest existing value — don't reuse.

## Chain-ID ranges

Stay inside the allocated range to keep each file searchable by mission.

### Castle_CellBlock — `castle_cellblock_chains.sql` (1001-1199)

```text
Mission 622:  1001-1010   Mission 638:  1011-1030
Mission 639:  1031-1040   Mission 640:  1041-1050
Mission 641:  1051-1070   Mission 680:  1071-1080
Missions 681-687: 1081-1130
```

### Castle (World 8) — 1201-1400, split one file per mission family

The Castle rebuild campaign reserves `1201-1400` and splits it across
three seed files so concurrent packets never edit the same file. Each is
included from `db/database.sql` after `castle_cellblock_chains.sql`.

```text
castle_701_chains.sql        Mission 701:  1201-1260
                               CA01 arrival + Gerschon:  1201-1230 (1201-1205 used)
                               CA03 body + relog restore: 1231-1260 (1231-1243 used)
castle_702_704_chains.sql    Missions 702/703: 1261-1290
                             Mission 704:      1291-1320
castle_706_708_chains.sql    Mission 706:  1321-1340
                             Mission 708:  1341-1380
                             Optional Level-5 branches: 1381-1400
```

Allocation source: [docs/analysis/castle-rebuild/work-packets.md](../../docs/analysis/castle-rebuild/work-packets.md)
"Worker Input And Ownership". `1200` is left unused as a gap between the
two zones' blocks; effect chains start at 2001.

## Linked references

- `docs/content/content-engine.md` — **runtime reference**: architecture, vocabulary, schema, lifecycle, observability, performance.
- `docs/content/extending-the-engine.md` — how-to guide for adding a new trigger / condition / action variant.
- `docs/content/proposed-extensions.md` — justified roadmap of engine extensions still to come.
- `docs/content/interaction-flags.md` — full per-bit cookbook with worked patterns.
- `docs/content/dialog-ui-client-contract.md` — what the client's dialog window really does: window types, drawable button types, close semantics, lure delivery.
- `docs/content/mission-chains.md` — every chain catalogued.
- `docs/architecture/data-driven-content-engine.md` — original design doc (historical; superseded by the runtime reference above for what's actually shipping).
- `python/cell/spaces/Castle_CellBlock.py` — original level script (source of truth for what the auto-converter *should* have produced).
