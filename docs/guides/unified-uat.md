---
title: Unified In-Game UAT Guide
type: how-to
audience: in-game testers (the owner and playtesters) working through the restored systems on the colo; no programming needed
last_updated: 2026-09-27
companion_docs:
  - organizations-uat.md
  - ../content/debug-hub.md
  - ../commands.md
  - ../analysis/zone-restoration-operator-guide.md
  - ../analysis/castle-cellblock-rebuild/uat-guide.md
  - ../operations/npc-ai-telemetry-runbook.md
  - ../operations/colo-deploy.md
  - ../architecture/observability.md
---

# Unified In-Game UAT Guide

> Type: how-to. Audience: an in-game tester who is not a programmer.
> Updated: 2026-09-27. Companions: [debug hub](../content/debug-hub.md), [commands](../commands.md), [organizations UAT](organizations-uat.md), [Cellblock UAT guide](../analysis/castle-cellblock-rebuild/uat-guide.md), [zone operator guide](../analysis/zone-restoration-operator-guide.md), [telemetry runbook](../operations/npc-ai-telemetry-runbook.md).

## Purpose and how to use it

This guide gathers every restored system's in-game acceptance test (UAT) into one place, so you can work through all of them from one document. It was compiled on **2026-09-27** from each campaign's own checklist.

**Each campaign's ledger stays canonical.** When a campaign changes a step, it changes its own checklist first, and this guide follows. If a step here disagrees with the ledger, the ledger wins; report the mismatch.

