---
title: "Stasis-Room Debug Hub"
type: reference
audience: engineers, testers
last_updated: 2026-09-26
---

# Stasis-Room Debug Hub

Five NPCs in the Castle_CellBlock stasis room let a tester exercise one server
system each from a single spot. The stasis room is where every new character
wakes up, so the hub is reachable a few seconds after character creation, with
no travel and no mission state.

All five are ordinary seeded spawns. Every player sees them. This was an owner
decision (2026-09-26): `spawnlist` has no dev or enabled column, and the owner
chose visible-to-all over gating. Compare Harset packet H13, which deleted the
old debug NPCs (templates 23 and 25) from the Harset gate plaza because they
stood in the arriving player's face. The hub NPCs stand against the room's back
wall instead.

## Where it is

World 12 (`Castle_CellBlock`), point set 2032 `Castle_Cellblock.Region1`.
Respawner 8 'Stasis Chamber' is at (-334.23, 73.47, -228.03). The NPCs stand in
a line 3 units in from the wall between corners A (-347.17, -230.14) and
B (-327.67, -240.70), 3 units apart, facing the centre of the room. The slot
nearest the respawner is left empty, so the closest NPC is 5.5 units from where
a new character appears.

| Spawn | Template | Tag | Name shown | Position (x, y, z) | Heading |
|---:|---:|---|---|---|---:|
| 400 | 300 | `DebugHub_Vendor` | Basic Equipment Quartermaster | (-343.90, 73.47, -228.49) | 1.4638 |
| 401 | 301 | `DebugHub_Trainer` | Archetype Skills Trainer | (-341.26, 73.47, -229.92) | 1.2651 |
| 402 | 302 | `DebugHub_DialogNpc` | Airman Lance | (-338.63, 73.47, -231.35) | 0.9473 |
| 403 | 303 | `DebugHub_LivewireTerminal` | Terminal | (-333.35, 73.47, -234.21) | 0.0464 |
| 404 | 304 | `DebugHub_LootCrate` | Crate | (-330.72, 73.47, -235.64) | -0.2710 |

The names are monikers the client PAK already ships. A new `texts.sql` id
cannot render, so the templates reuse existing ones.

> **Placement is unchecked in the client.** There is no navmesh or occluder
> data for this room. The coordinates are derived from the region corners and
> the respawner's floor height. Check in the client that each NPC stands on the
> floor, clear of the walls and the stasis pods, before relying on the layout.

## What each NPC tests

### Vendor (template 300)

Right-click opens the store. The lists are template 25's: buy list 1, and list
2 for sell, repair and recharge.

- Tests the store open (`onStoreOpen`), buy, sell, buyback, paid repair and
  paid recharge.
- The click routes to the store because the template carries
  `INT_VendorGeneral` (65536). `spawn_npc_from_record_into` derives
  `NpcInteractionType::Vendor` from any `INT_Vendor*` bit. Before the hub,
  nothing set that value, so a vendor-only template did nothing when clicked.
  Template 25 only ever worked because its trainer list answered the click
  first.
- Buy list 1's second row costs one item 55 (SI 3 9mm Pistol) as well as
  naquadah. Mission 622 hands out that pistol.

### Ability trainer (template 301)

Right-click opens the trainer window with trainer list 1, which offers every
tree node for every seeded archetype.

- Tests `onTrainerOpen`, `trainAbility` and the per-row trainable flags.
- Tests respec: the click pins this NPC as the player's interaction target,
  and AT-08's gate for `resetMyAbilities` (cell method 72) accepts a pinned
  trainer within interact range. Respec costs 1000 naquadah and needs at least
  one trainer-bought ability.

### Dialog NPC (template 302, Airman Lance)

Right-click shows dialog 100100. These dialogs are Cimmeria-authored. The
client draws them from `DIALOG_OVERRIDES` in
`crates/resources/src/base/dialog_overrides/mod.rs`.

1. Dialog 100100 has two screens, so Next pages through them. Its one button,
   "Send my choice" (Generic 1, ButtonID 8), is on the final screen.
2. Clicking it fires chain 7002, which shows dialog 100101.
3. Dialog 100101 has no buttons. Closing it sends `dialogButtonChoice(100101,
   -1)`, which fires chain 7003. Chain 7003 prints "Dialog round trip complete"
   in chat as Airman Lance.

This tests paging, a button click, a button-less close, the server's
offered-dialog check on `dialogButtonChoice`, and the `last_interaction_target`
pin. Chain 7002's event carries no NPC, so dialog 100101 finds its speaker only
through that pin.

Closing 100100 with X instead of the button sends nothing (a dialog with any
button sends nothing on close). Nothing happens, and clicking the NPC again
starts over. There is no branching on which button was pressed, because no
content condition can read the button id.

The NPC shows a side-quest "?" (`INT_NonAStoryMissionAvaliable`). That bit is
what makes the client send the click at all. There is no mission behind it.

