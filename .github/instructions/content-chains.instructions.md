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
| `4` | `INT_Auction` | Black Market auctioneer (opens the auction window) |
| `32` | `INT_RingNetwork` | Ring transporter — usable rings |
| `128` | `INT_Trainer` | Ability trainer |
| `256` | `INT_MinigameLivewire` | Hackable console (Livewire) |
| `512` | `INT_MinigameActivate` | Activate-style minigame |
| `8192`–`2097152` | `INT_Vendor*` | Vendor sub-categories (OR multiple) |
| `8388608` | `INT_AStoryMissionAvaliable` (sic) | "?" main-story available |
| `16777216` | `INT_AStoryMissionActive` | "!" main-story active |
| `33554432` | `INT_AStoryMissionTurnIn` | "?" main-story turn-in |
| `1073741824` | `INT_MissionWorldObject` | Quest item glow |

## UI-open actions (`open_black_market`)

`open_black_market` (action `Action::OpenBlackMarket`) opens the client Black
Market / auction window for the triggering player (`onBMOpen`, client method
90). It takes no `target_id`/`target_key`/`params` — the auctioneer entity is
resolved at execution time from the interact trigger's `target_entity_id`, so it
**must** be fired from an `interact_tag` (or `interact_template`) chain on the
auctioneer; firing it from a region/load trigger aborts with a `warn!` (no
auctioneer to bind). Pair it with a `set_interaction_type` (`INT_Auction`) on the
same tag — both on a `player_loaded` set chain (no clear, since the auctioneer is
a permanent fixture). Worked example: chains 5030/5031 (`BlackMarket_Auctioneer`)
in `space_castle_cellblock_chains.sql`.

## Set/clear pairing

Every `op: "|"` (set) needs a matching `op: "~"` (clear) on the chain that completes the work. Forgotten clears leave stale icons; missing sets leave entities unclickable.

For mission progression, also add a `player_loaded`-triggered chain that re-applies the bit for active steps. Interaction flags don't persist on the entity across server restart, so without restoration a relog mid-mission breaks interactivity. Worked example: chains 1045/1046 in `castle_cellblock_chains.sql` restore HackTheRings_Switch's bit based on which step is active.

## Mission grants must gate on `not_active`

Every chain whose actions include `accept_mission` must carry a
`mission_status <id> eq not_active` condition (see chain 1001 for the
canonical shape). Since #411 the server also refuses re-accepts
authoritatively (`accept_mission`'s offer guard: already-active,
failed-without-`can_repeat_on_fail`, or completed past `num_repeats`),
so a missing gate no longer corrupts saved mission rows — but it still
fires the chain's *other* actions (dialogs, highlights) spuriously, so
the condition remains a review requirement.

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

## Chain-ID ranges (`castle_cellblock_chains.sql`)

```text
Mission 622:  1001-1010   Mission 638:  1011-1030
Mission 639:  1031-1040   Mission 640:  1041-1050
Mission 641:  1051-1070   Mission 680:  1071-1080
Missions 681-687: 1081-1130
```

Stay inside the allocated range to keep the file searchable by mission.

## Linked references

- `docs/content/content-engine.md` — **runtime reference**: architecture, vocabulary, schema, lifecycle, observability, performance.
- `docs/content/extending-the-engine.md` — how-to guide for adding a new trigger / condition / action variant.
- `docs/content/proposed-extensions.md` — justified roadmap of engine extensions still to come.
- `docs/content/interaction-flags.md` — full per-bit cookbook with worked patterns.
- `docs/content/mission-chains.md` — every chain catalogued.
- `docs/architecture/data-driven-content-engine.md` — original design doc (historical; superseded by the runtime reference above for what's actually shipping).
- `python/cell/spaces/Castle_CellBlock.py` — original level script (source of truth for what the auto-converter *should* have produced).
