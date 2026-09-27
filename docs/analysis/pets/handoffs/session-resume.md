# Pets: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [README and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: Complete; awaiting owner UAT

| Packet | Status | PR | Notes |
|---|---|---|---|
| Plan | Done | #856 | Ledger and research notes |
| PT-E1 | Integrated | #863 | Client contract: indices 29/30/31; `Unit.PetN` binds on `ENTITYFLAG_Pet` + `PetOwnerId` (sent after `onEntityFlags`) + `onPetStanceList` |
| PT-S | Integrated | #865 | `pet_summons`, template 350 Straegis, 2826 → 350 |
| PT-01 | Integrated | #870 | Foundation, with the telemetry retrofit |
| PT-02 | Integrated | #892 | Owner hooks, corpse timer |
| PT-06 | Integrated | #889 | Owner XP and kill credit |
| PT-03 | Integrated | #890 | Summon via the ability; `SpeedPet` flag moved off the channel-movement bit |
| PT-05 | Integrated | #896 | Follow, teleport, stances, defend-owner, owner-anchored leash |
| PT-04 | Integrated | #901 | CM 88/89/90 and the ownership guard |
| PT-07 | Integrated | #908 | `.pet`, `.giveability` (persisted), hub trainer 360 / spawn 450 / list 350 |
| PT-11 | Integrated | #918 | Jaffa 351, Prime 352, Lo'taur 353; no-op pet abilities refused with feedback |
| PT-08 | Integrated | #920 | Holy Warrior, To The Death, Heed Our Calling, Lord's Concentration, pet heals (D-PT17 proposed) |
| PT-13 | Done | this PR | Close-out docs |
| PT-10, PT-12 | Not planned / Blocked | — | D-PT01 ephemeral; no turret model |
| PT-UAT | Waiting on the owner | — | The checklist below |

**Owner decisions:** D-PT01 and D-PT02 approved as proposed; the roster changed to Straegis first (D-PT13). No decisions outstanding. D-PT17 (Lord's Concentration's greenfield effect) is adopted at its default unless the owner objects.

**Coordinator:** session cimmeria-b5. ID block: templates 350-369, spawns 450-469 (acknowledged by crafting cimmeria-af and guilds cimmeria-fa).

