---
title: "Stasis-Room Debug Hub"
type: reference
audience: engineers, testers
last_updated: 2026-09-27
---

# Stasis-Room Debug Hub

Eight NPCs in the Castle_CellBlock stasis room let a tester exercise one server
system each from a single spot. The stasis room is where every new character
wakes up, so the hub is reachable a few seconds after character creation, with
no travel and no mission state.

All eight are ordinary seeded spawns. Every player sees them. This was an owner
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
| 450 | 360 | `DebugHub_PetTrainer` | Goa'uld Advanced Skills | (-328.08, 73.47, -237.07) | -0.4698 |
| 470 | 370 | `DebugHub_Banker` | Storage Officer | (-325.92, 73.47, -231.18) | -1.0739 |
| 490 | 390 | `DebugHub_MailClerk` | Sgt. Harriman | (-324.11, 73.47, -227.84) | -1.5123 |

The pet trainer (pets campaign PT-07) takes the next slot on the line after the
crate, about 2.1 units in from the room's B-C wall. Its ids come from the pets
block (templates 350-369, spawns 450-469), not the hub's 300-304 and 400-404.

The Banker (bank campaign BV-04) is not on that line. The line is full up to
corner B, and the crafting stations take the A-D side of the room. The Banker
stands at the middle of the B-C wall, 3 units in from it. That puts it 8.9
units from the respawner, at least 6.3 from every other hub NPC and 9.2 from
the C-D exit wall. Its ids come from the bank block (templates 370-389, spawns
470-489).

The Gate Mail Clerk (social-systems SS-U3) stands on the same B-C wall,
past the Banker towards corner C: 3 units in from the wall and 13 units
along it from B. That puts it 3.8 units from the Banker, 5.4 from the C-D
exit wall and 10.1 from the respawner. Its ids come from the social block
(templates 390-399, spawns 490-499).

The names are monikers the client PAK already ships. A new `texts.sql` id
cannot render, so the templates reuse existing ones. No shipped moniker says
"Pet Trainer", so template 360 shows "Goa'uld Advanced Skills" (8000). The
Banker uses the Omega Site banker's own name, "Storage Officer" (29462). No
shipped moniker says "Mail Clerk", so template 390 shows "Sgt. Harriman"
(26715), with Walter Harriman's SGC_W1 look (template 58). The dialog and the
mail's sender name say "Gate Mail Clerk".

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

### Pet trainer (template 360)

Right-click opens the trainer window with trainer list 350: the Goa'uld
Servant Lord pet nodes, 2826 Summon Straegis first (the first pet, D-PT13), then
1643 Summon Jaffa, 1644, 1645, 1652 and 1654 for the later pets.

- The ability-tree gates still apply: archetype tree, level, prerequisites and
  branch spend. List 350 is keyed to the Goa'uld only, the one tree that holds
  these nodes, so any other archetype sees an empty trainer. A Goa'uld sees
  1643 trainable at level 1 with a training point; 2826 is the level 50
  Servant Lord capstone (prerequisites 1643, 2069 and 2846, 20 branch points).
