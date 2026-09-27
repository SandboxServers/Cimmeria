# Pets: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [README and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan published, Wave 0 dispatching

| Packet | Status | Branch / PR | Notes |
|---|---|---|---|
| Plan | Done | #856 | Ledger and research notes |
| PT-S | Done | #865 | `pet_summons`, template 350 Straegis, 2826 → 350 |
| PT-E1 | Review | #863 | Client contract: indices 29/30/31; `Unit.PetN` binds on `ENTITYFLAG_Pet` + `PetOwnerId` (sent after `onEntityFlags`) + `onPetStanceList` |
| PT-01 | Review | #870 | Foundation; telemetry retrofit in progress |
| PT-02 | Writing (telemetry retrofit) | `pets/pt-02-lifecycle` | Owner hooks, corpse timer |
| PT-03 | Writing (telemetry retrofit) | `pets/pt-03-summon` | Summon via 2826; `SpeedPet` flag moved off the channel-movement bit |
| PT-04 | Writing (telemetry retrofit) | `pets/pt-04-commands` | CM 88/89/90 and the ownership guard |
| PT-05 | Writing | `pets/pt-05-ai` | Follow, teleport, stances |
| PT-06 | Writing (telemetry retrofit) | `pets/pt-06-kill-credit` | Owner XP and kill credit |
| PT-07 | Writing | `pets/pt-07-uat-tooling` | `.pet`, `.giveability`, hub trainer |
| PT-08, PT-11 | BlockedDependency | — | Wave 2 |
| PT-10, PT-12 | Not planned / Blocked | — | D-PT01 ephemeral; no turret model |

**Owner decisions:** D-PT01 and D-PT02 approved as proposed; the roster changed to Straegis first (D-PT13). No decisions outstanding.

**Coordinator:** session cimmeria-b5. ID block: templates 350-369, spawns 450-469 (acknowledged by crafting cimmeria-af and guilds cimmeria-fa).

## Resume steps

1. `git fetch origin` and read the status column above against the open `pets/*` PRs (`gh pr list --search "head:pets/"`).
2. For each packet in Review: check CI. Merge it if green and its CI is newer than `main`; otherwise rebase and re-test first.
3. Dispatch the next wave per [work-packets.md](../work-packets.md#dependency-graph-and-waves), one worktree and one test DB per worker.
4. Update this file after every merge.

## UAT checklist (owner, colo, after the PT-13 release)

Use a **Goa'uld** character (Castle Cellblock start) with GM rights for the `.` console. Every step says what to look for. Report failures with `.bug <note>` at the moment you see them, so the playtest bookmark captures the pet's state.

| # | Step | Expected |
|---|---|---|
| U1 | `.pet summon 350` | A Straegis Fighter appears beside you. The pet window/bar shows its portrait, name, ability row and three stance buttons, with Defensive highlighted. |
| U2 | Talk to the **Pet Trainer** in the stasis-room debug hub and buy **Summon Straegis** (2826) | The purchase succeeds and the ability appears in your ability window. |
| U3 | Cast Summon Straegis | A 6 s warmup with the Goa'uld summon effects, then the pet appears. Casting again replaces it (still one pet). Moving during the warmup interrupts it, and no pet appears. |
| U4 | Walk, run and turn | The pet follows at 2-5 u. Walk 50 u away quickly (or `.goto` somewhere in the same zone): the pet teleports to you within about 5 s. |
| U5 | Attack a hostile guard (Defensive) | The pet engages what you hit and what hits you. After the fight it comes back to you, not to where it was summoned. |
| U6 | Stance buttons (pet window **and** the small pet bar) | Passive: the pet ignores fights, even when hit. Aggressive: it engages hostiles near it. The highlighted button always matches. Both the window and the small bar work (the bar sends slot indices; the server maps them). |
| U7 | Click a pet ability on the pet bar | The pet uses it on your target. Out of range or on cooldown gives visible feedback. |
| U8 | Let the pet kill a mob | You get the XP, and a KillCount objective for that mob advances. |
| U9 | Die with the pet out | The pet disappears. After respawn, summon again. |
| U10 | Log out and back in; change zone (gate or ring) | The pet is gone after each (D-PT01 ephemeral). There is no orphan pet left in the old zone (check with a second character or `.pet list`). |
| U11 | A second player looks at your pet | They see a Straegis with your name ("X's Pet" if the client shows it). They cannot command it, and it has no pet bar for them. |
| U12 | `.pet info` | Owner, stance, ability list, AI state and distance match what you see. |

## Debugging from telemetry (SigNoz)

Each step above leaves a trail. Use the Logs Explorer with `service.name = 'cimmeria-server'`, then:

| Question | Filter |
|---|---|
| Everything pet-related a player did | `scope_name LIKE 'pets.%' AND account_id = <N>` |
| One pet's life (summon to despawn) | `scope_name LIKE 'pets.%' AND pet_id = <id>` |
| Why a summon or command was refused | `scope_name IN ('pets.lifecycle','pets.command') AND reason EXISTS` |
| Why the pet despawned | `scope_name = 'pets.lifecycle' AND event = 'despawned'` (see `reason`) |
| Leash teleports and stance changes | `scope_name = 'pets.ai' AND pet_id = <id>` |
| Kill XP and credit | `scope_name = 'pets.credit' AND account_id = <N>` |
| A spoofed pet command | `scope_name = 'pets.command' AND reason = 'not_owner'` |

Take the exact `event` and `reason` values from the `pets.*` rows in `docs/architecture/observability.md`. Use `.bug <note>` at the moment of a failure; the bookmark captures the pet's state.

Things the tests cannot show, so watch for them:

- whether the Straegis Fighter body renders and animates (this server has never spawned it);

- whether the pet bar fills from the owner binding PT-E1 chose;
- the pet's nameplate text;
- the summon VFX;
- whether the small pet bar's stance buttons map correctly.