### Livewire terminal (template 303)

Right-click starts a Livewire session (chain 7004, difficulty 1). Winning fires
chain 7005, which prints "Livewire round trip complete" in chat as Terminal.

- Tests the minigame round trip: ticket, SmartFox handshake, the server-built
  board, result validation, and the victory callback into the content engine.
- The terminal can be hacked any number of times. Its `INT_MinigameLivewire`
  bit is a permanent template default, not the set/clear pair a mission console
  uses.

### Loot crate (template 304)

Loot is rolled only when a mob dies, so the crate is a mob you kill. Shoot it,
then right-click the corpse and use Loot All. It respawns 30 seconds after
death.

- Tests loot generation from a real table, the loot window, and taking items
  and naquadah. Loot table 3 drops, every time: 2-3 Health Slappack TC1
  (stackable), one Processor (Electronics), one Cell (Bio-Medical) and 25-75
  naquadah. Every row is probability 1. A roll that drops nothing would leave
  the corpse unclickable, which would look like a broken loot path.
- The crate is faction 10, because that is the only faction a player can
  damage. It is not a threat to a new character:
  - The spawn row sets NEUTRAL aggression, so it never attacks on proximity.
  - It is stationary.
  - Ability set 6 holds only 710 'Staff Melee AA', whose effect deals no
    damage.
  - It still turns to face you, and swings at melee range, once hit.
- Level 1 means 250 HP, about 10 to 25 pistol shots. A GM `.kill` goes through
  the same death path, loot included.
- Shooting it puts you in combat. Kill it or walk out of its range to leave
  combat.

## What the hub cannot test, and why

| System | Why there is no hub NPC |
|---|---|
| Mail | The mail window opens from the client UI, not from a mailbox or NPC, so there is nothing for the hub to add. Only the read side works on the server; sending is a stub ([gap-analysis.md §24](../gap-analysis.md)). |
| Bank | Known missing on the server. Nothing handles an `INT_Banker` click, and the organization vault is also missing ([gap-analysis.md §23](../gap-analysis.md)). |
| Guilds / organizations | Known missing on the server ([gap-analysis.md §23](../gap-analysis.md)). |
| Black market | Known missing on `main` ([gap-analysis.md §25](../gap-analysis.md)). |
| Crafting | Known missing: the crafting verbs are still stubs ([gap-analysis.md §19](../gap-analysis.md)). |
| Pets | Known missing on the server ([gap-analysis.md §28](../gap-analysis.md)). |
| Player-to-player trade | Needs two players. An NPC cannot be a trade partner ([gap-analysis.md §22](../gap-analysis.md)). |

## Where it lives

| What | Where |
|---|---|
| Templates 300-304 | `db/resources/Entities/Seed/entity_templates.sql` |
| Spawns 400-404 | `db/resources/Worlds/Seed/spawnlist.sql` |
| Chains 7001-7005 | `db/resources/Content/Seed/debug_hub_chains.sql` |
| Dialogs 100100-100103, screens 200000-200004, button 200000 | `db/resources/Dialogs/Seed/` and `DIALOG_OVERRIDES` |
| Loot table 3 (loot rows 14-17) | `db/resources/Loot/Seed/` |
| Ability set 6 | `db/resources/Abilities/Seed/ability_sets.sql`, `ability_set_abilities.sql` |
| Vendor derivation | `static_interaction_for_flags` in `crates/cell-world/src/cell/space_manager/spawn.rs` |

Every seed row is commented `NEW CONTENT (debug hub)`.

## Tests

| Guard | What it pins |
|---|---|
| `cell-catalog` `spawner/tests/live_db_debug_hub.rs` | Role columns of each template; spawns inside Region1, on the floor, at least 5 units from the respawner and 2.5 from each other; the crate's ability set is exactly `[710]` and deals no damage; trainer list 1; vendor lists and loot table 3 name real items, with every loot row at probability 1; dialog screens and buttons, and neither dialog is a monologue |
| `cell-methods` `interaction/debug_hub_dispatch_tests.rs` | Each NPC, spawned from its real row, answers a right-click with its own interaction; the crate reroutes to an attack while alive and shows table 3's loot when dead; respec passes at the hub trainer and is refused at the vendor |
| `cell-content` `chain_replay_tests/debug_hub.rs` | Chains 7001-7005 resolve and execute: the `onDialogDisplay` speakers, the `StartMinigame` message, and both chat lines |
| `cell-world` `tests/npc_spawn.rs` | Any `INT_Vendor*` bit derives `Vendor`; no other bit derives anything |
| `resources` `dialog_overrides/override_seed_agreement_debug_hub.rs` | The overrides and the dialog seed agree screen for screen and button for button |
| `content-engine` `interact_tag_linter`, `dialog_button_linter` | Chains 7001 and 7004 are allowlisted (template-default bits); the hub dialogs obey the button hard rules |