| System | Canonical checklist |
|---|---|
| [Pets](#pets) | [pets session resume, UAT checklist](../analysis/pets/handoffs/session-resume.md#uat-checklist-owner-colo-after-the-pt-13-release) |
| [Organizations](#organizations) | [organizations-uat.md](organizations-uat.md) |
| [Crafting](#crafting) | [crafting session resume, CR-14 checklist](../analysis/crafting/handoffs/session-resume.md#cr-14-owner-uat-checklist) |
| [Bank and vault](#bank-and-vault) | [bank session resume, UAT checklist](../analysis/bank-vault/handoffs/session-resume.md#uat-checklist) |
| [Mail, chat and duels](#mail-chat-and-duels) | [SS-UAT in work-packets.md](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release) |
| [Black market](#black-market) | [black-market uat.md](../analysis/black-market/uat.md) |
| [NPC AI](#npc-ai) | [NPC AI session resume, owner checklist](../analysis/npc-ai-restoration/handoffs/session-resume.md#owner-uat-checklist-colo-after-the-next-release) |
| [Ability trees](#ability-trees) | [AT-06 in work-packets.md](../analysis/ability-trees/work-packets.md#at-06-owner-uat-colo-after-the-release) |
| [Dialog UI](#dialog-ui) | [DU-UAT in work-packets.md](../analysis/dialog-ui-redesign/work-packets.md#du-uat) |
| [Castle Cellblock tutorial](#castle-cellblock-tutorial) | [Cellblock UAT guide](../analysis/castle-cellblock-rebuild/uat-guide.md) |
| [Castle (world 8)](#castle-world-8) | [zone operator guide, runbook B](../analysis/zone-restoration-operator-guide.md#b-castle-world-8-ring-platform-build-current-main-all-of-ca00-ca10-merged) |
| [Harset](#harset) | [zone operator guide, runbook C](../analysis/zone-restoration-operator-guide.md#c-harset-worlds-57-68-69-70) |
| [Historical cellblocks](#historical-cellblocks) | [historical-cellblocks README](../analysis/historical-cellblocks/README.md#what-the-uat-must-settle) |
| [Ring transport](#ring-transport) | [ring-transport README, Phase 1](../analysis/ring-transport-cellblock-castle/README.md#phase-1-status) |
| [GM console command parity](#gm-console-command-parity) | [legacy command parity README](../analysis/legacy-command-parity/README.md#validation-and-uat-gates) |

**How to work a section.** Read its prerequisites, then do each numbered step in order. Every step keeps the campaign's own step id (`U1`, `T25`, `B7`, ...), so you can report a result against it. The `Notes / known issues` column tells you when a failure is already known and should not be filed again.

**Status legend.** Each section, and some steps, carry one of these:

| Status | Meaning |
|---|---|
| **Ready** | All the code is merged. Every step should pass; a failure is a new finding. |
| **Partly works** | Most steps should pass. Steps marked in the notes are expected to fail or cannot be reached yet. |
| **Known broken** | Expected to fail today, with an issue or PR tracking it. Note what you see, but do not file it again. |
| **Needs client patch** | Needs files installed in your game client that do not ship to testers. Owner-only unless you have been given the patch. |
| **Not ready for UAT** | Not released to testers yet. The section lists what is coming so you can plan; do not run it. |

A section only works on the colo once its code is in the build the colo runs. Every row in SigNoz carries `service.version`, the git SHA of that build. If a whole section fails at step 1, check with the owner that its release has deployed.

## Current known issues

Read these before you start. None of them needs a new report.

| # | Issue | What to do | Evidence |
|---|---|---|---|
| K1 | **Map-load crash investigation (open, 2026-09-27).** Some testers' clients crash or hang while loading `Castle_CellBlock`: the load stops after the map starts and the game never finishes entering the world. | Close the game. Delete `Cache.en-US\CookedDataDialogs.pak` under `Documents\My Games\Firesky\SGWGame\`. Start the game and retry; the client rebuilds the file at the next login. Do this before you log in if your client ever received the old debug-hub dialogs, even if it has not crashed yet: the server cannot remove them from your cache. If it still crashes, type nothing, note the time, and tell the owner. | [cooked-dialog-override-crash.md](../reverse-engineering/findings/cooked-dialog-override-crash.md), PR #943 |
| K2 | **Two debug-hub NPCs show no dialog.** The Dialog NPC (**Airman Lance**) and the Gate Mail Clerk (**Sgt. Harriman**) are quarantined: their dialogs (60100, 60101, 60104) are not sent to clients. A right-click opens nothing. | Skip them. For the mail test, use the GM `.mail` command instead of the clerk. | [debug-hub.md](../content/debug-hub.md#dialog-npc-template-302-airman-lance), PR #943 |
| K3 | **The Banker's "Expand vault" offer is quarantined** for the same reason (dialog 60110). Players cannot buy a vault expansion at the Banker; the button is pending #967 (the #943 quarantine). | GMs use `.bankexpand` (bank steps 11-12), and `.orgvaultexpand` for a Team vault (bank steps 24-25). | [bank session resume](../analysis/bank-vault/handoffs/session-resume.md#lifting-the-943-quarantine) |
| K4 | **The colo database is wiped on every deploy.** Characters, missions and inventory reset each time a new build rolls out, and on any container restart. | Expect to re-create characters after a deploy. Record your character name with every result. | [colo-deploy.md](../operations/colo-deploy.md#what-you-dont-get-yet) |
| K5 | **An NPC or corpse can stay invisible until you relog** (Cellblock NID guard corpse, possibly Marsh after the ring hop). | Type `.bug invisible <what>` next to where it should be, then relog and say whether it appeared. | Issues #582, #838; [Cellblock guide, Known risks](../analysis/castle-cellblock-rebuild/uat-guide.md#known-risks) |
| K6 | **Empty Cellblock during the first-login intro movie.** On a brand-new character the room fills only when the movie ends (about 16 s) or when you press Esc. This is intended, not a bug. | Nothing. | [first-login cinematic hold](../architecture/first-login-cinematic-aoi-hold.md) |
| K7 | **Several tutorial effects are inert.** Prison Boot, Stasis Sickness and its cure send nothing to the client, so no icon or movement lock appears. | Record what you see; "I could walk" is not a failure. | [Cellblock guide, Known limitations](../analysis/castle-cellblock-rebuild/uat-guide.md#known-limitations--not-validated) |
| K8 | **An objective checkbox may not tick** when a step completes it implicitly (Cellblock step 2144's second objective, mission 688's objective 2734). The step still advances. | Record it; not a new bug. | Issue #656 |
| K9 | **System messages do not render** (for example message 5040 on entering Cellblock Region2). | Nothing. | Issue #268 |
| K10 | **Col. Marsh's third bark does not fire** ("Flank their position while I draw their fire!" at Hallway05). | Record it as the known failure. | Open PR #826 |
| K11 | **Marsh is left behind if you relog** after the ring hop and before mission 686 completes. | Record it; documented gap. | [Cellblock guide T28](../analysis/castle-cellblock-rebuild/uat-guide.md#t28--marsh-rides-the-rings-and-follows-you-topside-gc1b-1-gc1b-2) |
| K12 | **The Lo'taur pet never attacks or heals**, and several pet-bar abilities (1652, 1654, 1653, 3326-3329) do nothing except say "Your pet can't use that ability yet." | Record it; known gap. | [pets README, Campaign outcome](../analysis/pets/README.md#campaign-outcome) |
| K13 | **Stale action-bar buttons after an ability respec.** The server keeps no hotbar, so a refunded ability's button stays; pressing it shows an error. | Expected. | [ability-trees session resume](../analysis/ability-trees/handoffs/session-resume.md#open-owner-decisions) |
| K14 | **Two client-only mail and duel cosmetics.** The unit-frame PvP indicator does not refresh live during a duel, and after a refused mail send the Send button stays grey until you press New or Reply, which clears the typed text. Both need a client patch. | Expected. | [social session resume, Q-k](../analysis/social-systems/handoffs/session-resume.md#owner-questions) |
| K15 | **Mixing servers can empty your world list.** A client that logged in to a server with the historical cellblocks, then to one without them, can lose its whole cached world table. | Ask the owner for the repair before you continue. | [historical-cellblocks README](../analysis/historical-cellblocks/README.md#mixing-servers-during-the-uat), issue #840 |
| K16 | **The black market window is not released to testers.** Its client patch does not ship yet. | Run only the steps marked `server` in that section (the auctioneer's chat line and the GM `.bm_*` tools). | [Black market](#black-market) |
| K17 | **All three Bankers are named "Storage Officer"**, and after a Team vault expansion, a Team vault window already open on another member keeps its old size until that member reopens it. | Tell the Bankers apart by their clothes ([Bank and vault](#bank-and-vault)). Reopen the window. | [bank session resume, Known gaps](../analysis/bank-vault/handoffs/session-resume.md#known-gaps-carried-forward) |
| K18 | **Crafting gaps known at release.** Using a Blueprint item or a Racial Paradigm Guide prints no line (the item goes and the window changes). Buying several non-stacking components in one purchase shows one stack. A queued crafting job still finishes if you walk away from the station or give the tool away. `/showracialparadigmlevels` and the client's own `/respeccraft` do nothing useful. | Record them; not new bugs. Use `.respeccraft`. | [crafting session resume, known gaps](../analysis/crafting/handoffs/session-resume.md#known-gaps-carried-forward) |

## Common setup

### The colo and your account

- You test on the colo server, which rebuilds its database from the repository seed on every deploy (K4). New characters wake up in the **stasis room** of `Castle_CellBlock` (world 12).
- You need a **GM account** (access level 2 or higher) for most sections. Ask the owner for one. A few steps need a second account (two clients, or a second player); each section says so.
- Several sections need a second character logged in at the same time. A second game window on a second account works.

### The GM consoles

You have two consoles, both typed into the chat box:

- **`/gm...` commands** are the game's own GM commands (for example `/gmgotolocation`, `/gmgivexp`, `/gmgiveitem`, `/gmmissionassign`). They take numbers, not names. The full list, with what works, is [commands.md](../commands.md#game-master-commands).
- **`.` commands** are Cimmeria's dev console. The server catches any line that starts with a registered `.`-command and answers you in chat; nobody else sees it. A non-GM who types one gets "is a GM command" back. Type `.help` for the live list, or `.help <word>` for one command. Full reference: [commands.md, Dev console](../commands.md#dev-console--commands).

The `.`-commands the campaigns rely on:

| Command | What it does | Used by |
|---|---|---|
| `.bug <note>` | Bookmarks this moment in SigNoz: you, your target and every entity within 60 units, with your note | Every section |
| `.giveability <abilityId>` | Gives the selected player (or you) an ability and saves it to the character | Pets, ability trees |
| `.pet summon <id>` / `dismiss` / `stance <0-2>` / `info` / `list` | Pet tools; nothing is saved | Pets |
| `.org_create`, `.org_join`, `.org_rank`, `.org_set_perms`, `.org_info`, `.org_list`, `.org_disband` | Team and Command tools | Organizations |
| `.squad_invite`, `.squad_join`, `.squad_info` | Squad tools | Organizations |
| `.bank`, `.bankdump [name]`, `.bankexpand` | Open your vault anywhere, list a vault, buy one +10 expansion | Bank |
| `.orgvaultexpand [team\|command] [from_slots]` | Quote, then buy, one +10 step of your Team's vault from its treasury; leader only | Bank |
| `.mail`, `.mailbox`, `.mail_expire <mailId>` | Send test mail, inspect a mailbox, expire a mail now | Mail |
| `.mute <name> <minutes>`, `.unmute <name>`, `.announce <text>` | Chat moderation and broadcast | Chat |
| `.duel_status [name]`, `.duel_end <name>` | Inspect or end a duel | Duels |
| `.aggro on` / `off` | Stop or restore idle NPCs noticing you (default on) | NPC AI |
| `.allcraft`, `.craftkit <blueprint> [count]`, `.learnblueprint <id>` | Crafting grants to the **selected** player (select yourself first) | Crafting |
| `.respeccraft` | Open a free crafting respec; any player may use it | Crafting |
| `.gotolocation <world> <x> <y> <z>`, `.gotoxyz`, `.goto <name>`, `.summon <name>` | Travel | Many |

### The stasis-room debug hub

The stasis room holds a row of NPCs, each for testing one system, a few seconds' walk from where a new character appears. Every player sees them. Placement has not been checked in a client: if one stands in a wall or floats, that is a finding. Full reference: [debug-hub.md](../content/debug-hub.md).

| Name shown | What it is for | Status |
|---|---|---|
| Basic Equipment Quartermaster | Vendor: buy, sell, buyback, repair, recharge | Ready |
| Archetype Skills Trainer | Ability trainer with every tree node; respec | Ready |
| Airman Lance | Dialog round trip (paging, a button, a button-less close) | Known broken (K2) |
| Terminal | Livewire minigame round trip; win prints "Livewire round trip complete" | Ready |
| Crate | A mob to kill for loot; always drops slappacks, a Processor, a Cell and naquadah; respawns 30 s after death | Ready |
| Goa'uld Advanced Skills | Pet trainer (Goa'uld pet summons) | Ready |
| Organization Registrar (SGC uniform) | Found a Team | Ready |
| Organization Registrar (armour) | Found a Command | Ready |
| Storage Officer | Banker: opens your personal vault | Ready |
| Storage Officer (Cellblock guard uniform) | Team Banker: opens your Team's vault; needs a Team | Ready (bank steps 15-25) |
| Storage Officer (plain crew clothes) | Command Banker: opens your Command's vault; needs a Command | Ready (bank steps 15, 21) |
| Sgt. Harriman | Gate Mail Clerk: sends you a test mail | Known broken (K2) |
| Machra | Black Market auctioneer, by the exit doorway: opens the auction house | Needs client patch for the window; the chat line works on any client |
| Common Materials Components | Crafting supplies vendor (1 naquadah each) | Ready |
| BioMedical / Electronics / Power Systems / Materials Crafting Station | Crafting stations; stand within 5 units and open J | Ready |

The crafting group stands along the wall on the opposite side from the main row.

### Reporting a failure

1. **At the moment it happens**, type `.bug <what you see>`, for example `.bug pet did not follow me through the door`. Say what you were looking at: the note is the only record of what you saw, and the bookmark captures the server's side.
2. Keep playing only if the section allows it; otherwise stop at that step.
3. Record the result with the [template below](#recording-results): step id, pass or fail, the time, your character, and the `.bug` note.

A good report says: the step id, what you did, what you saw, what you expected, the time (with time zone), your character name and archetype, and whether a relog changed it.

### Checking SigNoz

SigNoz holds the server's logs; every `.bug` lands there. Access is through the Cloudflare Access login ([signoz-remote-access.md](../operations/signoz-remote-access.md)). You do not need it to test; the coordinator reads it afterwards. If you want to look:

1. Open **Logs Explorer**, set the time range to cover your session, and filter `service.name = 'cimmeria-server'`.
2. Find your bookmark with `scope_name = 'playtest.bookmark'`, or the **Playtest: bookmarks (.bug notes)** saved view. Copy its `bookmark_id`.
3. `scope_name = 'playtest.bookmark.entity' AND bookmark_id = <id>` lists every entity near you at that moment.
4. Each section below gives its own queries. Add `AND player_id = <your character id>` to narrow them to you.
5. **Colo rows may still say `cimmeria.deploy_env = 'dev'`.** If a colo filter returns nothing, filter on `service.version = '<build sha>'` instead.

The full method is the [telemetry runbook](../operations/npc-ai-telemetry-runbook.md#start-from-a-bug-bookmark). Every `.`-command's reply is also logged: `scope_name = 'console.feedback'`.

## Suggested test order

Start from a **fresh character**, so the tutorial and every "first time" check run from the beginning. Groups are arranged so one play session covers several systems.

| Session | Character | Covers | Roughly |
|---|---|---|---|
| 1. Arrival | New Tau'ri (Soldier, Commando, Scientist or Archaeologist) | Character creation, the intro movie (K6), Cellblock M1 (T25, T01-T07, T26), then a detour to the debug hub | 45 min |
| 2. Hub pass | Same character, GM | Ability trees (AT-06 steps 1-7) at the Archetype Skills Trainer, bank steps 1-10 and 13-14 at the Storage Officer, mail steps 1-6 with `.mail`, pets U1-U13 and U17-U20 with `.giveability` | 60-90 min |
| 3. Tutorial run | Same character | Cellblock M2-M5 (T08-T31), dialog UI rows, NPC AI steps 1-9 in the same rooms | 60 min |
| 4. Jaffa run | New Jaffa (Shol'va) | Cellblock archetype rows (T05/T06, T11/T12, T18/T19 Jaffa), Castle M1 as Jaffa | 60 min |
| 5. Castle | Tau'ri from session 3 | Castle B1-B20, NPC AI steps 10-15 | 60 min |
| 6. Harset | Same, then an ordinary non-GM account | Harset C1, C4, C5, C6 | 45 min |
| 7. Two players | Two accounts (a third for the duel spectator) | Organizations steps 1-12, bank steps 15-25 (org vaults, treasury, Team expansion), chat and duel steps 7-13, pets U11, crafting step 17, NPC AI one-way visibility, Castle B20 | 120 min |
| 8. Goa'uld | New Goa'uld | Pets U2 at the trainer, U14-U16 | 20 min |
| 9. Owner only | Patched client | Historical cellblocks, ring transport Phase 1 | as needed |
| 10. Crafting | New character, GM | Crafting steps 1-16 and 18-21 at the crafting corner; step 17 fits session 7 | 75 min |

Relog at every step boundary that a section asks for. Most defects these campaigns found were "correct until you relog".

## Pets

Summoned companions (Straegis, Jaffa, Prime, Lo'taur) that follow you, fight, obey stance and ability orders from the pet bar, and give you the kill credit.

**Status:** Ready. Every planned packet merged; nothing has been seen in a client yet. Known broken: K12.

**Prerequisites:** a GM character. Grant the summons with `.giveability 2826` (Straegis), `1643` (Jaffa), `1644` (Lo'taur), `1645` (Prime); any archetype can then cast them. Buying the pet nodes at the trainer needs a Goa'uld, and 2826 needs level 50 there. U11 needs a second player; U13 needs a non-GM character. Location: anywhere; the trainer is in the debug hub.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| U1 | `.pet summon 350` or `.pet summon 2826` | A Straegis Fighter appears beside you; chat names the pet id (and "via ability 2826" for the ability id). The pet window/bar shows portrait, name, ability row and three stance buttons, Defensive highlighted. | |
| U2 | Right-click **Goa'uld Advanced Skills** in the debug hub (last NPC on the back-wall line, right of the Crate) | It offers 2826, 1643, 1644, 1645, 1652 and 1654. A level-1 Goa'uld with a training point can buy 1643; 2826 stays greyed below level 50. A non-Goa'uld sees an empty trainer. | Run once as Goa'uld (session 8) and once as another archetype |
| U2b | `.giveability 2826` with nothing selected | Chat says it was saved; the ability appears at once. After a relog it is still known. A second `.giveability 2826` says "already knows". | |
| U3 | Cast Summon Straegis | 6 s warmup with the Goa'uld summon effects, then the pet appears. Casting again replaces it (still one pet). Moving during the warmup interrupts it, and no pet appears. | |
| U4 | Walk, run and turn; then walk 50 units away fast (or `.goto` somewhere in the same zone) | The pet follows at 2-5 units, and teleports to you within about 5 s when you get far away. | |
| U5 | Attack a hostile guard, pet in Defensive | The pet engages what you hit and what hits you. Afterwards it comes back to you, not to where it was summoned. | |
| U6 | Press each stance button, in the pet window **and** on the small pet bar; also `.pet stance 0`, `1`, `2` | Passive: ignores fights even when hit. Aggressive: engages hostiles near it. The highlighted button always matches, from all three places. | |
| U7 | Click a pet ability on the pet bar | The pet uses it on your target. Out of range or on cooldown gives visible feedback. | |
| U8 | Let the pet kill a mob | You get the XP, and a KillCount objective for that mob advances. | |
| U9 | Die with the pet out | The pet disappears. After respawn, summon again. | |
| U10 | Log out and back in; change zone (gate or ring) | The pet is gone after each. `.pet list` from a second character in the old zone shows no orphan pet. | Pets are per session by design |
| U11 | A second player looks at your pet | They see a Straegis with your name ("X's Pet" if the client shows it). They cannot command it and get no pet bar. | Two players |
| U12 | `.pet info` (select another player's pet to inspect theirs) | Owner name and id, stance and allowed stances, AI state, abilities `[221, 1156]`, what is toggled off, distance to owner, last teleport ("never" until it has teleported). | |
| U13 | On a non-GM character, type `.pet summon 2826` | One chat line: ".pet is a GM command; you do not have GM rights". Nobody nearby sees the text; no pet appears. | Non-GM account |
| U14 | `.giveability 1643`, `1644`, `1645`, then cast each summon in turn | Each replaces the last pet. Jaffa (Praxis Jaffa armour), Prime (Praxis Jaffa Lieutenant look) and Lo'taur (Goa'uld in servant dress and slave headwrap) each render and animate at your level with three stance buttons. | The Lo'taur look has never been rendered before |
| U15 | Fight with the Lo'taur out, in Defensive and in Aggressive | The Lo'taur follows but never attacks and never heals. The Jaffa and Prime fight with their staffs. | Known broken for the Lo'taur (K12) |
| U16 | With the Prime out, click 1654 Focus Degeneration on the pet bar (or any Lo'taur ability with the Lo'taur out) | The pet does nothing; chat says "Your pet can't use that ability yet." | Refusal is the expected result |
| U17 | `.giveability 2824` (Holy Warrior), press it twice with a pet out | First press: "Holy Warrior is on."; second: "Holy Warrior is off." | |
| U18 | `.giveability 2839` (To The Death), cast it with a pet out | After the 2 s warmup: "Your pet fights to the death: it dies in 60 seconds." After 60 s the pet dies; its corpse goes 10 s later. No XP or kill credit. Casting again meanwhile: "Your pet is already fighting to the death." | |
| U19 | `.giveability 2852` (Heed Our Calling), then cast Summon Straegis | The very next summon is instant, with no 6 s warmup. | |
| U20 | `.pet dismiss`, then press an owner-pet ability such as Holy Warrior | "You have no pet to use that on."; no cooldown is charged. | |

**SigNoz** (base `service.name = 'cimmeria-server'`):

| Question | Filter |
|---|---|
| Everything pet-related a player did | `scope_name LIKE 'pets.%' AND account_id = <N>` |
| One pet's life | `scope_name LIKE 'pets.%' AND pet_id = <id>` |
| Why a summon or command was refused | `scope_name IN ('pets.lifecycle','pets.command') AND reason EXISTS` |
| Failed summons | `scope_name = 'pets.lifecycle' AND event IN ('summon_failed','summon_refused')` |
| Why the pet despawned | `scope_name = 'pets.lifecycle' AND event = 'despawned'` |
| Leash teleports and stance changes | `scope_name = 'pets.ai' AND pet_id = <id>` |
| Kill XP and credit (U8) | `scope_name = 'pets.credit' AND account_id = <N>` |
| U16 refusals | `scope_name = 'pets.command' AND reason = 'ability_not_implemented'` |
| Owner buffs (U17-U20) | `scope_name = 'pets.buff' AND account_id = <N>` |
| Was a `.giveability` saved (U2b) | `body CONTAINS 'GmGrantAbility' AND subject_player_id = <player>` |
| A non-GM tried a GM command (U13) | `reason = 'not_gm' AND account_id = <N>` |

**Things only a human can check:** whether the Straegis Fighter body renders and animates (the server has never spawned it); whether the Jaffa, Prime and Lo'taur looks render as pets; the trainer's placement and name plate; whether the pet bar fills; the pet's nameplate text; the summon VFX; whether the small pet bar's stance buttons map to the right stances.

Source: [pets session resume](../analysis/pets/handoffs/session-resume.md#uat-checklist-owner-colo-after-the-pt-13-release); background in [pet-system.md](../gameplay/pet-system.md).

## Organizations

Squads (the temporary party), Teams (small persistent groups) and Commands (the persistent guild, with ranks, permissions, MOTD, notes and officer chat).

**Status:** Ready: server-side feature-complete, not yet client-verified. Released with the ORG-11 close-out (#955). Vault and treasury tests belong to [Bank and vault](#bank-and-vault).

**Prerequisites:** two accounts, **A** (GM) and **B** (any). For the one-client fallback, a third character **S** parked online. Both in the stasis room; the two registrars stand in the debug hub. Before you start, note each character's player id: `.org_info <name>` prints it.

Each step has a two-client script and a one-client GM fallback. The full script, fallback and the log rows for each step are in [organizations-uat.md](organizations-uat.md); this table is the short form.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| 1 | Squad invite: A types `/squadinvite B`; B accepts. (Solo: `.squad_join S`) | Both squad frames show the other member; the accept closes the prompt. A refused invite (B already in a squad, too many invites) gives A a feedback line. | |
| 2 | Squad chat and loot mode: both type on the squad channel; B (not leader) changes the loot mode, then A does | **Each squad line shows once in each chat window, not twice.** B's change snaps back with "only the leader can change it"; A's reaches both frames. | Needs a real client: a doubled line on the speaker's screen is a client echo; report either kind with `.bug` |
| 3 | Squad across worlds: B gates to another world; both chat before and after B arrives | The squad survives; B's frame still shows A; chat reaches B after arrival. | A line sent while B is mid-transit may miss B (known gap; `.bug` it, do not fail) |
| 4 | Squad leave: A leaves, then B leaves | B becomes leader; after B leaves, no squad on either frame. | |
| 5 | Right-click the **Command** registrar (Organization Registrar in armour), name the Command; then try the same name again | The naming window opens; after the name, the Command window opens with A as Leader and a one-member roster. The duplicate name is refused with a visible line. | Needs a real client: does right-click open the naming window, and does it stay open or close on a refused name? Note which. Fallback `.org_create command <name>` only if the registrar fails |
| 6 | A invites B from the Command window; B accepts. (Solo: `.org_join <orgId> S`) | B gets an invite naming the Command; after accepting, B's window opens with full state and A's roster shows B online. | |
| 7 | A promotes B to Officer (rank 6); B tries to promote A; B invites and kicks a third character | Both rosters show B's new rank; B's promote of A is refused with a line; the kicked character's window closes. | Solo fallback cannot test the Officer's own invite/kick; mark it untested |
| 8 | A sets the MOTD, a note, an officer note on B, and renames a rank; then removes `OfficerNotes` (0x40) from B's rank (solo: `.org_set_perms <orgId> 6 <mask>`) | Each text appears for B without a relog; officer notes vanish from B's roster when the bit goes. | |
| 9 | Both chat on the Command channel and the officer channel; remove `OfficerChat` (0x100) from B's rank | Each line shows once; nobody outside sees it; B's next officer line is refused with a line. | |
| 10 | Both log out and in; then A logs out while B watches. (Solo: relog, then `/ReloadOrganizations`) | The Command window returns with name, MOTD, rank names and roster; B sees A go offline and online. `/ReloadOrganizations` gives the same window and a line with the count it re-sent. | Needs a real client: does `/ReloadOrganizations` redraw an already open Command window? |
| 11 | Repeat 5 and 6 at the **Team** registrar (SGC uniform) | A holds a Team and a Command at once; a second Team is refused. A Team uses ranks 2, 3 and 8 and shows 12 permission bits (Command: 14). | |
| 12 | A tries to leave the Command with B in it; B leaves; A leaves | A's first leave is refused (leader cannot leave while others remain); B's leave closes B's window; A's leave ends the Command (`.org_list` no longer shows it). | `.org_disband` is refused while the vault holds anything |

**SigNoz:** start every query with `service.name = 'cimmeria-server' AND scope_name IN ('squad', 'org')`, then add:

| To see | Add |
|---|---|
| Everything one player did | `AND player_id = <id>` |
| Why something was refused | `AND outcome = 'rejected'`, grouped by `event`, `reason` |
| Everything a GM did | `AND event = 'org.gm_action'` |
| Login restore (step 10) | `AND event IN ('org.login_restore', 'org.state_push', 'member_online', 'member_offline')` |
| A member was not told | `AND event IN ('org.send_failed', 'org.broadcast_failed', 'squad.send_failed')` |

Per-step queries: [organizations-uat.md](organizations-uat.md). If an ORG-08 or ORG-09 event name returns nothing, check the `org` row of [observability.md](../architecture/observability.md) for the names as merged.

**Things only a human can check:** each squad, Team, Command and officer chat line shows once (step 2, 9); right-clicking a registrar opens the naming window, and whether it stays open on a refused name (step 5); whether `/commandinvite` and `/teaminvite` reach the invite path, and how the Command window reacts to a kick (steps 6, 7); whether the rank editor's change reaches every open window and an empty note clears (step 8); whether `/ReloadOrganizations` redraws an open window (step 10). The list is the ledger's [Known gaps: needs a real client](../analysis/organizations/README.md#known-gaps-and-follow-ups).

Source: [organizations-uat.md](organizations-uat.md), [ORG-UAT in work-packets.md](../analysis/organizations/work-packets.md#org-uat-owner-two-client-uat-colo) and the [organizations session resume](../analysis/organizations/handoffs/session-resume.md#after-the-owners-uat-reading-signoz).

## Crafting

The Crafting (J) and Applied Science (Ctrl+J) windows: learning disciplines with applied-science points (ASP), craft, research, reverse engineering and alloying, respec, stations and Field Crafting Tools, Blueprint items and Racial Paradigm Guides, and the crafting bag.

**Status:** Ready once the crafting release deploys: all the code is merged (the crafting campaign, CR-01 to CR-17, with respec CR-10) except CR-18 (step 20) and CR-19 (step 21), and none of it has been run in a client. The canonical checklist is the [CR-14 checklist in the crafting session resume](../analysis/crafting/handoffs/session-resume.md#cr-14-owner-uat-checklist); the step numbers below are its step numbers. The provisional CRP-1 to CRP-11 list that stood here is retired.

**Prerequisites:** a GM account and a **new character** in the stasis room. The crafting corner stands along the wall opposite the main hub row: the **Common Materials Components** vendor (1 naquadah per item) and four **Crafting Stations**. Any station allows all four verbs; you are "at" one within 5 units. `.allcraft`, `.craftkit` and `.learnblueprint` act on your **selected target**: select your own character first (click your portrait), and note it if the client will not let you. Step 17 needs a second player. Note the time you log in: your `player_id` is on your `login_sync` row in SigNoz.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| 1 | Log in with the new character; open Ctrl+J | ASP reads 1. The four root disciplines (Biomedical, Electronic, Power Systems, Materials Engineering) are learnable. | Every character starts at Common paradigm 5 |
| 2 | `/gmgiveappliedsciencepoints 5` | "gmGiveAppliedSciencePoints: +5 (total 6)"; Ctrl+J reads 6 without a relog. | |
| 3 | `/gmgivexp <amount>`, enough for at least one level | ASP rises by one per level gained, live. | |
| 4 | Learn Materials Engineering, click it again; learn Biomedical Engineering | Expertise 1 and ASP down by 1; the second click says "You already know Materials Engineering."; Biomedical is learned for one more ASP. | |
| 5 | Relog | Disciplines, expertise, ASP and blueprints are all still there. | |
| 6 | With nothing in the crafting bag, open J away from the stations; walk to a station; walk away | The four pages are unavailable away from a station, enable at it, and disable again when you leave. | Needs a real client: the pages' look |
| 7 | Buy an **MAS-5 Field Crafting Tool** (8406) and walk away; move it into your vault (`.bank`) and back | The tool lands in the crafting bag. Away from the stations craft, research and reverse engineering enable and alloy does not; with the tool in the vault they disable. | A tool counts only in the crafting bag and cannot go in the backpack |
| 8 | Buy and use **Blueprint: Steel Plating (Materials Subcombine A)** (6483); buy and use a second | Blueprint 25 appears in J. The second: "You already know this blueprint. The item was not used." It stays. | Using a Blueprint item prints no line of its own (K18) |
| 9 | Click **Regenerative Energetics** in Ctrl+J; buy and use **Racial Paradigm Guide: Goa'uld** (7808); click it again | Before: "Regenerative Energetics requires Goa'uld paradigm level 2; yours is 1." After: "Regenerative Energetics requires Biomedical Engineering at expertise 50; yours is 1." | The guide prints no line (K18); `/showracialparadigmlevels` is not implemented |
| 10 | Buy 13 **Steel Core (Materials)** (5254) or `.craftkit 25`; at a station craft Steel Plating (blueprint 25); press Craft again during the bar if you have 13 more | The 3-second bar shows, the cores go, "You crafted Steel Plating (Materials Subcombine A) x1.", Materials Engineering expertise 1 → 2. A second craft during the bar is "queued behind 1 other crafting job(s)". | Buying several cores at once shows one stack (K18) |
| 11 | Craft with only 12 Steel Cores | "You do not have enough components: 12 of 13 needed. Nothing was used." Nothing leaves your bags. | If the client will not send the request, note that |
| 12 | Buy **Crafted Pistol of the Whale** (5481) and the **Materials Engineering Research Kicker** (5671); research the pistol with it at a station; try again with the **BioMedical** kicker (5668) | "Research succeeded: Biomedical Engineering expertise increased to 6. You learned 1 new blueprint." Pistol and kicker used, blueprint 1 in J. The second is refused: "Kickers cannot come from the same applied science as the item being researched. Nothing was used." | |
| 13 | Buy 10 pistols; put all 10 on the reverse-engineering page and confirm | Ten inductions in turn, each "Reverse engineering complete: recovered N components."; components land in the crafting bag. | |
| 14 | `.learnblueprint 42`; buy 1 **Cell (Bio-Medical)** (5192) and 5 **T1 Cell (Bio-Medical)** (5189); alloy blueprint 42 at a station; repeat with four T1 Cells | "Alloying complete: 2 x Blend (Bio-Medical Alloy)." and Biomedical expertise +1. With four: "The quantity of elementary components per item quality was not met: ... Nothing was used." | Five Good tier-1 Cells meet the Good count |
| 15 | Start a craft and log out during the bar; log back in | Nothing consumed, no product. | |
| 16 | Kill the **Crate** and loot it; fill the crafting bag with `.craftkit` kits, kill it again and loot its Cell | The Cell lands in the crafting bag. With the bag full: "Your crafting bag is full. The item was left on the corpse." and the Cell stays on the corpse. | The Crate respawns 30 s after death; a kit that does not fit is refused whole, so finish with `.craftkit 42 <count>` |
| 17 | Two players: A trades B a component from the crafting bag; repeat with B's crafting bag full | It lands in B's crafting bag. Full: the trade closes for both, and each reads a "Trade cancelled: ..." line saying whose crafting bag has no room. Nothing moves. | Two players |
| 18 | `.respeccraft`, answer Yes; `.respeccraft` again; learn a discipline, `.respeccraft`, wait over 60 s, answer Yes | The prompt costs 0. After Yes every discipline reads 0, the ASP spent learning them comes back, blueprints and paradigms stay. Again: "You have no crafting disciplines to unlearn. Nothing was changed." Late: "The crafting respec was not confirmed within 60 seconds. Type .respeccraft to start again." | `.respeccraft` works for any player; the client's own `/respeccraft` is not supported |
| 19 | `.allcraft` | "allcraft [...]: N disciplines at 100, M blueprints, 5 paradigms at 7; craft anywhere is on until logout." Every page enables anywhere. | From here on, research of any item with a tech competency of 100 or less is refused (step 20): do the research steps first |
| 20 | After step 19, buy another Crafted Pistol of the Whale (5481) and research it at a station | Refused at once: "None of your disciplines can learn from that item: research needs one of its disciplines at an expertise above 0 and below 20. Nothing was used." The pistol stays and no bar runs. | Needs CR-18 (D-CR29) in the build. SigNoz: `event = 'rejected' AND reason = 'no_eligible_discipline'` |
| 21 | Right-click a Crafting Station | Its crafting window opens. | **Provisional:** needs CR-19 (D-CR30); until then a click does nothing, which is not a new bug |

Mailing a component from the crafting bag is [Mail, chat and duels](#mail-chat-and-duels) step 3b.

**SigNoz:** start from `service.name = 'cimmeria-server' AND scope_name = 'crafting' AND player_id = <id>`, then `event = 'rejected'` for any refusal (its `reason` says which rule), or find a job with `event = 'queued'` and filter on its `job_id` to see the whole craft, before and after. One query per step: [CR-14 query table](../analysis/crafting/handoffs/session-resume.md#signoz-queries).

**Things only a human can check:** the induction bar and its countdown; whether the pages enable and disable at a station and with the tool; which window a station click opens (step 21); whether any text shows for "no ASP" beside the chat line; whether the reverse-engineering page keeps its slots on confirm.

Source: the [CR-14 checklist](../analysis/crafting/handoffs/session-resume.md#cr-14-owner-uat-checklist) and the [crafting ledger](../analysis/crafting/README.md). Change this section and the checklist together.

## Bank and vault

Your personal vault (bank container 17), opened at a Banker, with deposit, withdraw, stack handling and paid expansion from 40 to 100 slots. Then the Team (19) and Command (20) vaults, the organization treasury, and the Team vault's expansion from 40 to 100 slots, paid from the treasury.

**Status:** Ready. Steps 1-14 (the personal bank) since release 1; steps 15-25 (org vaults, the treasury and the Team expansion) once release 2 is deployed. Known broken: the player-facing Expand offer (K3), so steps 11-12 and 24-25 use the GM commands. Step 19 needs a local server.

**Prerequisites:** a fresh GM character with some naquadah, a stackable item and a mission item. Location: the **Storage Officer** in the debug hub (middle of the right-hand wall). Step 13 needs a non-GM character too.

**Prerequisites for steps 15-25:** two characters online at once, **A** (GM) and **B** (any). A founds a Team with `.org_create team Vault Testers` and a Command with `.org_create command <name>`, reads their ids and each rank's permission mask with `.org_info`, and adds B to both with `.org_join <orgId> <B>`; B joins at the entry rank, which may deposit items and cash but not withdraw. A needs a few hundred naquadah, and B a stackable item (the hub's Common Materials Components vendor sells them at 1 naquadah). The Team and Command Bankers stand in a second row in front of the Storage Officer; all three are named "Storage Officer" (K17). Full setup: [debug hub doc](../content/debug-hub.md#team-and-command-bankers-templates-371-and-372).

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| 1 | Type `.bank` | The vault window opens with 40 slots and does not scroll past them. | |
| 2 | Right-click the **Storage Officer** | The vault opens on the first click. No "Expand vault" dialog appears. | Expand dialog quarantined (K3) |
| 3 | Drag an item from your backpack into the vault; close and reopen | It stays in the vault. | |
| 4 | Drag it back | It lands in the backpack; the vault slot empties. | |
| 5 | Split a stack into the vault, then merge it back | The totals never change. | |
| 6 | Deposit an item, log out, log in, open the vault | The item is still there. `.bankdump <name>` lists it. | |
| 7 | Open the vault at the Banker, walk more than about 5 m away, then drag | The move is refused with a visible message; the item snaps back. | |
| 8 | Try to bank a mission item | Refused with a message. | |
| 9 | Try to trade, sell or mail a banked item | Only carried bags are offered; a banked item cannot be used. | |
| 10 | Sell an item; try to drag it out of the vendor's buyback tab without paying | Refused. | |
| 11 | `.bank`, then `.bankexpand`; repeat to 100; try once more; try with no vault open; try as a non-GM | "Your vault now has 50 slots. You paid 100 naquadah."; cash drops by 100; an open window's scroll range grows to 50 (confirm). At 100: "Your vault is already at its full size of 100 slots." No vault open: "bankexpand: open your vault first (.bank, or a Banker in range). Nothing was charged." Non-GM: ".bankexpand needs GM access. Nothing was charged.", not said aloud. | GM command stands in for the quarantined button (K3) |
| 12 | With less than 100 naquadah, `.bankexpand` | "You need 100 naquadah to expand your vault. Nothing was charged."; nothing changes. | |
| 13 | As a GM away from any Banker, `.bank` and move items; as a non-GM, `.bank` | The GM's vault opens and moves work; the non-GM is refused. | |
| 14 | Drag a stack onto a same-type stack in your backpack | They merge up to the stack limit; the total never changes. | No success telemetry for this; report a wrong total with `.bug` |
| 15 | A right-clicks the **Team Banker** (Storage Officer in the Cellblock guard uniform, second row); then the **Command Banker** (plain crew clothes) | The Team vault opens on the first click with 40 slots; the Command vault with 100. | Confirm the Team window shows 40, not 100 |
| 16 | A character in no Team right-clicks the Team Banker, then the Command Banker | Nothing opens; chat says "You are not in a Team, so there is no Team vault to open." (then "Command"). | |
| 17 | B opens the Team vault and drags the stackable item into it; close and reopen | It stays in the vault. | |
| 18 | B drags it back to a bag; A adds `WithdrawBank` (0x20000) to rank 2 with `.org_set_perms <orgId> 2 <mask>`; B drags again | The first drag is refused with a line and snaps back; the second lands in B's bag. | |
| 19 | Local server only: bind one of B's items in the database, relog B, drag it into the Team vault | Refused with a line; it stays in the bag. | Skip on the colo: nothing in game binds an item |
| 20 | With B's Team vault open, A kicks B from the Team; B drags in that window | B's next drag is refused with a line and snaps back. Record whether B's window closed by itself. | The server sends no close for the window; A re-adds B with `.org_join` before step 22 |
| 21 | Put an item in the Command vault; A types `.org_disband <commandId>`; A withdraws it and disbands again | The first disband is refused with a line and `.org_list` still shows the Command; the second succeeds. | Keep the Team for steps 22-25 |
| 22 | A and B both have the Team vault open; A deposits an item, then withdraws it | The item appears in B's window without reopening, then leaves it. | |
| 23 | A deposits 300 naquadah into the Team treasury from the vault window; B deposits 10; B tries to withdraw; A withdraws 50 | A: "You deposited 300 naquadah into the team treasury. It now holds 300."; B sees the treasury change; B's deposit works; B's withdraw is disabled or refused ("Your rank may not withdraw naquadah."); A: "You withdrew 50 naquadah from the team treasury. It now holds 260." | |
| 24 | A (the Team leader) types `.orgvaultexpand`, then `.orgvaultexpand 40`, then `.orgvaultexpand 40` again; reopen the Team vault | A quote naming the size, the price (100) and the treasury; then "orgvaultexpand: the Team vault now has 50 slots. The treasury paid 100 and holds 160."; the repeat is refused ("the Team vault has 50 slots, not 40"); the reopened window shows 50 slots. | GM command stands in for the quarantined button (K3); B's open window keeps 40 until reopened (K17) |
| 25 | `.orgvaultexpand command`; with less than 100 in the treasury, `.orgvaultexpand 50`; B (non-GM) types `.orgvaultexpand` | "a Command vault is fixed at 100 slots"; "the next step costs 100; the treasury holds N"; ".orgvaultexpand needs GM access. Nothing was charged.", not said aloud. Nothing changes. | A needs a Command for the first; found one again if step 21 disbanded it |

**SigNoz:** filter `scope_name = 'bank'` (the bank checklist writes it as `target=bank`) plus your `player_id`, then:

| Step | Filter |
|---|---|
| 1, 13 | `event = 'vault_session_opened' AND gm_override = true` |
| 2 | `event = 'vault_session_opened'` with `banker_id`; the quarantine shows as `event = 'expand_offer_suppressed' AND reason = 'dialog_quarantined'`; a failed click is `event = 'vault_open_rejected'` |
| 3, 4, 5 | `event = 'move_accepted'` with `kind` = `deposit`, `withdraw`, `split`, `merge` |
| 6 | `event = 'vault_session_closed' AND reason = 'logout'`; `.bankdump` is `event = 'gm_action' AND action = 'bankdump'` |
| 7 | `event = 'move_rejected' AND reason = 'banker_out_of_range'` |
| 8 | `event = 'move_rejected' AND reason = 'mission_item_not_bankable'` |
| 9 | Mail: `scope_name = 'mail' AND event = 'mail.attachment_refused' AND reason = 'item_in_vault'` |
| 10 | `event = 'move_rejected' AND reason = 'source_container_not_player_movable'` |
| 11, 12 | `event = 'expand'` or `event = 'expand_rejected'` with `reason` (`at_ceiling`, `no_vault_session`, `not_gm`, `insufficient_cash`) |
| 15 | `event = 'org_vault_open_requested'`, then `event = 'org_vault_opened'` (with `org_type`, `rank`, `vault_slots`), then `event = 'vault_session_opened'` with `org_id` |
| 16 | `event = 'org_vault_open_rejected' AND reason = 'not_in_org'` |
| 17, 22 | `event = 'org_move_accepted' AND direction = 'deposit'`; step 22 also `event = 'org_vault_fanout'` (`updated_recipients`, `removed_recipients`) |
| 18 | `event = 'org_move_rejected' AND reason = 'missing_permission'`, then `event = 'org_move_accepted' AND direction = 'withdraw'` |
| 19 | `event = 'org_move_rejected' AND reason = 'bound_item_not_org_storable'` |
| 20 | `event = 'vault_session_closed' AND reason = 'org_left'`, then `event = 'org_move_rejected' AND reason = 'no_vault_session'`; the kick itself is `scope_name = 'org' AND event = 'org.kick'` |
| 21 | `scope_name = 'org' AND event = 'org.disband'` with `outcome = 'rejected' AND reason = 'vault_not_empty'`, then `outcome = 'ok'` |
| 23 | `event = 'org_cash_transfer'` with `direction` = `deposit` or `withdraw`, `amount`, and the wallet and treasury before and after; a refusal is `event = 'org_cash_rejected'` with `reason` (`no_permission`) |
| 24 | `event = 'expand_quote' AND scope = 'team'`, then `event = 'expand' AND scope = 'team'` and `event = 'org_cash_transfer' AND direction = 'vault_expansion'`; the repeat is `event = 'expand_rejected' AND reason = 'replay'` |
| 25 | `event = 'expand_rejected'` with `reason` (`command_vault_fixed`, `insufficient_org_cash`, `not_gm`) |

Afterwards the coordinator also reads every `scope_name = 'bank'` WARN row in your session: a WARN no step expects is a finding.

**Things only a human can check:** whether the vault window resizes live after `.bankexpand` while it is open; the snap-back in step 7; the backpack merge total in step 14; whether the Team vault window shows its real size (40, then 50) rather than 100 (steps 15, 24); whether a kick closes B's vault window (step 20); whether B's withdraw control is disabled for a rank without `WithdrawCash` (step 23).

Source: [bank session resume, UAT checklist](../analysis/bank-vault/handoffs/session-resume.md#uat-checklist).

## Mail, chat and duels

Gate mail (text, cash, items, COD, return, expiry), chat (tells, Ignore, channels, flood limit, GM broadcast and mutes) and 1v1 non-lethal duels.

**Status:** Ready, and released: everything social is live on the colo (the last release, at `3684fa7eb`, includes #943 and #946). One part is known broken: the Gate Mail Clerk (Sgt. Harriman, dialog 60104) opens no dialog (K2), so every clerk step in the source uses `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]` instead. Restoring the clerk waits on the crash investigation (K1).

**Prerequisites:** two accounts, **A** (GM) and **B**, each with a character in the stasis room. A needs GM rights for the solo fallbacks in steps 2-4 and for steps 6 and 10. A third account **C** only as the duel spectator in step 11. Solo fallbacks are in brackets. **Before you log in:** if your client ever received the bad dialog push, delete `Cache.en-US\CookedDataDialogs.pak` first (K1). **Duels need two real players on the colo:** the solo partner `sparbot` has no colo account yet (owner question Q-g).

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| 1 | A mails B and a name that does not exist; B logs in later; with B online, A mails again. [Solo: mail yourself] | A sees the bad name reported. B finds the mail at login. Online, B gets "You have new gate-mail from A." with the mailbox closed; with it open the mail appears at once. | |
| 2 | A sends B 100 naquadah; B takes it. [Solo: `.mail cash 100`, take it] | A's balance drops by 125 (25 postage); the mail header refreshes to show no cash. | |
| 3 | A sends B an item; B takes it by right-click; retry with B's bags full. [Solo: `.mail item <typeId>`] | The item leaves A's bags at once and arrives with B. With full bags it stays in the mail and B is told. | Use an ordinary backpack item with `.mail`, never a mission-only one: system mail can still escrow an item nobody can take (#959). The clerk opens no dialog (K2) |
| 3a | A gets a mission-only item with `/gmgiveitem 10 1` ("Gopher Head") and tries to mail it to B, plain and with COD | The send is refused with a line. Nothing leaves A's bags, nothing is debited, and no mail is written. | New with #946. Its pay-COD half cannot be reached from the client any more (no such COD can be sent); a CI guard covers it |
| 3b | A drags a crafting component (for example 5188) from the crafting bag (bag 15) into the attachment slot and sends it; B takes it | It leaves A's crafting bag and arrives as an attachment; B's take places it. | Needs crafting items; see [Crafting](#crafting) (decision D-CR28). If the client refuses the drag, note it |
| 4 | A sends B an item with COD 200; B tries to take it, then pays. [Solo: `.mail item <typeId> cod 200`] | B cannot take the item before paying. After paying, the header refreshes and A gets a payment mail with 200. | |
| 5 | B returns an unpaid COD mail | It arrives back with A, item included; A is told; it cannot be returned again. | |
| 6 | GM: `.mailbox`; `.mail_expire <id>` on an unpaid COD mail, on a plain mail, and on the returned mail from step 5 | The COD mail returns to its sender with the price cleared; the plain mail is deleted; the returned mail with its item is quarantined (gone from the mailbox, counted by `.mailbox`). Each time the command says which. | |
| 7 | A tells B; A tells an offline name; B sets DND and A tells again. [Solo: tell your own name; tell an offline name] | B sees the tell and A sees it was sent. Offline: A is told. DND: A gets B's DND message. Solo: "You cannot send a tell to yourself." | |
| 8 | B ignores A; A sends tells, says, mail and a duel challenge; B un-ignores A | Nothing from A reaches B, and A is told. A stays visible to B. | |
| 9 | Log in and watch the chat; say, squad and tell in turn; paste ten lines as fast as you can | The welcome line is sky blue in the Info tab, with no popup and no "You have joined channel" lines. Each channel lands in its own tab. The extra pasted lines are refused with one "too quickly" line. | Team chat is tested in [Organizations](#organizations) step 9 (ORG-09, #951) |
| 10 | GM: `/gmshout hello` or `.announce hello`; `.announce space hello`; `.location`; `.mute B 1` | The broadcast reaches everyone as a red line with a modal "Server Message" prompt; `space` reaches only your space. GM feedback is sky blue in the Info tab. B's chat is refused with feedback and allowed again a minute later. | |
| 11 | A challenges B from 10 units; B accepts; fight. C stands near | After the countdown A and B can damage each other; C can damage neither and is not hit by A's area abilities. B's health stops at 1 and A is told A won. Nobody dies or drops loot. | Two real players on the colo: `sparbot` has no colo account yet (Q-g). The unit-frame PvP flag does not refresh live during the duel (K14) |
| 12 | Repeat, ending with a forfeit, with B walking out of the arena, with B gating away, and with B's client killed | Each ends the duel; afterwards A and B cannot damage each other. If B leaves during the countdown, A hears "Duel aborted" at once. | Known client limit: A's countdown splash keeps counting. After every duel, right-click a hub NPC such as the Storage Officer: it still responds. (Not the clerk, which opens no dialog, K2) |
| 13 | Challenge yourself, someone 50 units away, someone in another world, someone already dueling, and one player five times in a row | Each is refused with a message. | |

**SigNoz** (the coordinator reads these from each `.bug` bookmark):

| Area | Filter |
|---|---|
| Mail | `scope_name = 'mail' AND player_id = <P>`: every send, take, pay, return, expiry and refusal, each with `reason` |
| Chat | `scope_name = 'chat'`: `chat.tell_*`, `chat.ignore_*`, `chat.gm_broadcast`, `chat.gm_mute`, `chat.channel_rejected` |
| One duel | `scope_name = 'duel' AND duel_id = <D>`, ending in `duel.ended` with its `reason` |
| Flood limit | `scope_name = 'rate_limit'`, grouped by `category` |

**Things only a human can check:** the mail header refresh after take and pay; the chat tab each line lands in and its colour; the modal "Server Message" prompt; the duel countdown splash.

Known client cosmetics, not bugs (owner question Q-k; K14): after a refused send, the Send button stays grey until you press New or Reply, which clears the typed text; the unit-frame PvP flag does not refresh live during a duel.

Source: [SS-UAT in work-packets.md](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release) and the [social session resume](../analysis/social-systems/handoffs/session-resume.md#owner-uat) (the release state, the post-close-out fixes and owner questions Q-a to Q-l). Change this section and SS-UAT together.

## Black market

The auction house: search, bid, buyout, create and cancel listings.

**Status:** Needs client patch. The Black Market window, and with it every search, bid and listing, needs the client patch (the patch DLL and the `BlackMarket.lua` overlay, packets BM-03 to BM-06), which does not ship to testers yet (K16). Without it, only the steps marked `server` below can run.

**Prerequisites:** a GM character for the `.bm_*` commands, and a second character on a second client for the outbid step (U8). The auctioneer is Machra, by the exit doorway of the Cellblock stasis room. Give characters cash with `.givecash`.

| Step | Do | You should see | Needs |
|---|---|---|---|
| U0 | `.bm_seed` | "Listed 8 Black Market auction(s) from the system seller: ids A to B." | server |
| U1 | `.bm_list` | The newest auctions, each with its `#id`, seller, price and time left | server |
| U2 | Right-click Machra | The chat line "The auctioneer opens the Black Market. (No window? ...)"; with the patch, the window opens | server (line), patch (window) |
| U3-U5 | Search with no filter, then `pistol` with tech competency 10-20, then page through after `.bm_seed 40` | Seeded rows with names and icons; the filters narrow them; paging keeps the total | patch |
| U6-U10 | Create a listing, bid, outbid from the second client, buy out the 40-naquadah pistol, cancel a listing | Items and cash move at once, each row updates | patch |
| U11-U13 | `.bm_expire <id>` on a listing with a bid, and on one without; then open both mailboxes | The GM line says sold (to whom, for how much) or returned; the item, and the seller's cash, arrive by mail from "Black Market" | server (settlement and mail), patch (to list and bid) |
| U14-U20 | Bid too low, bid without the cash, bid on your own listing, list a 21st item, list a bound item, bid after walking away, press Watch | The window shows the refusal and nothing changes | patch |
| U21 | Without the patch, click Machra and log out | Only the chat line; the server records that the client never answered | server |
| U22-U23 | `.bm_expire 999999`; `.bm_seed` on a non-GM character | "no auction has id 999999"; "is a GM command" | server |

The full table, with the exact expected lines, the SigNoz query for every step and the error ids, is the canonical checklist: [uat.md](../analysis/black-market/uat.md).

**SigNoz:** the saved Logs Explorer view **Black Market** (`service.name = 'cimmeria-server' AND (event LIKE 'bm.%' OR scope_name LIKE '%black_market%')`), narrowed per step with `event = '<bm.event>'` and `player_id = <P>`. The definition is [black-market.view.json](../operations/signoz/black-market.view.json).

Source: [black-market uat.md](../analysis/black-market/uat.md) and the [black-market plan](../analysis/black-market/README.md). Change this section and uat.md together.

## NPC AI

How NPCs aggro, assist, move, take cover, leash home, face you and fire, in the Cellblock, Castle and the other worlds.

**Status:** Ready. Every packet merged; UAT-1 (2026-09-25) covered the first half and its findings are fixed. Everything merged since that session is untested in a client.

**Prerequisites:** a GM character with `.aggro on` (the default). Locations: the Cellblock topside (Mess Hall, hallways, the med-station desk), Castle and Harset. Step 11 needs mission 680's escort (the Cellblock run).

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| 1 | Walk the Cellblock topside room by room; shoot one Mess Hall guard; pick up the vial; enter Region8; then `.aggro off` | Each room's guards engage when you enter, never through walls or floors. A guard spawned in cover sees you and engages from past its prop. Shooting one Mess Hall guard pulls the other. The PRU drone waits for the vial; the first guard waits for Region8. With `.aggro off` guards ignore you. | |
| 2 | Watch guards that stop to shoot; lead one up a ramp; let one close on you | No running in place; it stays on the ramp surface; it stops about 1 unit short of you. | An SMG guard with a clear line may still close in until the 2 s tick sees it in range; a range-aware stop is an open owner decision |
| 3 | Watch each shot | A visible SMG or pistol fire animation in time with each damage number, at the ability's cooldown cadence. | Castle SMG guards currently fire the Pistol Shot fallback; note which animation you see |
| 4 | Strafe around a fighting guard | It keeps facing you and never fires backward. | Note any lag; the turn updates on the 2 s AI tick |
| 5 | Run past a guard's leash range, or die; then walk back after it resets | It walks home (a walk animation, not a slide), heals, and does not teleport or freeze. Your in-combat flag clears and regen starts. It engages again once the 5 s suppression ends. | |
| 6 | Kill a guard | Its fire stops the moment it dies; the death animation plays once; the corpse stays, loot works, mission progress counts. | |
| 7 | Stand near the med-station desk | The drone fires at you across the desk at 12-16 m. | |
| 8 | Fight guards in cover; shoot one from the front then the flank; stand on top of a ranged guard | Guards hold authored cover and fire from it; guards in the open move to cover. Frontal shots at a covered guard do less damage than flank shots. A ranged guard steps back before firing. | Pose experiment: does the model crouch at its slot? Report what you see |
| 9 | Stand behind a same-floor wall from a guard; target a guard behind a wall and use an ability | No guard shoots through a same-floor wall. Your ability is refused with "You do not have Line of Sight to your target". | |
| 10 | Watch the first guard after Region8, and the PRU after the vial | Once armed, it shows as hostile on its nameplate or target frame. | |
| 11 | Escort Col. Marsh (mission 680) | He follows visibly without an idle-pose slide, survives one failed path query, rides the ring hop with you (chain 1173). Mission-controlled removal still works (chain 1161). | Re-following after combat is expected to fail. Relogging mid-escort leaves him behind (K11) |
| 12 | In Castle, fight guards around walls | They face, fire and deal damage only with a clear line, and never chase through walls. | |
| 13 | Harset: arrive by gate; watch the five mobile world-57 NPCs; walk the raised platforms. Any other world: look for floating or falling NPCs | Arrival lands on the gate row; the mobile NPCs stay on the ground; platforms hold players and NPCs; nothing floats or falls through. | |
| 14 | Dial a gate | The dial opens almost at once, and the gate sequence plays before the load screen. | |
| 15 | After the session, open the **Cimmeria — NPC AI health** dashboard | The dashboard fills in; `cimmeria.deploy_env = 'colo'` appears on rows; `stale_velocity`, `ground_deviation`, leash loop, `idle_parked` and `cleared_without_exit` all read about 0. | Coordinator step. If `deploy_env` still reads `dev`, the compose has not been re-applied |

Open item from the ledger, worth a note if you see it: **one-way player visibility** (one player cannot see the other) in a two-client session. Capture it with `.bug` from both clients.

**SigNoz:** start from your `.bug` bookmark and follow the [telemetry runbook](../operations/npc-ai-telemetry-runbook.md): open the NPC's `entity_id` from `playtest.bookmark.entity`, then the **NPC AI — Timeline for one NPC** view with `AND npc_id = N`, or **NPC AI — What did the client see?** with `AND entity_id = N` for how it looked. A refused ability for line of sight is `scope_name = 'abilities' AND event = 'los_refused'`. Cover stance is `cover.stance event=granted`.

**Things only a human can check:** fire animations and their timing (3); facing (4); the walk-home animation (5); the death animation (6); the cover crouch pose (8); nameplate hostility (10); Marsh's slide (11).

Source: [NPC AI session resume, owner checklist](../analysis/npc-ai-restoration/handoffs/session-resume.md#owner-uat-checklist-colo-after-the-next-release) and [UAT-1 findings](../analysis/npc-ai-restoration/worknotes/uat-1.md).

## Ability trees

Buying abilities at a trainer with training points: three tree tabs per archetype, prerequisites, archetype-wide spend gates, the level cap of 50, respec, and charged abilities.

**Status:** Ready. Every packet merged and released; awaiting UAT.

**Prerequisites:** a GM character per showcase archetype, in this order: Soldier, Commando, Scientist, Archaeologist, Free Jaffa (Shol'va). Location: the **Archetype Skills Trainer** in the debug hub, which offers every tree node (the source names the Interaction Debug NPC, template 25, which the hub trainer copies). Some naquadah for respec (1000).

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| 1 | Open the trainer | Three tabs, in workbook order. Locked nodes are visible and greyed. | |
| 2 | Buy the root | The point counter drops at once; the ability appears in the known list. | |
| 3 | Click a node whose prerequisite you lack (a stale window works) | The press shows an error; nothing is spent. | Does the error show any text, or only a window refresh? Note which |
| 4 | Buy the tier-1 nodes in one branch; watch a tier-2 node in another branch | It opens once archetype-wide spend reaches 4 and its own branch prerequisite is known. | |
| 5 | Relog and reopen the trainer | Learned nodes and points persist. | |
| 6 | Walk away from the trainer and try to buy | Rejected with feedback. | The source says "replay a train packet"; a stale window after walking away is the in-game way |
| 7 | Double-click a purchase | Only one point is spent. | |
| 8 | `/gmgivexp` to level 21, then to 50 | The XP bar behaves (at the cap it is full, never 0); there is no level 51; the capstone opens at 50 once its path and spend are met; you have 50 points in total. | |
| 9 | Respec at the trainer with the Ability window open | Trainer nodes go; starter abilities stay; points are refunded; 1000 naquadah is charged. A hotbar button for a refunded ability stays on the bar and shows an error when pressed. | K13. Note whether the open Ability window drops the removed abilities |
| 10 | Use a trained ability with a charge-up | Damage lands when the charge finishes, not when it starts. Interrupt a charge by moving. | Does the charge bar appear? Is the cooldown zeroed on an interrupt? |

**SigNoz:** `scope_name = 'abilities' AND player_id = <id>`, events `train_requested`, `train_rejected` (with `reason`), `granted` and `train_raw_cost_zero`.

**Things only a human can check:** whether `onErrorCode` shows any text (3, 6, 9); whether the Ability window drops removed abilities after a respec (9); whether the charge bar appears and the cooldown resets on an interrupt (10); whether the XP bar stays sane at level 50 (8).

Source: [AT-06 in work-packets.md](../analysis/ability-trees/work-packets.md#at-06-owner-uat-colo-after-the-release) and the [ability-trees session resume](../analysis/ability-trees/handoffs/session-resume.md#owner-uat-at-06).

## Dialog UI

Dialog buttons that do something on the first press: redundant buttons stripped, button-less closes that report, Col. Marsh's lines as chat barks, and the Castle dialogs' Accept, Decline and Take Missions.

**Status:** Partly works. The 2026-09-26 colo playtest passed T30 rows 1-5, T31 and T32 lines 1-2 (recorded in open PR #826). T32 line 3 is known broken until #826 deploys (K10).

**Prerequisites:** a Tau'ri and a Jaffa character running the Cellblock tutorial, then Castle mission 701. The Castle rows need a server restart before the run (patches apply once at load).

The DU-UAT rows live inside the Cellblock and Castle checklists; they are gathered here so you can report them together.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| T30.1 | At step 3563, right-click Col. Marsh, page to the last screen of 3999 (Tau'ri) or 5023 (Jaffa), press Done | Step advances to 3564; Marsh's indicator clears; the Preparation terminal gets the Livewire cursor. | Passed 2026-09-26 (#826) |
| T30.2 | Start 4001 (Tau'ri) or 5022 (Jaffa) and close it on screen one with X | Mission 641 is accepted as if you had paged to the end. | Passed 2026-09-26 |
| T30.3 | Close dialog 2299 on screen one with X | 2298 shows, 639 is accepted and 638 completes. | Passed 2026-09-26 |
| T30.4 | Look at blurbs 2305, 4000 and 2518 | No Accept, Decline or More Info; only the X. | Passed 2026-09-26 |
| T30.5 | Look at 2516 and 5859 | No buttons. | Passed 2026-09-26 |
| T30.6 | Close dialog 2309 | No buttons; nothing else happens. | |
| T31 | Watch the Straegis aftermath | 2516 appears, then 5859 replaces it after about half a second; no mission changes from closing either. | Passed 2026-09-26. You cannot read 2516 in time: known content timing |
| T32 | Ride the rings, enter the Mess Hall, enter Hallway05 | Three chat lines from "Col. Marsh", no window, nothing interrupts you. | Lines 1-2 passed; line 3 known broken (K10) |
| M1b-a | Server restarted. As a Human, talk to Gerschon | 2573 shows Next on screens 1-6 and Accept plus Decline only on screen 7; Accept starts 701. | Quirk, not a defect: the final screen shows Next and Previous instead of Done |
| M1b-b | As a Jaffa, the same for 5861 | Accept and Decline only on screen 8; the Moh'katan radio call 5862 follows the accept. | |
| M1b-c | Page 2576 to screen 5, press Take Missions | 701 completes; 702 and 703 arrive. | Done sits beside Take Missions and sends nothing |
| M1b-d | Open 2576 and close it early with X | Nothing is granted, no error; Copplemann can be clicked again. | |
| DU-08 | Let one dialog evict another (T31) | The evicted dialog's close is accepted quietly; no chain fires. | Seen working in the 2026-09-26 playtest (#826) |

**SigNoz:** `scope_name = 'dialog.display'` (each dialog shown, with `replaced_dialog_id`), and `body CONTAINS 'fire_dialog_choice'` for each close or button press (`button_id = -1` is a button-less close). A bark is `body CONTAINS 'Content: npc bark'`.

**Things only a human can check:** the button rows on each dialog; the bark's speaker prefix "Col. Marsh" and exact text; that no window opens for a bark; screenshots of 2572, 4001, 5862, 2584, 2580 and a bark line (requested by DU-UAT).

Source: [DU-UAT](../analysis/dialog-ui-redesign/work-packets.md#du-uat), [Cellblock guide M5 and T32](../analysis/castle-cellblock-rebuild/uat-guide.md#milestone-m5--dialog-chrome-du-02a), [Castle M1b](../analysis/castle-rebuild/README.md#validation-and-uat-gates), PR #826.

## Castle Cellblock tutorial

The first zone: the stasis room, Prisoner 329, Col. Marsh's briefing and escort, the cure, the hallway fights, the Straegis scene, the Armory and the exit to Castle.

**Status:** Partly works. The chains are merged; several effects are inert (K7) and some items are known gaps (K8, K9, K11). The 2026-09-26 dialog-UI playtest ran two Cellblock passes and recorded only the dialog rows (PR #826); the other scenarios have no recorded result.

**Prerequisites:** two fresh characters, one **Jaffa** and one **non-Jaffa Tau'ri**. There is no mission reset: delete the character at character-select to start over. Useful GM commands: `/gmgotoxyz`, `/gmgotolocation Castle_CellBlock <x> <y> <z>`, `/gmmissionassign <id> 1`, `/gmmissionadvance <missionId> <stepId>`, `/gmkilltarget <entityId>`; to survive a fight, `/gmsethealthmax <n> 0` then `/gmsethealth <n> 0`.

Relog checks are part of almost every scenario: the full guide says what must come back after each relog.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| T25 | New character: try to walk; check the boots slot; use the worn Prison Boots; win Livewire if it opens. Relog before and after | Mission 689 is hidden; Prison Boots (3438) worn. Record whether you can walk, whether the client offers "use" on a worn item, and whether a Livewire opens. | K7: the lock is inert, so "I could walk" is not a fail. An item named "NO ITEM NAME" is seed data |
| T01/T02 | Enter the world; read the dialog; check the quest log and debuff bar | Dialog 2982 ("The last thing you remember..."); mission 622 on step 2113; Frost's body searchable, the Guard's not yet. | K7: no Stasis Sickness icon expected. Record whether one appears |
| T03/T04 | Search Cpl. Frost's corpse; then the NID Guard's corpse; equip the SI 3 9mm Pistol | Dialog 3995; Frost's Letter (3730) in mission inventory; mission 1360 on step 4037; then dialog 3996, the pistol in your backpack, step 80622; equipping opens the stasis door and completes 622. | Guard corpse may be invisible (K5) |
| T05/T06 | Cross into Region2; right-click Prisoner 329; pick "Free Prisoner 329". Once per archetype | 638 accepted exactly once; exactly one topic: Tau'ri dialog 2300, Jaffa dialog 5021 ("My symbiote will cure me..."). Advances to 2115. | Two topics, or the other archetype's line, is a fail. The "uncomfotable" typo is 2009 data |
| T07 | Win Livewire on the cell-door button; talk to 329 again; agree | Step 2116, the cell door opens; follow-up 2299 (Tau'ri) or 5020 (Jaffa); blurb 2298; 638 completes; 639 accepted. | |
| T26 | Walk into Region8 | The NID guard turns aggressive and comes at you without being shot. | |
| T08 | Pick up the Ambernol vial; take cover behind the med-station desk; kill the drone. Run twice: cover first, then kill first | The TakeCoverIndicator shows on pickup and hides in cover; step 2144 needs **both** objectives before advancing to 2343. | K8: the second checkbox may not tick. 639 completing outright is a hard fail |
| T09 | Use the Ambernol Vial from mission inventory; relog | Exactly one vial consumed; 639 completes; 640 accepted; blurb 2305. After the relog, Stasis Sickness is not re-launched. | K7: no visible debuff change |
| T10 | Win Livewire on the ring switch; right-click it again | Blurb 2305 exactly once; you ring to the Preparation floor; 640 completes; Marsh gets his indicator. | |
| T11/T12 | Talk to Marsh; accept; take the P90 from the locker; equip it; talk again. Once per archetype | Exactly one briefing (4001 Tau'ri, 5022 Jaffa); 641 accepted; blurb 4000 once; SMG in the backpack; Marsh not talkable until you equip; then 3999/5023 and step 3564. | A second P90 from the locker is a fail |
| T13 | Win Livewire on the Preparation terminal | Dialog 3998; 641 completes; 680 accepted on step 2344; the ring switch lights up. | No accept blurb on 680 is correct |
| T27 | Talk to Marsh on step 2344 | Dialog 2309, three screens, speaker "Col. Marsh". | Earlier Marsh dialogs may show a blank speaker label (seed data) |
| T28 | Ride the rings to region 3; look for Marsh at once; walk the topside route | 680 advances to 2345; Marsh appears beside you, follows, keeps pace and paths around geometry. | Highest-risk scenario: Marsh may be invisible (K5); relogging leaves him behind (K11) |
| T32 | See [Dialog UI](#dialog-ui) | | |
| T14 | Enter Region9, then the Mess Hall; kill both guards | 680 completes and 681 is accepted on entry; the second kill completes 681 and accepts 682. | Record whether 681 shows in the quest log |
| T15 | Kill Hallway01-04 and both Hallway05 guards | Each hidden controller mission (682-686) completes on its guard and accepts the next, exactly once. | |
| T29 | Flank a Mess Hall or Hallway05 guard that holds cover (fire from long range, then circle wide) | Objective 2725 (or 2731) ticks; the mission still needs the kills. | May be unreachable: if guards never take cover, record "flank not exercisable" |
| T16/T17 | Kill the last Hallway05 guard; do not move; time it; relog afterwards | t=0: Matinee 1751 plays and Marsh vanishes; ~10.1 s: dialog 2516; ~10.6 s: blurb 5859; 687 accepted. After the relog nothing replays and Marsh stays gone. | Camera-only scene (no creature). Report a broken camera explicitly |
| T18/T19 | Search the wooden crate; kill the three barracks guards. Once per archetype | Tau'ri: dialog 3942 and six stealth-suit items; Jaffa: 3943, jacket and staff. The third kill completes 687 and accepts 688. | Both sets, or a re-grant, is a fail |
| T20 | Read the 688 prompt; use the terminal; use the Armory ring switch | Blurb 2518 once; the terminal advances to 80688 (688 must **not** complete here); the switch completes 688 and moves you to Castle. | No ring animation on this exit is deliberate |
| T21/T22 | Arrive in Castle; open the quest log; find Sgt. Gerschon; check your appearance | You land on the ring platform near Gerschon; 1360 still active; Frost's Letter still in mission inventory; no appearance corruption. | |
| T23 | Relog at every step boundary above | Each restore chain repaints what it should; no one-shot cinematic or blurb replays on login. | The full restore table is in the guide |
| T30, T31 | See [Dialog UI](#dialog-ui) | | |

**SigNoz:** every scenario in the guide names its evidence as a log string (for example `Content: accepting mission` with `mission_id=622`); those rows reach SigNoz too, so search `body CONTAINS '<string>'` around the time. Chain matching is `scope_name = 'content.resolve'`. For an invisible entity: `scope_name = 'aoi.cinematic_hold'` (did the first-login hold arm and release), `scope_name = 'aoi.create_emit'` filtered to the entity id, and `scope_name = 'mercury.retransmit'` over the first 75 s.

**Things only a human can check:** whether any icon appears for Prison Boot and Stasis Sickness (T25, T01, T09); whether the client offers "use" on worn boots (T25); speaker labels (T27); Marsh's visibility and pathing (T28); the Straegis camera (T16); objective checkboxes (T08); appearance after the Castle transfer (T21).

Source: [Castle Cellblock UAT guide](../analysis/castle-cellblock-rebuild/uat-guide.md) (full preconditions, "Must NOT happen" lists and relog checks for every scenario, and its own results table).

## Castle (world 8)

Castle, from the ring-platform arrival: respawn checkpoints, missions 701-708 (Gerschon, Copplemann, Zuritska, Romney, the Communications room, the Throne Room) and the stargate to Harset.

**Status:** Partly works. All packets CA00-CA10 merged; every coordinate (the four respawn checkpoints, the Op-Core respawner, the Armory prefab and the comms room) is a reconstruction, so a bad spot is expected and worth reporting.

**Prerequisites:** a character arriving from the Cellblock with mission 688 complete and 1360 active (or GM travel). One Human and one Jaffa for B2 and B18. Two players for B20.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| B1 | Die once at each of the four Castle checkpoints | You respawn on real ground, not at the origin or under the floor. | Coordinates provisional |
| B2 | Before talking, look at Gerschon | A "!" over him. Human: indicator 2573; Jaffa: 5861. | |
| B3 | Accept 701; talk to Gerschon again | 701 is accepted exactly once. | |
| B4 | Follow Copplemann's step | The step advances with no wave spawning. | |
| B5 | Win the Livewire terminal | 2575 shows once; 2576 completes 701 and accepts 702 and 703. | See Dialog UI M1b-c/d for the buttons |
| B6 | Relog anywhere in B2-B5 | The right indicator re-shows for the current step. | |
| B7 | Go to the Interrogation Block (`Castle_Zuritska_Cell`, 268.0 / 66.79 / 1042.59) | Zuritska (male) and Romney both exist for every player. | |
| B8 | Free Zuritska | 702 completes once. | |
| B9 | Kill Romney | 703 completes. | |
| B10 | Kill a hostile Castle mob; wait | It respawns after about 120 s. | |
| B11 | Walk into the Interrogation Block | Fires 702 step 2402. | Region boxes have ceilings: report the spot if one does not fire |
| B12 | Take Zuritska to the Communications room (Level 5), or enter the region | She follows there, or the step advances on region entry. | Comms-room placement provisional |
| B13 | Win the Communications terminal's Livewire; deliver | Grants 5029 exactly once; delivery starts 706. | |
| B14 | Walk into the ThroneRoom | Advances 2411. | |
| B15 | Use the Access Panel | Completes 706. | |
| B16 | Trigger the surrender or the panel diagnosis | The crystal is revealed. | |
| B17 | At Bravo or Muelbach (the bunker above Bravo), take the crystal | Grants 2790 exactly once. | |
| B18 | Report in: Human to Marsh, Jaffa to Moh'katan | Only your faction's NPC accepts it, never both. | |
| B19 | Win the DHD Livewire; open the DHD list; dial Harset | Harset appears in the list at once, without a relog; the gate opens about 4 s after dialling. | |
| B19a | Relog; open the DHD | Harset is still listed. | Missing after relog = the address was not saved |
| B20 | Two players on different steps of 701-708 | Neither disturbs the other's indicators, actors or steps; a second player on step 2417 can still click Marsh after the first reports in. | Two players |

**SigNoz:** mission chain matches are `scope_name = 'content.resolve'`; a failed address grant logs `reason` starting `grant_address_`; a gate arrival refused off-mesh logs `arrival_unrecoverable_off_mesh`. Anchor each report on a `.bug` bookmark.

**Things only a human can check:** every provisional coordinate (B1, B7, B12); the "!" indicator (B2); the gate's 4 s timing and crossing animation (B19).

Source: [zone operator guide, runbook B](../analysis/zone-restoration-operator-guide.md#b-castle-world-8-ring-platform-build-current-main-all-of-ca00-ca10-merged) and the [Castle ledger's milestones](../analysis/castle-rebuild/README.md#validation-and-uat-gates).

## Harset

Harset (world 57) and its interiors (Command Center 68, Market 69, Storage Room 70): gate arrival, rings, doors, respawners, populated NPCs and regions.

**Status:** Partly works. Travel, combat and population are merged, and the best-guess placements merged in #717. Every placement is an estimate from map data, so mismatches are expected and useful. World 57 runs an advisory navmesh. Mission chains beyond the first few are not written yet.

**Prerequisites:** a character holding Harset's gate address (win the Castle DHD Livewire, [Castle](#castle-world-8) B19). C6.4 and C6.5 need an **ordinary non-GM account**. For placement corrections, stand on the right spot on foot and type `.bug pin <what it is>`.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| C1.1 | Log in on Harset | Nothing from the Cellblock fires (no stray icons, dialogs or objectives). | |
| C1.2 | Right-click a ring switch | A list of four destinations; it does not teleport by itself. | |
| C1.3 | Pick a destination | You arrive on that pad within about 90 s. | An off-mesh pad should abort and leave you where you stood, not freeze you |
| C1.4 | Walk into the Command Center door | You arrive in the Command Center. | |
| C1.5 | Click the Harset DHD | The DHD window opens with the local point-of-origin glyph. | |
| C1.6 | Kill a seeded guard or mob; wait | It respawns in about 30 s. Guards, lieutenants, Petbe and Anat stand still. | |
| C4.1 | Dial an address you do not know | Refused with a client-visible error. | |
| C4.4 | Dial Harset from the Castle DHD; walk into the gate | You land on the gate dais at about (-0.08, -67.27, 38.01), facing down the plaza, standing on the floor. You can walk off at once and are not sent back through the gate. | Current behaviour after NA29. C4.2, C4.2a and C4.3 in the source describe older builds |
| C5.2 | Die once on the plaza | Respawn at about (-8.0, -68.99, 34.0) on the plaza floor. | Guessed placement |
| C5.3 | Die once in the Market and once in the Storage Room | Market about (48.0, 3.61, 78.0); Storage about (50.0, 0.0, 44.0). | Market has no navmesh (MEDIUM) |
| C5.4 | Walk back through the Command Center north door | You arrive at the Harset north door (about (0, -67.6, -231)) and are not bounced back. | |
| C5.5 | Use each of the five ring switches | All five arrive on a real platform. | The navmesh, not the rows, is believed wrong |
| C5.6 | In the Command Center, find Ba'al, the Royal Guard and symbiote tank, Moh'katan, Marsh, Copplemann, Blackstock, Nerus, Opheltes, Athena | Each stands on the floor facing a sensible way. | Blackstock, Opheltes and Athena are LOW confidence |
| C5.7 | In world 57, find Hansen, Jacobs, Lo'rak, the two Former-Ra Jaffa, the Suspicious Jaffa, the SecondBug and ThirdBug baskets, the three shield towers and Shield Controls, the Bank anchor, the Storage Lo'taur and the Petbe quarters search object | Each is at its landmark and clickable. | Shield Controls is the weakest row |
| C5.8 | Walk into each named region (Jaffa Zone, OP-CORE Zone, Bank, Petbe quarters, three shield towers, Shield Controls, the Lab, Marketplace and Storage interiors) | Entering fires the region. | |
| C5.9 | Anywhere a spot is wrong: stand on the right spot on foot, `.bug pin <what>` | The pin records your position and nearby entities. | A spot you cannot walk to is not a valid pin |
| C5.10 | In the doorway that loads the Market, `.bug pin market door`; repeat inside the Market and for the Storage Room both ways | Two doorway coordinates. | These door pairs are unplaced (no evidence); your pins place them |
| C6.1 | Shoot a plaza guard from its side or back beyond about 30 units; strafe | It turns to face you within about 2 s and keeps turning. | `.bug guard not turning` next to it |
| C6.2 | Aggro a gate-side sentry from inside 30 units with a clear view | It shoots back. | |
| C6.3 | Die, respawn, then use a ring switch and the Command Center door | Both still work after the respawn. | |
| C6.4 | **Non-GM account:** walk (no GM travel) east across the plaza and on to the Command Center door | You reach the door on foot without being yanked back. | Worthless from a GM account: the gate is warn-only for GMs |
| C6.5 | Same non-GM character: ring out and back, then die and respawn | Rings still arrive; the respawn leaves you somewhere you can walk away from. | |

**SigNoz:** `movement.validation` rows with `reason = "navmesh"` for C6.4 snap-backs; the boot line `reason = "navmesh_mode_summary"` for world 57 must read `navmesh_mode = advisory`; a C1.3 hang may show `arrival_unrecoverable`; C6.3 should show `respawn: re-registered client-hinted regions after reanchor` followed by `region_hint` lines, and `no_region_hints_since_respawn` from `playtest.friction` means the fix did not take.

**Things only a human can check:** every guessed position and facing (C5); whether you can walk away from each arrival (C4.4, C5); guards turning (C6.1).

Source: [zone operator guide, runbook C](../analysis/zone-restoration-operator-guide.md#c-harset-worlds-57-68-69-70), the [placement ledger](../analysis/harset-rebuild/placements/README.md) and the [Harset session-4 resume](../analysis/harset-rebuild/handoffs/session-4-resume.md).

## Historical cellblocks

Seven historical versions of the Castle Cellblock map (builds 43485 to 63682), each loadable as its own empty world, 1201-1207, for comparing against today's map.

**Status:** Needs client patch. The server side is merged (#831). The seven map folders must be installed in your client; they do not ship to testers. The steps are **derived** from the README's "What the UAT must settle" (confirm with the campaign owner).

**Prerequisites:** a GM account and a client with the historical-cellblocks patch installed. Read K15 first: mixing this server with one that lacks these worlds can empty your client's world table.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| HC-1 | `.gotolocation CellBlock43 -334.231 73.472 -228.026` | The world loads; record load success, streaming of sublevels, geometry and prop differences, collision, native cover-node behaviour, unexpected current content, and any package or namespace errors. | Derived. 43485 is uncooked and has no MapData; it is the likeliest to fail |
| HC-2 | Repeat HC-1 for `CellBlock55`, `CellBlock57`, `CellBlock58`, `CellBlock60`, `CellBlock62` and `CellBlock63` | Same record per world. | Derived. The worlds are empty by design: no NPCs, missions or spawns |
| HC-3 | `.gotolocation Castle_CellBlock -334.231 73.472 -228.026` | Back in the stock Cellblock, which behaves as before. | Derived |

Record each world in the README's table (Loads, Streams, Geometry / props, Collision, Cover nodes, Errors).

**SigNoz:** no dedicated queries are documented. Anchor each world's result on a `.bug` note, for example `.bug CellBlock43 loaded, no streaming`.

**Things only a human can check:** all of it. The recovered packages use older package versions whose loading only the client can settle.

Source: [historical-cellblocks README](../analysis/historical-cellblocks/README.md#what-the-uat-must-settle).

## Ring transport

A full ring-transporter ceremony for the Cellblock-to-Castle exit (mission 688), restored by patching the client map.

**Status:** Needs client patch. Phase 0 passed in-client on 2026-09-19. Phase 1 (the cloned ring rig at the Armory pad) is built and awaits its in-client test. Phases 2-3 are not started, so the exit is still a direct teleport (Cellblock T20). Owner-only: the patched map chunk and its install script live beside the owner's client. The step is **derived** from the README's "To test" paragraph (confirm with the campaign owner).

**Prerequisites:** the owner's client with the Phase 1 chunk installed (`Install-Phase1.ps1 rig`), a server with the sequence overrides, a GM account, something targeted.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| RT-1 | Stand at the Armory pad; `.net_seq 10187 3` | The rings rise and flash as they do at region 3 with `.net_seq 1951 3`. | Derived. If nothing happens, try `.net_seq 1951 3` at region 3 and report both results |
| RT-2 | Walk near the Armory pad | The Cellblock loads normally near the Armory. | Derived. If it fails to load, restore with `Install-Phase1.ps1 original` |

**SigNoz:** none documented. Use a `.bug` note at each attempt.

**Things only a human can check:** whether the cloned rig animates, and whether the pad looks right.

Source: [ring-transport README, Phase 1 status](../analysis/ring-transport-cellblock-castle/README.md#phase-1-status).

## GM console command parity

The legacy emulator's `.`-commands, restored on Cimmeria's dev console: search, stat readouts, grants, spawn authoring, travel and debug.

**Status:** Partly works. Many commands are registered and merged, but the campaign's milestone UAT has not run, and several legacy commands are not registered yet (`.kill`, `.revive`, `.level`, `.givetp`, `.giveitem`, `.removeitem`, `.god`, among others). Steps are **derived** from the README's milestones M1, M2, M4 and M6 and the command reference (confirm with the campaign owner). Use `.help <command>` for each command's arguments.

**Prerequisites:** a GM account, a **distinct selected player** (a second character), an observer where a step says so, and a non-GM account.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| M1-1 | `.help`, `.help searchitem`, `.searchitem <word>`, `.searchmission <word>`, `.searchtemplate <word>`, `.players` | Each answers in chat; the search results name real items, missions and templates. | Derived |
| M1-2 | Select the second player; `.info`, `.stats`, `.primarystats`, `.speedstats`, `.armorstats`, `.qrstats`, `.absorbstats`, `.stealthstats`, `.listabilities` | Readouts describe the selected player, not you. | Derived |
| M1-3 | With the second player selected, `.givecash <n>` and `.givexp <n>` | The selected player's cash or XP changes and their UI updates; you get the feedback line; an observer sees only intended updates. | Derived |
| M1-4 | As a non-GM, type any registered command | "is a GM command"; nobody sees the text. | Derived |
| M2-1 | `.spawn <templateId>`; `.savespawn` twice; move and `.savespawn`; `.delspawn`; `.despawn`; `.visible` off and on with an observer | The entity appears at your position and facing; saves write the database (commit with `.seedconfirm`); the observer sees visibility changes on re-entry. | Derived. On the colo the database resets on deploy (K4) |
| M4-1 | `.gotoxyz <x> <y> <z>`; `.goto <name>`; `.summon <name>`; `.gotolocation <world> <x> <y> <z>`; include a same-world, different-instance destination | Each moves the right player; `.summon` always brings the player to **your** instance and position. A cross-world move goes through a load screen. | Derived. Mandatory: the same-world different-instance case |
| M4-2 | `.speed <0-500>` on a selected player | Their movement and rotation speed change together. | Derived |
| M6-1 | `.net_seq <id> <viewType>`, `.net_timer`, `.net_speak`, `.net_dialog` | The client plays the sequence, timer, speech or dialog. | Derived. `.net_dialog` with a quarantined hub dialog shows nothing (K2) |

**SigNoz:** every `.`-command reply is `scope_name = 'console.feedback'` (first 400 characters), with your `player_id`. Organization GM actions are `scope_name = 'org' AND event = 'org.gm_action'`.

**Things only a human can check:** the selected player's UI after a grant (M1-3); what an observer sees (M1-3, M2-1); the load screen on a cross-world move (M4-1).

Source: [legacy command parity README, Validation and UAT gates](../analysis/legacy-command-parity/README.md#validation-and-uat-gates), its [work packets](../analysis/legacy-command-parity/work-packets.md), and [commands.md, Command families](../commands.md#command-families).

## Recording results

Copy this block once per step you run, fill it in, and send it to the owner (or paste it on the campaign's tracking issue). Keep failures and passes both: a pass is evidence too.

```text
System:        <for example Pets>
Step id:       <for example U6>
Result:        PASS | FAIL | BLOCKED (could not reach the step) | SKIPPED
Time:          <YYYY-MM-DD HH:MM and time zone>
Character:     <name, archetype, level>  Account: <GM | non-GM>
Build:         <service.version from SigNoz, if you know it>
.bug note:     <the exact text you typed after .bug, or "none">
Saw:           <what happened>
Expected:      <what the step says should happen>
After relog:   <same | fixed | worse | not tried>
Known issue?:  <K-number or step note, if it matches one>
```

For a whole session, a summary table works:

| System | Step | Result | Time | Character | `.bug` note |
|---|---|---|---|---|---|
| | | | | | |

A failure worth its own issue gets the `.bug` bookmark's `bookmark_id` and the SigNoz query that shows it. The coordinator for each campaign turns your reports into issues and updates its ledger.