- **For UAT, grant the summon directly** with the GM console:
  `.giveability 2826`, then cast it, or skip the cast with `.pet summon 2826`.
  `.giveability` saves the ability to the character. See
  [commands.md](../commands.md#dev-console--commands).
- Template 360 is an ordinary placed trainer (class `mob`, faction 1, no
  ability set). It is not a pet: 350-359 are the pet templates and 360-369
  the pets campaign's placed NPCs.

### Banker (template 370)

Right-click opens your personal vault (`onVaultOpen`).

- Tests the Banker open path: the vault session, pinned to this Banker, and
  the proximity check it carries into every vault move. Any later
  right-click on another NPC ends the session.
- The click routes to the vault because the template carries `INT_Banker`
  (2): `spawn_npc_from_record_into` derives `NpcInteractionType::Banker`
  from it, with the template's `vault_scope`, here `personal`. Nothing else
  on the template answers a click first: no trainer list, no vendor lists
  or bits, no dialog or chain.
- The Banker is faction 1 and cannot die. A death would replace its Banker
  interaction with a loot window until the server restarts.
- The window's size is the character's own `sgw_player.bank_slots`,
  40 by default.
- To see what a vault holds without opening it, or to check another
  character's, a GM types `.bankdump [name]`. Nothing can grant an item
  straight into the vault (the grant path refuses 17 by design); a tester
  drags items in from the main bag once BV-03 lands. See
  [commands.md](../commands.md#dev-console--commands).

### Gate Mail Clerk (template 390)

Right-click shows dialog 100104, which has one button, "Send me a mail".
Pressing it mails you a test mail from "Gate Mail Clerk" with 5 Health
Slappack TC1 and 50 naquadah. A chat line names the mail. Open the mail window
and take the naquadah and the slappacks.

- Tests the mail window with a system mail: the header, take-cash and
  take-item (SS-M3). A tester needs no GM rights.
- **One mail per character every 10 minutes.** Pressing the button again
  inside the window mails nothing, and a chat line says how long to wait
  ("You can ask again in 7 minutes"). The window is stored in the database
  (`sgw_player_content_cooldown`), so it holds across a relog, a server
  restart and deleting the mail.
- The mail has no sender character, so it cannot be returned, and its cash is
  not a COD. For COD and return, use `.mail ... cod <n>` or two players (see
  below).
- Closing the dialog with X sends nothing, so nothing is mailed. Click the
  clerk again to get the dialog back.
- Chain 7010 opens the dialog. Chain 7011 runs the `send_system_mail` content
  action ([content-engine.md](content-engine.md#send_system_mail-params)).
- Deviation from plain intent: none. The cooldown is per character, not per
  account, because mail is per character.

## What the hub cannot test, and why

| System | Why there is no hub NPC |
|---|---|
| Mail (partly) | The Gate Mail Clerk covers receiving a mail and taking its cash and item. Sending mail, postage, COD, return and expiry need a sender, not an NPC: send from the mail window to a second character, or as a GM use `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>]`, `.mailbox [name]` and `.mail_expire <mailId>` ([commands.md](../commands.md#command-families)). The two-player checks are SS-UAT steps 1-6 ([work-packets.md](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release)). |
| Chat | Tells and ignore need a second player; an NPC does not chat. Alone you can still check the refusals: a tell to your own name ("You cannot send a tell to yourself.") or to an offline name, and the flood limit (paste ten lines into say). As a GM, `.mute <name> <minutes>` and `.unmute <name>` mute a player, and `.announce` (or `/gmshout`) broadcasts ([commands.md](../commands.md#command-families)). The two-player checks are SS-UAT steps 7-10. |
| Duels | A duel needs a second player. Alone, run `sparbot`, a second account that accepts every challenge and forfeits after a set time ([wireclient.md](../architecture/wireclient.md#sparbot-a-duel-partner-for-solo-testing)). As a GM, `.duel_status [name]` shows a duel's stage and `.duel_end <name>` ends it. The duel checks are SS-UAT steps 11-13. |
| Bank (partly) | The Banker (template 370) opens the personal vault. Moving items into and out of it is the bank campaign's BV-03, the vault-size purchase BV-05, and the Squad, Team and Command vaults BV-07 ([docs/analysis/bank-vault/](../analysis/bank-vault/README.md), [gap-analysis.md §23](../gap-analysis.md)). No hub NPC is an organization Banker: those need an organization, which the hub cannot give a new character. |
| Guilds / organizations | Known missing on the server ([gap-analysis.md §23](../gap-analysis.md)). |
| Black market | Known missing on `main` ([gap-analysis.md §25](../gap-analysis.md)). |
| Crafting | Known missing: the crafting verbs are still stubs ([gap-analysis.md §19](../gap-analysis.md)). |
| Pets (partly) | The pet trainer (template 360) sells the summon abilities, but a pet needs a tester, not a hub NPC: summon with `.pet summon 2826` or the ability (`.giveability 1643`, `1644` or `1645` for the Jaffa, Lo'taur or Prime), then walk, fight and change stance. Every pets-campaign packet has merged; the owner's UAT checklist is in [the pets session resume](../analysis/pets/handoffs/session-resume.md) ([docs/analysis/pets/](../analysis/pets/README.md), [gap-analysis.md §28](../gap-analysis.md)). |
| Player-to-player trade | Needs two players. An NPC cannot be a trade partner ([gap-analysis.md §22](../gap-analysis.md)). |

## Where it lives

| What | Where |
|---|---|
| Templates 300-304, 360, 370, 390 | `db/resources/Entities/Seed/entity_templates.sql` |
| Spawns 400-404, 450, 470, 490 | `db/resources/Worlds/Seed/spawnlist.sql` |
| Trainer list 350 | `db/resources/Abilities/Seed/trainer_ability_lists.sql`, `trainer_abilities.sql` |
| Chains 7001-7005, 7010-7011 | `db/resources/Content/Seed/debug_hub_chains.sql` |
| Dialogs 100100-100104, screens 200000-200005, buttons 200000-200001 | `db/resources/Dialogs/Seed/` and `DIALOG_OVERRIDES` |
| The clerk's mail and cooldown | `mail/content.rs` in `crates/base-methods`; table `db/sgw/Players/Tables/sgw_player_content_cooldown.sql` |
| Loot table 3 (loot rows 14-17) | `db/resources/Loot/Seed/` |
| Ability set 6 | `db/resources/Abilities/Seed/ability_sets.sql`, `ability_set_abilities.sql` |
| Vendor and Banker derivation | `static_interaction_for_flags` in `crates/cell-world/src/cell/space_manager/spawn.rs` |

Every seed row is commented `NEW CONTENT (debug hub)`, except the pet
trainer's, which are commented `Pets campaign, PT-07`, and the Banker's,
commented `Bank and Vault campaign, BV-04`, and the mail clerk's,
commented `Social-systems campaign, SS-U3`. The `trainer_abilities`
rows carry no comment: that file is regenerated by
`tools/ability_trees/generate_seed.py`, which keeps other lists' rows but not
comments.

## Tests

| Guard | What it pins |
|---|---|
| `cell-catalog` `spawner/tests/live_db_debug_hub.rs` | Role columns of each template; spawns (the mail clerk's 490 included) inside Region1, on the floor, at least 5 units from the respawner and 2.5 from every other spawn in the room; the crate's ability set is exactly `[710]` and deals no damage; trainer list 1; vendor lists and loot table 3 name real items, with every loot row at probability 1; dialog screens and buttons, and neither dialog is a monologue |
| `cell-methods` `interaction/debug_hub_dispatch_tests.rs` | Each NPC, spawned from its real row, answers a right-click with its own interaction (the mail clerk's dialog included); the crate reroutes to an attack while alive and shows table 3's loot when dead; respec passes at the hub trainer and is refused at the vendor; the pet trainer opens list 350 for a Goa'uld and an empty list for anyone else; the Banker opens the personal vault, pinned to itself |
| `cell-catalog` `spawner/tests/live_db_debug_banker.rs` | Template 370 is a personal Banker and nothing else: exactly `INT_Banker`, `vault_scope = 'personal'`, a shipped name, no trainer list or vendor lists, not faction 10; spawn 470 inside Region1, on the floor, clear of the respawner and of every other NPC in the room |
| `cell-catalog` `spawner/tests/live_db_pet_trainer.rs` | Template 360's role columns and name; spawn 450 inside Region1, on the floor, clear of the respawner and the other hub NPCs; list 350 keyed to the Goa'uld only, with exactly the six pet nodes, each a Goa'uld tree node |
| `cell-catalog` `spawner/tests/live_db_mail_clerk.rs` | Template 390's role columns and name; dialog 100104 is one clerk screen with one Generic 1 button, and not a monologue |
| `cell-content` `chain_replay_tests/debug_hub.rs` | Chains 7001-7005 resolve and execute: the `onDialogDisplay` speakers, the `StartMinigame` message, and both chat lines |
| `cell-content` `chain_replay_tests/debug_hub_mail_clerk.rs` | Chain 7010 opens 100104 as the clerk; chain 7011 sends the base exactly one `ContentSystemMail` with the seeded contents and cooldown |
| `base-methods` `mail/tests/content_live.rs` | One mail per press, refused with the wait inside 10 minutes; the window outlives the mail and ends on time; the claim rolls back with a refused mail; two simultaneous presses write one mail |
| `cell-world` `tests/npc_spawn.rs` | Any `INT_Vendor*` bit derives `Vendor`; no other bit derives anything |
| `resources` `dialog_overrides/override_seed_agreement_debug_hub.rs` | The overrides of 100100, 100101 and 100104 and the dialog seed agree screen for screen and button for button |
| `content-engine` `interact_tag_linter`, `dialog_button_linter` | Chains 7001, 7004 and 7010 are allowlisted (template-default bits); the hub dialogs obey the button hard rules |
