# HA-15d: collectible mission-item grants

Research packet HA-15d, 2026-10-18. Docs only: no seed, code, lab or client
change. Inputs: [work-packets.md](../work-packets.md) (R1, HA-15d),
[owner-decisions-2026-10-18.md](owner-decisions-2026-10-18.md),
[HA-15-audit.md](HA-15-audit.md), [id-allocation.md](../id-allocation.md).

Labels: FACT is read from a seed row or from source in this worktree.
INFERENCE is reasoning, with a confidence of H, M or L. A ticket or draft
chapter is a claim, not evidence. No ID is allocated here; a chain that needs
one is written `ID needed: chain`.

## Scope and decisions

- O18-03: every one of these items is a real, visible, collectible item in
  the mission bag. No invisible completion.
- O18-06: Agnos items 5703, 5704, 5706, 5760, 5763, 5766 are out of scope.
- O18-09: 2698 is a collectible world item. Castle packet CS-08 owns the
  pickup. Harset chains 6503 and 6504 stay disabled. Not designed here.
- 4512 and 2714 belong to HA-15a (owner BlockedDecision). O18-03 makes the
  treat a real item either way. Not designed here.
- In scope: 2720, 2734, 2743, 2818, 2823, 2825, 4396, 4690, 5146, 5736.
- Class: inventory grant and mission-chain wiring. No propId write, no
  bandolier, no vendor. Hazard scan results are in
  [Hazards](#hazards).

## The grant primitive (FACT, H)

- Action `add_item`: `target_id` is the item id, params `{"qty":N}` and an
  optional `"container"`. Executor `grant()` in
  `crates/cell-content/src/cell/content/executor/inventory.rs` resolves the
  container from the item's `container_sets` and sends `GrantItem` to the
  base, which runs `resolve_placement`, `refuse_storage_grant`,
  `persist_grant`.
- All ten items have `container_sets {2}` (mission bag), so a grant lands in
  bag 2 without a `"container"` param.
- `persist_grant` merges into an existing same-type, unbound stack only when
  `max_stack_size > 1` and stack plus count fits. Otherwise it takes a free
  slot. A full bag returns `ContainerFull`, and the content path then sends
  a system mail "Mission Rewards" with the message "Your bag was full. Your
  item is in your mailbox." So a mission item can end up in mail, not the
  mission bag.
- There is no uniqueness guard on a grant and no has-item condition in the
  loader. Loader conditions are `mission_status`, `step_status`,
  `objective_status`, `world`, `entity_tag_state`, `counter`, `archetype`.
  Duplicate protection is therefore only a persisted step or mission flip
  made by the same chain.
- `remove_item` goes through `remove()`: it uses the instance id when
  `fire_item_use` set one, otherwise `RemoveInventoryItemByType`.
- `complete_mission_with_item` is hard-wired to mission 1360, item 3730 and
  one dialog tuple (`action_letter_turn_in.rs`). It cannot serve any item
  here. A new tuple is a code change and would need a maintainer decision;
  this note does not propose one.

### Precedents (FACT, H)

All in `db/resources/Content/Seed/`.

| Shape | Chain | Trigger and guard | Actions, in order |
|---|---|---|---|
| Grant on accept | 6101 (item 2820, qty 3) | `mission_accepted` 742, `mission_status 742 active` | `add_dialog_set`, `add_item` |
| NPC hand-over by dialog choice | 6102 (item 2819) | `dialog_choice` 2638, `step_status 742 2502 active` | `add_item`, `remove_dialog_set`, `advance_step 742 2503` |
| NPC hand-over, second beat | 6110 (item 2864) | `dialog_choice` 2639, `step_status 742 2505 active` | `remove_dialog_set`, `add_item`, `advance_step 742 2506`, `add_dialog_set` |
| Body pickup | Castle 1003 (item 3730) | `dialog_open` 3995, `step_status 622 2113 active` | `set_interaction_type` (clear the click bit) first, `add_item` (container 0), `add_dialog_set`, `advance_step` |
| Item use, no consume | 6103 (disguise) | `item_use` | the item is not consumed; `UseInventoryItem` no longer auto-consumes |

Why these are safe: the guard is a persisted step flip made by the same
chain (`advance_step`), so a second click or a relog finds the step already
past the guard. `once` is not a guard; it resets on relog. 1003 additionally
removes the click affordance before granting, so a fast second press cannot
re-enter.

## Per-item design

Every row: container is mission bag {2}; no ammo, clip or charges; quality
Normal; no rows in `loot.sql` (tables 1 and 2 hold only {3730, 55}); no
existing chain references the item (HA-15 audit scan E, PASS at frozen
SHA 2785df72). Flags are raw values; their bit meanings were not found in
`docs` and are unverified, so no design depends on them.

| Item | Name | Stack | Flags | Event set | Owner packet |
|---|---|---|---|---|---|
| 2720 | Dawson's Symbiote | 1 | 34816 | none | HA-08a |
| 2734 | Monitoring Device | 4 | 32768 | none | HA-06b, HA-07 |
| 2743 | Tollan Control Technology | 1 | 32768 | (2410, 5) | HA-08a |
| 2818 | Goa'uld Symbiote | 1 | 36864 | (2068, 5) | HA-11 |
| 2823 | Petbe's Robes | 1 | 35840 | (597, 5) | HA-13b |
| 2825 | Petbe's Bloody Robes | 1 | 35840 | none | HA-13b |
| 4396 | Straegis Scanner | 1 | 35840 | (2092, 5) | HA-09 |
| 4690 | Nanite Tracking Gun | 1 | 32768 | (2409, 5) | HA-07 |
| 5146 | Straegis beacon core | 1 | 32768 | none | HA-12 |
| 5736 | Lethander's Head | 1 | 34816 | none | HA-14 |

Item and event-set columns are FACT (H). Mission text and source claims below
come from rebuild and four-zone planning docs and the
[items research](../../../../.claude/reports/harset-continuation/research-items-advisor.md);
the source column is INFERENCE because no seed row supplies a source.

### Source type per item

| Item | Mission | Visible source | Basis | Conf |
|---|---|---|---|---|
| 2734 | 1352, 1362 | Quest-giver hand-over (x3 on accept in 1352; x4 in 1362) | Rebuild packets say granted on accept; the Security quest giver | L |
| 4690 | 1363 | Quest-giver hand-over of the tagging gun | Legacy ability 2409 "Use Tagging Gun" | M-L |
| 4396 | 1374 | Quest-giver hand-over of the scanner | Legacy ability 2092 "Use Straegis Scanner" | M-L |
| 2743 | 1365 | NPC hand-over (Tollan technology) | Mission text only; the giving NPC is undetermined | L |
| 2720 | 1365 | Kill or discovery reward (Dawson) | Rebuild packet: the Dawson kill grants it | L |
| 2818 | 741, 1353 | Pickup or `interact_tag` at the symbiote tank | Rebuild packet: tank `interact_tag` grants and advances | L |
| 5146 | 1241 | Dismantle the beacon (objective 5538) as a pickup, or a kill drop | Rebuild text "kill infiltrators and dismantle beacon" | L |
| 2823 | 1245 | Chain grant at the Petbe step | Rebuild packet 1245 | L |
| 2825 | 1245 | Chain swap from 2823 (grant 2825, remove 2823) | 2825 is arbitrary, D-H14 picks it | L |
| 5736 | 1247 | Kill drop from the Lethander clone, turn in to Anat | Rebuild packet 1247 | L |

Overall INFERENCE (L): the original source of each item is not recoverable
from the seed. The designs below are CIMMERIA_RECONSTRUCTION unless a
precedent chain makes them RESTORED_SOURCE-shaped; none is RESTORED_SOURCE
because no legacy chain for these items survives. Where a design needs
Harset NPC content (a quest giver, a tank prop, a corpse), the host entity
must exist in the owning packet's layout; if a pickup needs a new world prop
or a client change, that is a MAINTAINER DECISION, not assumed free.

### Beat design (shared template)

Each grant is one chain with one trigger, a persisted guard, and ordered
actions. Template, using the 6102 shape:

1. Trigger: `dialog_choice` (hand-over), `dialog_open` (pickup) or
   `entity_dead_tag` / `interact_tag` (kill, tank).
2. Condition: `step_status <mission> <step> eq active`, plus `world` where
   the chain must not fire in another world (shared worlds 57 and 68 actors
   are never mutated per player, so use a private clone or dialog sets, not
   the shared spawn).
3. Actions in order:
   - Remove the click or choice affordance first (`set_interaction_type`
     clear, or `remove_dialog_set`).
   - `add_item` (qty from the table).
   - `advance_step` to the next beat, which flips the guard.
   - Optional `add_dialog_set` for the follow-up.

Duplicate guard: the `advance_step` flip. Relog: the guard is persisted
mission state, so a relog after the grant sees the step past the guard and
grants nothing; a relog before the grant simply re-offers it. Never use
`once`. First-press feedback: the hand-over dialog line or step text change
is the visible acknowledgement, plus the item appearing in the mission bag;
a full-bag fallback to mail must also be visible (the GM feedback line). A
pickup must change or clear its interaction so the first press visibly
does something.

Consume point: only on the use or turn-in beat, via `remove_item` by type
after the beat's conditions hold, and before any `advance_step` that ends
the mission (item and bind cleanup goes before the mission mutation). Items
that are tools (4396, 4690, 2743) are reusable during their use beats
([HA-15e](HA-15e-event5-audit.md) lists them as `KNOWN_REUSABLES`: an
`item_use` chain on them never carries `remove_item`). If a packet consumes
one at all, it does so once, by an explicit chain at the turn-in or final
beat, not at the first use.

### 2734 Monitoring Device (stack 4)

- 1352 (HA-06b): grant qty 3 on `mission_accepted`, like 6101. Anchors at
  Market, Bar and Storage each `remove_item 2734` then `advance_step`.
- 1362 (HA-07): qty 4 on accept; four objectives at the OpCORE anchors.
- Stack hazard: max stack is 4. 3 plus 4 does not merge (7 > 4), so a
  player who still holds 1352 devices when 1362 starts gets a second row.
  Both missions consume by type, so the chain that removes must not assume
  one slot. If 1352 leaves unused devices, decide whether to clear them on
  1352 completion (INFERENCE M: recommended, `remove_item` by type with the
  remaining count before the mission-end mutation).
- Reaccept: `mission_accepted` re-fires only if a re-accept is allowed; the
  offer guard refuses it (FACT for the 6101 precedent), so no duplicate.

### 4690 Nanite Tracking Gun and 4396 Straegis Scanner (event 5)

- Grant at quest-giver dialog choice (6102 shape), guard on the offering
  step. The use beats are anchors or NPCs tagged with `interact_tag`, so the
  use is a target interaction, not `UseInventoryItem`.
- Both carry an event-5 binding (2409, 2092). If the use beat is an
  `item_use` chain with `remove_item`, the item has an `item_use` chain, so
  per `consumable_use.rs` a chain owns the item and the native path stands
  aside (FACT, H). Then `remove_item` is the single consume. If the use beat
  is a target interaction (no `item_use`), event 5 never fires and
  `remove_item` is again the single consume.
- Double-consume risk exists only if the item classifies as native
  (HealHealth, HealFocus or StatBuff scripts, ability not 597) and an
  `item_use` chain is absent while a `remove_item` is added. INFERENCE (M)
  from the audit: none of the ten classify as native today. HA-15e verifies
  this and owns the exact verdict; this note does not re-trace it.
- Unconfirmed: whether the use should trigger the real ability effect. That
  is O18-07 and HA-15e.

### 2743 Tollan Control Technology and 2720 Dawson's Symbiote (HA-08a, 1365)

- 2743 is held; the `interact_tag` on the Dawson instance clone (and on the
  two optional trooper tests, which HA-15e records as rebuild additions, not
  source) launches the detect effect. Grant 2743 at the Tollan hand-over. The
  giver is undetermined (INFERENCE L); HA-08a must name the NPC and dialog.
- Do not gate `interact_tag` on holding the item: there is no has-item
  condition. Gate on the mission step, and hand the item over at the step
  before. The item is the visible prop, not the guard.
- 2720 is granted by the Dawson kill or discovery beat, guarded by the step
  and removed at the turn-in beat. Dawson must be a per-player clone or the
  grant keyed on dialog, since shared actors are never mutated per player.
- Event 5 (2410) on 2743 is real only if HA-15e confirms it; otherwise leave
  unwired.

### 2818 Goa'uld Symbiote (HA-11, 741 and 1353)

- Tank pickup: `interact_tag` on the tank grants 2818 and advances, using the
  1003 shape: clear the interaction bit first, grant, advance.
- 1353 reuse is deferred (the owning packet for 1353 is not HA-11).
- Event 5 (2068) on 2818: if the use beat is a target interaction, event 5
  is not involved. If a later packet makes it an `item_use` chain, HA-15e
  governs consumption.
- Shared hazard: the tank must be a private clone or prop that every player
  can interact with independently; do not mutate a shared spawn per player.

### 5146 Straegis beacon core (HA-12, 1241)

- Visible pickup at the beacon (the dismantle objective 5538), 1003 shape.
  Alternate: kill drop via `open_loot` on a new loot table; that needs a
  loot-table row and an entity wired for it, which is a separate ID and seed
  edit. Prefer the pickup (fewer moving parts, INFERENCE M).
- The core is removed at the turn-in beat. No event set.

### 2823 and 2825 Petbe's robes (HA-13b, 1245)

- Grant 2823 at the Petbe step. Petbe is a private Storage clone (template
  221); the shared spawn 223 is immutable. Grant before any equip signal
  (the equip-from-inventory pattern): the robes are a mission-bag item, so
  "equip" here is whatever the chain triggers after the grant, never the
  reverse.
- 2825 replaces 2823: grant 2825, then remove 2823, then advance, in that
  order, so a failure between steps leaves the player holding at least one
  robe, not none.
- 2823 has the 597 filler binding. 597 is not a real effect and is never
  fired from an item, so there is no native consume on 2823 (FACT, H).
- 2825 is arbitrary by D-H14; its rows are byte-identical to 2831. Record
  this and keep the pick explicit.

### 5736 Lethander's Head (HA-14, 1247)

- Kill drop: death of the Lethander clone in the Market instance grants
  5736 via `entity_dead_tag`, guarded by the kill step. Turn-in to Anat at
  step 3934 removes it by type and advances.
- No event set. Stack 1. If the corpse is a shared actor, use a per-player
  clone (INFERENCE M).

## Hazards

- Double consume with event 5. Applies to 2743, 2818, 2823, 4396, 4690.
  See above. HA-15e owns the verdict. A grant chain must never pair
  `UseInventoryItem`/event 5 with `remove_item` on a native-classified item.
- Stack limits. Only 2734 has stack greater than 1 (max 4). Vendors are not
  a source: they ignore `max_stack_size` and have no faction gate, and
  worlds 69 and 70 have zero spawns.
- Full bag. A grant into a full mission bag goes to mail, silently
  from the quest's point of view. The beat must not advance on the
  assumption that the item is in the bag; INFERENCE (M): either accept the
  mail path as legitimate and say so in the step text, or add a step that
  tells the player. This is a design decision for the owning packet.
- Grant before equip. 2823 and 2825 only.
- Shared-world actors. Worlds 57 and 68 shared actors are never mutated per
  player. Every beat above that needs a per-player change uses a private
  clone, dialog set or `interact_tag`, not a shared spawn edit.
- No has-item condition. Turn-in cannot test possession. Use the step flip.
  A player who loses the item (for example, mail was auto-collected and
  discarded) cannot re-obtain it unless the design adds a re-grant beat;
  INFERENCE (M): add a recovery beat (re-offer dialog at the giver, guarded
  by the same step) where an item is mission-critical.
- `once` resets on relog; never use it as a guard.
- propId: none triggered. Bandolier: not in scope.
- Flags 32768, 34816, 35840, 36864: bit meanings unverified. Do not alter
  them in a seed change without a Ghidra or docs check.

## IDs

All ten items exist, so no `items.item_id` row is needed. Each owning
packet needs chain rows only.

- Item rows: none needed.
- Chains: `ID needed: chain` per owning packet (grant beats, use beats,
  turn-ins). Do not cite a range here; the packet author takes it from
  [id-allocation.md](../id-allocation.md).
- Loot table: only if a packet chooses a kill drop via `open_loot` for 5146
  (`ID needed: loot table`).

## Owning packets

| Packet | Items | Designs consumed |
|---|---|---|
| HA-06b | 2734 (1352) | Accept grant, anchor removal, reaccept guard |
| HA-07 | 2734 (1362), 4690 | Accept grant, stack handling, tagging gun |
| HA-08a | 2743, 2720 | Hand-over, kill reward, per-player Dawson |
| HA-09 | 4396 | Scanner hand-over, anchor uses |
| HA-11 | 2818 (741) | Tank pickup, 2820 neighbour |
| HA-12 | 5146 | Beacon pickup |
| HA-13b | 2823, 2825 | Grant, swap, grant before equip |
| HA-14 | 5736 | Kill drop, turn-in to Anat |

## Open questions

1. Giver NPC for 2743 and the exact source of 2720 (HA-08a).
2. Whether 1352 leftovers are cleared before 1362 (HA-06b, HA-07).
3. Whether each mission-critical item needs a re-grant recovery beat.
4. Whether 5146 is a pickup (preferred) or a kill drop (new loot table).
5. Whether any pickup requires a new world prop or a client change: that is
   a MAINTAINER DECISION and is not assumed here.

## Tests the implementation owes

Per [TESTING.md](../../../../TESTING.md) a runtime-behaviour change needs a
test that fails when the fix is reverted.

- Seed lint: every new grant chain has a persisted `step_status` or
  `mission_status` guard, never `once`, and the item exists with
  `container_sets {2}`. Fails if the guard is removed.
- Live-DB chain test: fire each grant beat twice. Assert exactly one item
  row and the step advanced once. Guards the duplicate-click and relog bug.
- Live-DB relog test: grant, relog, fire the beat again. Assert no second
  item. Guards a `once`-style guard.
- Consumption test per event-5 item: use then turn in. Assert exactly one
  item consumed and none remaining. Fails if `remove_item` is added next to
  a native consume (double consume).
- Stack test for 2734: hold 3 from 1352, accept 1362 (4). Assert two rows
  and that each removal takes one unit with no negative count.
- Full-bag test: fill the mission bag, fire a grant. Assert the mail path
  and that the step behaves as the packet designs it.
- Grant-then-equip test for 2823/2825: assert order grant, remove, advance.
- Wire: none (no serializer change).

## Telemetry the implementation owes

- Per grant beat: `chain_id`, `item_id`, `qty`, `container`, outcome
  (granted, merged, mailed, refused), mission id and step id.
- Per consume: `chain_id`, `item_id`, instance id or by-type, and the stack
  count before and after.
- Per full-bag fallback: a counter with the item id, so mailed mission items
  are visible in SigNoz.
- Per guard rejection: chain id and the failing condition, so a duplicate
  click is distinguishable from a missed trigger.
