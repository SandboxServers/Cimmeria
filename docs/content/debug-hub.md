---
title: "Stasis-Room Debug Hub"
type: reference
audience: engineers, testers
last_updated: 2026-09-27
---

# Stasis-Room Debug Hub

Twelve NPCs in the Castle_CellBlock stasis room let a tester exercise one server
system each from a single spot. A second group along the opposite wall, four
crafting stations and a crafting supplies vendor, covers crafting (see
[Crafting corner](#crafting-corner)). The stasis room is where every new character
wakes up, so the hub is reachable a few seconds after character creation, with
no travel and no mission state.

All twelve are ordinary seeded spawns. Every player sees them. This was an owner
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
| 430 | 330 | `DebugHub_TeamRegistrar` | Organization Registrar | (-327.40, 73.47, -233.91) | -0.7086 |
| 431 | 331 | `DebugHub_CommandRegistrar` | Organization Registrar | (-330.04, 73.47, -232.48) | -0.5276 |
| 450 | 360 | `DebugHub_PetTrainer` | Goa'uld Advanced Skills | (-328.08, 73.47, -237.07) | -0.4698 |
| 470 | 370 | `DebugHub_Banker` | Storage Officer | (-325.92, 73.47, -231.18) | -1.0739 |
| 471 | 371 | `DebugHub_TeamBanker` | Storage Officer | (-327.65, 73.47, -228.09) | -1.4294 |
| 472 | 372 | `DebugHub_CommandBanker` | Storage Officer | (-325.84, 73.47, -224.74) | -1.9148 |
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

The Team and Command registrars (organizations ORG-05) stand as a pair between
the pet trainer and the Banker. The Team registrar is on the B-C wall, 3 units
in from it and 6.1 units along it from B. The Command registrar is 3 units
further into the room, level with it. The pair is 3.0 units apart, at least
3.1 from every other NPC (the Banker, the pet trainer and the crate) and 6.1
from the respawner. Their ids come from the organizations block (templates
330-349, spawns 430-449).

The Team and Command Bankers (bank campaign BV-10a) stand in a second row, 6
units in from the B-C wall, because that wall's line is full from corner B to
the mail clerk. Each stands in the gap between two wall NPCs, so none hides
another from the room centre: the Team Banker between the Storage Officer and
the mail clerk (11.1 units along the wall from B), the Command Banker past the
mail clerk (14.9 along). The Team Banker is 3.55 units from the Storage Officer
and from the mail clerk, 3.8 from the Command Banker, 5.0 from the Command
registrar and 6.6 from the respawner. The Command Banker is 3.55 from the mail
clerk, 6.4 from the Storage Officer, 3.5 from the C-D exit wall and 9.0 from
the respawner. Their ids come from the bank block.

The names are monikers the client PAK already ships. A new `texts.sql` id
cannot render, so the templates reuse existing ones. No shipped moniker says
"Pet Trainer", so template 360 shows "Goa'uld Advanced Skills" (8000). The
Banker uses the Omega Site banker's own name, "Storage Officer" (29462), and so
do the Team and Command Bankers: no shipped moniker names a Team or Command
banker. Their bodies tell the three apart. The Storage Officer is the woman in
SGC uniform, the Team Banker wears the Cellblock guards' SGC uniform (template
15, without the pistol) and the Command Banker the plain crew clothes of the
crafting supplies vendor (template 314). No
shipped moniker says "Mail Clerk", so template 390 shows "Sgt. Harriman"
(26715), with Walter Harriman's SGC_W1 look (template 58). The dialog and the
mail's sender name say "Gate Mail Clerk". Both registrars use the Omega Site
registrar's name, "Organization Registrar" (29068), which covers both types;
the Team registrar wears the dialog NPC's SGC uniform and the Command
registrar the trainer's armour.

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

> **Quarantined (2026-09-27).** Dialogs 60100, 60101 and 60104 are not
> served to clients. Their definitions sit in `QUARANTINED_DIALOG_OVERRIDES`,
> not `DIALOG_OVERRIDES`. After they started going out as cooked-data
> overrides, a tester's client crashed on every entry into Castle_CellBlock:
> it died after `onClientMapLoad` and never sent `mapLoaded`. Renumbering
> them below 65536 did not help, and the field that crashes the client is
> not yet known. The chains, the seed rows and the NPCs stay. A right-click
> still sends `onDialogDisplay`, but the client has no entry for the id, so
> **the Dialog NPC and the Gate Mail Clerk show no dialog**. Use
> `.mail` for the mail test until the quarantine is lifted.
>
> The server evicts nothing from the client's dialog cache. A client that
> already received these dialogs (as 100100/100101 or 60100-60104) keeps
> them on disk and keeps crashing. Delete `Cache.en-US\CookedDataDialogs.pak`
> in the client folder; the client rebuilds it at the next login.

Right-click shows dialog 60100. These dialogs are Cimmeria-authored. The
client draws them from the dialog overrides in
`crates/resources/src/base/dialog_overrides/mod.rs`.

Cimmeria-authored dialog ids live in 60100-60199 and must stay at or below
65535. The hub's dialogs were first numbered 100100-100104, and pushing
100100 and 100101 as overrides crashed the client while it loaded
Castle_CellBlock. The renumber did not stop the crash (see the quarantine
note above). The client's own dialog ids stop at 6427. The guard is
`every_cooked_override_element_id_fits_in_16_bits` in `crates/resources`; the
evidence is in
`docs/reverse-engineering/findings/cooked-dialog-override-crash.md`.

1. Dialog 60100 has two screens, so Next pages through them. Its one button,
   "Send my choice" (Generic 1, ButtonID 8), is on the final screen.
2. Clicking it fires chain 7002, which shows dialog 60101.
3. Dialog 60101 has no buttons. Closing it sends `dialogButtonChoice(60101,
   -1)`, which fires chain 7003. Chain 7003 prints "Dialog round trip complete"
   in chat as Airman Lance.

This tests paging, a button click, a button-less close, the server's
offered-dialog check on `dialogButtonChoice`, and the `last_interaction_target`
pin. Chain 7002's event carries no NPC, so dialog 60101 finds its speaker only
through that pin.

Closing 60100 with X instead of the button sends nothing (a dialog with any
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
  naquadah. Those rows are probability 1, so the corpse always has loot: a
  roll that drops nothing would leave the corpse unclickable, which would
  look like a broken loot path. It also drops each of the five Racial
  Paradigm Guides (7805-7809) and Blueprint: Steel Plating (6483) at 0.2, one
  of each at most. The Processor, the Cell, the guides and the blueprint are
  `{17,15}` items and land in the crafting bag; use a guide or the blueprint
  from there.
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

### Team and Command Bankers (templates 371 and 372)

Right-click the Team Banker to open your Team's vault (`onTeamVaultOpen`), or
the Command Banker to open your Command's (`onCommandVaultOpen`). If you are in
no Team (or no Command), a line says so and nothing opens.

- Tests the org vault open round trip: the cell asks the base, the base checks
  your membership under the organization lock and sends the vault's size and
  contents, and the cell opens the window pinned to this Banker. Then the
  moves in, out and within the vault, which check your rank's bank bits on
  every move, and the fan-out to the other online members.
- The click routes to the org vault because the template carries
  `INT_Banker` (2) with `vault_scope` `team` (371) or `command` (372).
  Nothing else on either template answers a click first, and both are
  faction 1 and cannot die.
- Opening needs no bank bit: every member may look. Depositing needs
  `DepositBank`, which every default rank holds. Withdrawing needs
  `WithdrawBank`, which only the Leader holds by default (D-ORG21).
- The Team vault is 40 slots until the Team buys more (BV-09). The Command
  vault is always 100.

To test you need a Team or a Command, and for most checks a second character:

1. Found one at the registrars (spawns 430 and 431), or as a GM type
   `.org_create team <name>` or `.org_create command <name>`. You lead it, and
   the Leader holds every bit.
2. Find its id with `.org_info` (it also shows each rank's permission mask in
   hex) or `.org_list`.
3. Put a second, online character in it with `.org_join <orgId> <name>`. It
   joins at the entry rank (Team Member, Command Initiate), which can deposit
   but not withdraw.
4. To let that rank withdraw, add `WithdrawBank` (0x20000) to its mask with
   `.org_set_perms <orgId> <rank> <mask>`, where `<mask>` is the mask
   `.org_info` shows plus 0x20000. `.org_rank <name> <rank> [orgId]` moves a
   member to another rank.
5. To deposit, use anything carried that the personal vault would take and
   that is not bound. The crafting supplies vendor sells stackable items at 1
   naquadah each; they land in the crafting bag, which is a valid source.
6. `.org_disband <orgId>` is refused while the vault holds anything. Empty it
   first.

See [commands.md](../commands.md#command-families) for the `.org_*` commands
and [inventory-system.md](../gameplay/inventory-system.md#moving-items-in-and-out-of-a-team-or-command-vault)
for every move rule and refusal reason.

## Crafting corner

Four crafting stations and a crafting supplies vendor stand along the room's
D-A wall, D(-338.39, -213.94) to A(-347.17, -230.14), 3 units in from it and
2.6 units apart, the vendor nearest A. They were added by the crafting
campaign (`docs/analysis/crafting/`, packet CR-11), whose id blocks are
templates 310-329 and spawns 410-429. Their spawn tags start with `CraftHub_`,
not `DebugHub_`, so the hub's own guards, which count `DebugHub_` tags, stay as
they were.

| Spawn | Template | Tag | Name shown | Position (x, y, z) | Heading |
|---:|---:|---|---|---|---:|
| 414 | 314 | `CraftHub_Supplies` | Common Materials Components | (-341.67, 73.47, -226.29) | 1.6889 |
| 410 | 310 | `CraftHub_Station_BioMedical` | BioMedical Crafting Station | (-340.43, 73.47, -224.01) | 1.9913 |
| 411 | 311 | `CraftHub_Station_Electronics` | Electronics Crafting Station | (-339.20, 73.47, -221.72) | 2.3079 |
| 412 | 312 | `CraftHub_Station_PowerSystems` | Power Systems Crafting Station | (-337.96, 73.47, -219.44) | 2.5830 |
| 413 | 313 | `CraftHub_Station_Materials` | Materials Crafting Station | (-336.72, 73.47, -217.15) | 2.7937 |

The nearest is 7.4 units from the respawner and 3.1 from the hub vendor
(spawn 400). The same placement warning applies: nothing here has been checked
in the client.

### Crafting stations (templates 310-313)

Stand within 5 units of any station and open the crafting window (J). The
window shows the station as the machine, and every crafting page is enabled.

- Each station carries all four `ENTITYFLAG_Craft_*` bits (craft 2048,
  research 4096, reverse engineering 8192, alloying 16384), so any one of them
  enables every page. The name is only a label. The station gate does not
  check the science.
- A station is found by proximity. The cell's 1 Hz station tick reports the
  nearest station per verb to the base, which sends
  `onUpdateCraftingOptions` (140). There is nothing to click: a station has no
  interaction bit. The `INT_Machine_*` bits would give a cursor and a minimap
  icon, but nothing on the server answers a click on one, and the hub's rule
  is that every click gets feedback.
- The names are the client's own monikers (27180, 27182, 27184, 27186). The
  mesh is the Cellblock terminal screen of template 19, which already renders
  in this world.

### Crafting supplies vendor (template 314)

Right-click opens a store that only sells, at 1 naquadah each (buy list 310):

| Group | Items |
|---|---|
| UAT recipe components | 5254 Steel Core, 5256 Titanium Core, 5401 Titanium Plating, 5192 Cell (Bio-Medical), 5189 T1 Cell (Bio-Medical) |
| Research and reverse-engineering target | 5481 Crafted Pistol of the Whale |
| Research kickers | 5668 BioMedical, 5669 Electronics, 5670 Power Systems, 5671 Materials |
| Field Crafting Tools | BMAS-5 (5369), BMAS-50 (8415), EAS-5 (8402), EAS-50 (8441), PSAS-5 (8405), PSAS-50 (8461), MAS-5 (8406), MAS-50 (8451) |
| Racial Paradigm Guides | 7805 Human, 7806 Common, 7807 Asgard, 7808 Goa'uld, 7809 Ancient |
| Blueprint item | 6483 Blueprint: Steel Plating (teaches blueprint 25) |

- A purchase lands in the first carried bag the item lists: the crafting bag
  (15) for the `{17,15}` supplies, tools included, and the main bag (1) for
  anything else. The crafting verbs consume components from the main and
  crafting bags, and a Blueprint item or a guide is used from either bag, so
  these work straight away.
- A Field Crafting Tool counts only in the crafting bag (15). Move it there to
  enable crafting without a station.
- The vendor has no sell, repair or recharge list.
- The name is the client's "Common Materials Components" vendor moniker
  (27239). No moniker in the client says "Crafting Supplies".

### A crafting test run

1. Buy 13 Steel Cores and Blueprint item 6483, then use the Blueprint item.
   Blueprint 25 (Steel Plating) is now known. Instead, a GM can run
   `.learnblueprint 25` on the tester, and `.craftkit 25` to grant the 13
   cores into the crafting bag.
2. Learn discipline 78 (Materials Engineering) at the discipline trainer
   (Ctrl+J). Every character starts with the ASP and the Common paradigm level
   it needs.
3. Stand next to a station, open the crafting window and craft blueprint 25.

The other UAT recipes (blueprints 412 and 161, alloy 42, research and reverse
engineering with the kickers) are in section 2 of
`docs/analysis/crafting/audit.md`.

### Gate Mail Clerk (template 390)

> **Quarantined (2026-09-27).** Dialog 60104 is not served, so the clerk
> shows no dialog and nothing can be mailed from it. See the quarantine note
> under [Dialog NPC](#dialog-npc-template-302-airman-lance).

Right-click shows dialog 60104, which has one button, "Send me a mail".
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

### Team and Command registrars (templates 330 and 331)

Right-click the Team registrar to found a Team, or the Command registrar to
found a Command. The client's naming window opens (`CreateTeamWin` or
`CreateCommandWin`); type a name and confirm. The organization window then
opens with you as its leader, and a line says the organization was founded.

- Tests the founding flow end to end: the registrar click, the base's
  eligibility check, the naming dialog (`launchOrganizationCreation`), the
  name (`onOrganizationCreation`), and the founder's window
  (`onOrganizationCreationResult`, `onOrganizationJoined`, the name, the rank
  permissions and the roster). Creation is free (D-ORG15).
- Refusals to try, each answered with a line: a second Team (or Command)
  while you lead one, a name another organization of that type already has
  (compared without case), an empty name or one over 60 characters, and
  characters outside letters, digits, spaces, `'`, `-` and `.`. Three refused
  names spend the offer; the registrar then asks you to wait until the
  5-minute offer expires.
- A click from further than 5 units says you are too far away. The offer ends
  if you leave the room's space before naming.
- The click routes to the registrar because the template carries
  `INT_Organization` (64) and exactly one registrar interaction set in
  `static_interaction_sets`, 7447 for a Team or 7448 for a Command (the 2009
  server's `INTERACTION_OrganizationRegister*` ids). Nothing else on the
  template answers a click first.
- To found one without walking here, a GM types
  `.org_create <team|command> <name>` ([commands.md](../commands.md#dev-console--commands)).
- The registrars are faction 1 and cannot die.

## What the hub cannot test, and why

| System | Why there is no hub NPC |
|---|---|
| Mail (partly) | The Gate Mail Clerk covers receiving a mail and taking its cash and item. Sending mail, postage, COD, return and expiry need a sender, not an NPC: send from the mail window to a second character, or as a GM use `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>]`, `.mailbox [name]` and `.mail_expire <mailId>`, which expires a mail at once and says whether it was returned, deleted or quarantined ([commands.md](../commands.md#command-families)). The two-player checks are SS-UAT steps 1-6 ([work-packets.md](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release)). |
| Chat | Tells and ignore need a second player; an NPC does not chat. Alone you can still check the refusals: a tell to your own name ("You cannot send a tell to yourself.") or to an offline name, and the flood limit (paste ten lines into say). As a GM, `.mute <name> <minutes>` and `.unmute <name>` mute a player, and `.announce` (or `/gmshout`) broadcasts ([commands.md](../commands.md#command-families)). The two-player checks are SS-UAT steps 7-10. |
| Duels | A duel needs a second player. Alone, run `sparbot`, a second account that accepts every challenge and forfeits after a set time ([wireclient.md](../architecture/wireclient.md#sparbot-a-duel-partner-for-solo-testing)). As a GM, `.duel_status [name]` shows a duel's stage and `.duel_end <name>` ends it. The duel checks are SS-UAT steps 11-13. |
| Bank (partly) | The Banker (template 370) opens the personal vault, and the Team and Command Bankers (371 and 372) open the vaults of a character's Team and Command. A new character belongs to neither, so found one first (see [Team and Command Bankers](#team-and-command-bankers-templates-371-and-372)); withdraw permissions and the fan-out need a second character. There is no Squad vault Banker. Org cash (BV-08) and the Team vault purchase (BV-09) are separate packets ([docs/analysis/bank-vault/](../analysis/bank-vault/README.md), [gap-analysis.md §23](../gap-analysis.md)). |
| Guilds / organizations (partly) | The registrars (templates 330 and 331) found a Team or a Command. Inviting, ranks, texts and organization chat need a second player and later organizations packets ([docs/analysis/organizations/](../analysis/organizations/README.md), [gap-analysis.md §23](../gap-analysis.md)). |
| Black market | Known missing on `main` ([gap-analysis.md §25](../gap-analysis.md)). |
| Crafting verbs | The [crafting corner](#crafting-corner) gives stations and supplies. Whether each verb works depends on the crafting campaign's progress ([gap-analysis.md §19](../gap-analysis.md)). |
| Pets (partly) | The pet trainer (template 360) sells the summon abilities, but a pet needs a tester, not a hub NPC: summon with `.pet summon 2826` or the ability (`.giveability 1643`, `1644` or `1645` for the Jaffa, Lo'taur or Prime), then walk, fight and change stance. Every pets-campaign packet has merged; the owner's UAT checklist is in [the pets session resume](../analysis/pets/handoffs/session-resume.md) ([docs/analysis/pets/](../analysis/pets/README.md), [gap-analysis.md §28](../gap-analysis.md)). |
| Player-to-player trade | Needs two players. An NPC cannot be a trade partner ([gap-analysis.md §22](../gap-analysis.md)). |

## Where it lives

| What | Where |
|---|---|
| Templates 300-304, 330, 331, 360, 370-372, 390 | `db/resources/Entities/Seed/entity_templates.sql` |
| Spawns 400-404, 430, 431, 450, 470-472, 490 | `db/resources/Worlds/Seed/spawnlist.sql` |
| Trainer list 350 | `db/resources/Abilities/Seed/trainer_ability_lists.sql`, `trainer_abilities.sql` |
| Chains 7001-7005, 7010-7011 | `db/resources/Content/Seed/debug_hub_chains.sql` |
| Dialogs 60100-60104, screens 200000-200005, buttons 200000-200001 | `db/resources/Dialogs/Seed/` and `QUARANTINED_DIALOG_OVERRIDES` (not served) |
| The clerk's mail and cooldown | `mail/content.rs` in `crates/base-methods`; table `db/sgw/Players/Tables/sgw_player_content_cooldown.sql` |
| Loot table 3 (loot rows 14-23) | `db/resources/Loot/Seed/` |
| Ability set 6 | `db/resources/Abilities/Seed/ability_sets.sql`, `ability_set_abilities.sql` |
| Vendor and Banker derivation | `static_interaction_for_flags` in `crates/cell-world/src/cell/space_manager/spawn.rs` |
| Crafting corner: templates 310-314, spawns 410-414, buy list 310 (rows 3101-3124) | `entity_templates.sql`, `spawnlist.sql`, `db/resources/Items/Seed/item_lists.sql` and `item_list_items.sql` |
| Registrar recognition | `registrar_type` in `crates/cell-interactions/src/cell/interactions/org_registrar.rs` |

Every seed row is commented `NEW CONTENT (debug hub)`, except the pet
trainer's, which are commented `Pets campaign, PT-07`, the Banker's,
commented `Bank and Vault campaign, BV-04`, the Team and Command Bankers',
commented `BV-10a`, and the mail clerk's,
commented `Social-systems campaign, SS-U3`, and the registrars',
commented `Organizations campaign, ORG-05`. The `trainer_abilities`
rows carry no comment: that file is regenerated by
`tools/ability_trees/generate_seed.py`, which keeps other lists' rows but not
comments. The crafting corner's rows are commented `NEW CONTENT (debug
hub, crafting)`.

## Tests

| Guard | What it pins |
|---|---|
| `cell-catalog` `spawner/tests/live_db_debug_hub.rs` | Role columns of each template; spawns (the mail clerk's 490 included) inside Region1, on the floor, at least 5 units from the respawner and 2.5 from every other spawn in the room; the crate's ability set is exactly `[710]` and deals no damage; trainer list 1; vendor lists and loot table 3 name real items, the naquadah row and at least one item row at probability 1 and every row able to drop; the guides and the blueprint item drop once each, quantity 1, below certain, and each is a `{17,15}` item with a crafting effect; dialog screens and buttons, and neither dialog is a monologue |
| `cell-methods` `interaction/debug_hub_dispatch_tests.rs` | Each NPC, spawned from its real row, answers a right-click with its own interaction (the mail clerk's dialog included); the crate reroutes to an attack while alive and, when dead, carries every certain table-3 row and nothing outside the table, and shows its loot; respec passes at the hub trainer and is refused at the vendor; the pet trainer opens list 350 for a Goa'uld and an empty list for anyone else; the Banker opens the personal vault, pinned to itself; each registrar asks the base about its own type; each org Banker asks the base for its own vault type |
| `cell-catalog` `spawner/tests/live_db_debug_registrars.rs` | Templates 330 and 331 are a Team and a Command registrar and nothing else: exactly `INT_Organization` and their own registrar set, a shipped name, no trainer or vendor list, not faction 10; they are the only registrar templates; spawns 430 and 431 inside Region1, on the floor, clear of the respawner and of every other NPC in the room |
| `cell-catalog` `spawner/tests/live_db_debug_org_bankers.rs` | Templates 371 and 372 are a Team and a Command Banker and nothing else: exactly `INT_Banker`, `vault_scope` `team` and `command`, a shipped name, no trainer list or vendor lists, not faction 10; spawns 471 and 472 inside Region1, on the floor, clear of the respawner and of every other NPC in the room |
| `services` `bank_org_round_trip_tests.rs` | Spawned from their real rows and clicked through the cell's dispatcher, the base's cell dispatch against the live database, and the cell's grant: a member gets exactly 107 (Team) or 108 (Command) and a session naming the organization; a character in no organization gets `org_vault_open_rejected reason=not_in_org` and its line; a Team-only character is refused by the Command Banker |
| `cell-catalog` `spawner/tests/live_db_debug_banker.rs` | Template 370 is a personal Banker and nothing else: exactly `INT_Banker`, `vault_scope = 'personal'`, a shipped name, no trainer list or vendor lists, not faction 10; spawn 470 inside Region1, on the floor, clear of the respawner and of every other NPC in the room |
| `cell-catalog` `spawner/tests/live_db_pet_trainer.rs` | Template 360's role columns and name; spawn 450 inside Region1, on the floor, clear of the respawner and the other hub NPCs; list 350 keyed to the Goa'uld only, with exactly the six pet nodes, each a Goa'uld tree node |
| `cell-catalog` `spawner/tests/live_db_mail_clerk.rs` | Template 390's role columns and name; dialog 60104 is one clerk screen with one Generic 1 button, and not a monologue |
| `cell-content` `chain_replay_tests/debug_hub.rs` | Chains 7001-7005 resolve and execute: the `onDialogDisplay` speakers, the `StartMinigame` message, and both chat lines |
| `cell-content` `chain_replay_tests/debug_hub_mail_clerk.rs` | Chain 7010 opens 60104 as the clerk; chain 7011 sends the base exactly one `ContentSystemMail` with the seeded contents and cooldown |
| `base-methods` `mail/tests/content_live.rs` | One mail per press, refused with the wait inside 10 minutes; the window outlives the mail and ends on time; the claim rolls back with a refused mail; two simultaneous presses write one mail |
| `cell-world` `tests/npc_spawn.rs` | Any `INT_Vendor*` bit derives `Vendor`; no other bit derives anything |
| `resources` `dialog_overrides/override_seed_agreement_debug_hub.rs` | The quarantined overrides of 60100, 60101 and 60104 and the dialog seed agree screen for screen and button for button; `dialog_overrides/mod.rs` `quarantined_dialogs_are_not_served` fails if one is served again without lifting the quarantine |
| `content-engine` `interact_tag_linter`, `dialog_button_linter` | Chains 7001, 7004 and 7010 are allowlisted (template-default bits); the hub dialogs obey the button hard rules |
| `cell-catalog` `spawner/tests/live_db_crafting_hub.rs` | Stations carry all four craft bits and no interaction bit, with the client's station monikers; the vendor sells only list 310; the crafting spawns stand inside Region1, on the floor, clear of the respawner and of every other spawn in the room; list 310 is exactly the supplies, each a real item at 1 naquadah and no item cost, and covers the UAT recipes |
| `cell-methods` `interaction/crafting_hub_station_tests.rs` | Spawned from their real rows, the stations are reported for every verb to a player at the supplies vendor, and none reaches the respawn spot |
| `base-methods` `vendor/purchase/crafting_supplies_tests.rs` | Bought supplies land in the crafting bag: the guide and the Blueprint item are used from there, and the cores are consumed by a crafting transaction |
