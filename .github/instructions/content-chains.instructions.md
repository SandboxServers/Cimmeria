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

## Inventory consumption

`UseInventoryItem` on the base side consumes the item *before* `fire_item_use` reaches the cell. Don't add `remove_item` actions to chains triggered on `item_use` — they'll double-consume any stack >1. The current `Action::RemoveItem` executor is a stub, so today the bug is latent, but new chains should not rely on the stub.

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

```
Mission 622:  1001-1010   Mission 638:  1011-1030
Mission 639:  1031-1040   Mission 640:  1041-1050
Mission 641:  1051-1070   Mission 680:  1071-1080
Missions 681-687: 1081-1130
```

Stay inside the allocated range to keep the file searchable by mission.

## Linked references

- `docs/content/interaction-flags.md` — full per-bit cookbook with worked patterns.
- `docs/content/mission-chains.md` — every chain catalogued.
- `docs/architecture/data-driven-content-engine.md` — how chains/triggers/conditions/actions fit together.
- `python/cell/spaces/Castle_CellBlock.py` — original level script (source of truth for what the auto-converter *should* have produced).