Known gaps and follow-up issues (#906, #919, #891) are in the README's [Campaign outcome](../README.md#campaign-outcome).

## Resume steps

1. `git fetch origin` and check that nothing new is open on `pets/*` (`gh pr list --search "head:pets/"`).
2. Run the UAT checklist below with the owner on the colo, after a release that contains every packet above.
3. Record each result in #570. A failure gets its own issue with the `.bug` bookmark and the SigNoz query that shows it.
4. Close #570 once UAT passes. The known gaps stay open as their own issues.

## UAT checklist (owner, colo, after the PT-13 release)

Use a GM character. Grant the summon with `.giveability 2826`: it is saved to the character and survives relog, and any archetype can then cast it. `.giveability 1643`, `1644` and `1645` grant the other three summons the same way. Only a Goa'uld can buy the pet nodes at the trainer, and 2826 needs level 50 there. Report failures with `.bug <note>` at the moment you see them, so the playtest bookmark captures the pet's state.

| # | Step | Expected |
|---|---|---|
| U1 | `.pet summon 350` or `.pet summon 2826` | A Straegis Fighter appears beside you, and the chat reply names the pet id (and "via ability 2826" for the ability id). The pet window/bar shows its portrait, name, ability row and three stance buttons, with Defensive highlighted. |
| U2 | Talk to the **Pet Trainer** in the stasis-room debug hub (shown as "Goa'uld Advanced Skills", the last NPC on the back-wall line, right of the crate) | It offers 2826, 1643, 1644, 1645, 1652 and 1654. A level-1 Goa'uld with a training point can buy 1643; 2826 stays greyed below level 50. A non-Goa'uld sees an empty trainer. |
| U2b | `.giveability 2826` with nothing selected | The chat line says it was saved, and the ability appears at once. After a relog it is still known. A second `.giveability 2826` says "already knows". |
| U3 | Cast Summon Straegis | A 6 s warmup with the Goa'uld summon effects, then the pet appears. Casting again replaces it (still one pet). Moving during the warmup interrupts it, and no pet appears. |
| U4 | Walk, run and turn | The pet follows at 2-5 u. Walk 50 u away quickly (or `.goto` somewhere in the same zone): the pet teleports to you within about 5 s. |
| U5 | Attack a hostile guard (Defensive) | The pet engages what you hit and what hits you. After the fight it comes back to you, not to where it was summoned. |
| U6 | Stance buttons (pet window **and** the small pet bar) | Passive: the pet ignores fights, even when hit. Aggressive: it engages hostiles near it. The highlighted button always matches. Both the window and the small bar work (the bar sends slot indices; the server maps them). `.pet stance 0\|1\|2` from the console also works, and the highlighted button follows. |
| U7 | Click a pet ability on the pet bar | The pet uses it on your target. Out of range or on cooldown gives visible feedback. |
| U8 | Let the pet kill a mob | You get the XP, and a KillCount objective for that mob advances. |
| U9 | Die with the pet out | The pet disappears. After respawn, summon again. |
| U10 | Log out and back in; change zone (gate or ring) | The pet is gone after each (D-PT01 ephemeral). There is no orphan pet left in the old zone: `.pet list` from a second character in the old zone shows none. |
| U11 | A second player looks at your pet | They see a Straegis with your name ("X's Pet" if the client shows it). They cannot command it, and it has no pet bar for them. |
| U12 | `.pet info` (select another player's pet to inspect theirs) | It shows the owner name and id, the stance and allowed stances, the AI state, the abilities `[221, 1156]`, what is toggled off, the distance to the owner, and the last teleport ("never" until the pet has teleported). |
| U13 | A non-GM character types `.pet summon 2826` | One chat line, ".pet is a GM command; you do not have GM rights". Nobody nearby sees the text, and no pet appears. |
| U14 | `.giveability 1643`, `.giveability 1644` and `.giveability 1645`, then cast each summon in turn | Each summon replaces the last pet. The Jaffa (Praxis Jaffa armour), the Prime (the Praxis Jaffa Lieutenant look) and the Lo'taur (a Goa'uld in the servant dress and slave headwrap) each render and animate, at your level, with three stance buttons. The Lo'taur look has never been rendered before, so look closely. |
| U15 | Fight with the Lo'taur out, in Defensive and in Aggressive | The Lo'taur follows but holds fire: it never attacks and never heals (known gap). The Jaffa and the Prime fight with their staffs. |
| U16 | With the Prime out, click 1654 Focus Degeneration on the pet bar (or any Lo'taur ability with the Lo'taur out) | The pet does nothing, and chat says "Your pet can't use that ability yet." |
| U17 | `.giveability 2824` (Holy Warrior), then press it twice with a pet out | The first press says "Holy Warrior is on.", the second "Holy Warrior is off.". |
| U18 | `.giveability 2839` (To The Death), then cast it with a pet out | After the 2 s warmup chat says "Your pet fights to the death: it dies in 60 seconds.". After 60 s the pet dies and its corpse despawns 10 s later. You get no XP and no kill credit for it. Casting it again while it runs says "Your pet is already fighting to the death." |
| U19 | `.giveability 2852` (Heed Our Calling), then cast Summon Straegis | The very next summon is instant, with no 6 s warmup. |
| U20 | Dismiss your pet (`.pet dismiss`), then press an owner-pet ability such as Holy Warrior | Chat says "You have no pet to use that on.", and no cooldown is charged. |

## Debugging from telemetry (SigNoz)

Each step above leaves a trail on the `pets.*` targets. Their `OTEL_FILTER` row (`pets=debug`) and their catalog entries are in `docs/architecture/observability.md`. Use the Logs Explorer with `service.name = 'cimmeria-server'`, then:

| Question | Filter |
|---|---|
| Everything pet-related a player did | `scope_name LIKE 'pets.%' AND account_id = <N>` |
| One pet's life (summon to despawn) | `scope_name LIKE 'pets.%' AND pet_id = <id>` |
| Why a summon or command was refused | `scope_name IN ('pets.lifecycle','pets.command') AND reason EXISTS` |
| Why the pet despawned | `scope_name = 'pets.lifecycle' AND event = 'despawned'` (see `reason`) |
| Leash teleports and stance changes | `scope_name = 'pets.ai' AND pet_id = <id>` |
| Kill XP and credit | `scope_name = 'pets.credit' AND account_id = <N>` |
| A spoofed pet command | `scope_name = 'pets.command' AND event = 'ownership_rejected'` (DEBUG; see `reason`, e.g. `not_owner`) |
| Failed summons | `scope_name = 'pets.lifecycle' AND event IN ('summon_failed','summon_refused')` (see `reason`, `stage`) |
| Stance changes | `scope_name = 'pets.command' AND event IN ('stance_set','stance_changed')` |
| A pet-bar order for an ability that does nothing (U16) | `scope_name = 'pets.command' AND reason = 'ability_not_implemented'` |
| Owner buffs on the pet (U17-U20) | `scope_name = 'pets.buff' AND account_id = <N>` (see `event`: `buff_applied`, `buff_removed`, `doom_armed`, `doom_fired`, `owner_ability_refused`, `summon_speed_applied`) |
| Was a `.giveability` saved | `body CONTAINS 'GmGrantAbility' AND subject_player_id = <player>` (see `persisted`, `reason`) |
| A player tried a GM command | `reason = 'not_gm' AND account_id = <N>` |
| Refusal counts by reason | `scope_name LIKE 'pets.%' AND reason EXISTS`, grouped by `scope_name, reason` |

Take the exact `event` and `reason` values from the `pets.*` rows in `docs/architecture/observability.md`. Use `.bug <note>` at the moment of a failure; the bookmark captures the pet's state.

Things the tests cannot show, so watch for them:

- whether the Straegis Fighter body renders and animates (this server has never spawned it);
- whether the Jaffa, Prime and Lo'taur looks render as pets (class 0x05); the Lo'taur composite has never been rendered;
- the trainer's placement (2.1 u from the B-C wall; this room has no navmesh) and its name plate;
- whether the pet bar fills from the owner binding PT-E1 chose;
- the pet's nameplate text;
- the summon VFX;
- whether the small pet bar's stance buttons map correctly.
