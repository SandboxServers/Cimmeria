---
title: "Duel System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Duel System

> **Last updated**: 2026-09-27
> **Status**: Challenge, response, countdown, engaged duel, PvP flag, harm gate and every end path implemented (SS-D1, SS-D2, SS-D3). Duels are non-lethal and carry no rewards (D-SS20, D-SS22).

## Overview

The duel system lets two players agree to fight each other. One player challenges another nearby, the target accepts, a 5-second countdown runs, and then the two can damage each other, and only each other, until one of them forfeits, would be killed by the other (held at 1 HP instead), dies to something else, leaves, or strays outside the duel area. The original design placed `SGWDuelMarker` entities with a proximity detector to track the duel area; Cimmeria keeps the duel in a server-side registry and checks the area on a tick instead (D-SS24), so no marker is spawned.

The `SGWDuelMarker` entity is defined in `entities/defs/SGWDuelMarker.def` (parent: `SGWSpawnableEntity`).

## Implementation Status

Every part of a 1v1 duel is implemented: the challenge and the answer (SS-D1), the countdown and the engaged duel (SS-D2), and every end path (SS-D3), in the social-systems campaign ([ledger](../analysis/social-systems/README.md), [work packets](../analysis/social-systems/work-packets.md)). None of it has been run in the real client yet; the owner's [SS-UAT](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release) steps 11-13 cover it. The duel state lives on the cell in `DuelRegistry` (`crates/cell-world/src/cell/duel/`, a `SpaceManager` resource), keyed by `player_id`. Duels are a cell plugin ([plugin architecture](../architecture/plugin-architecture.md) §4.2): the answer, the forfeit and the tick are in `crates/cell-duel/`, registered by `DuelPlugin`; the challenge, the end paths and the harm gate's inputs stay in `cell-world`.

| Method | Index | Handler |
|--------|-------|---------|
| `sendDuelChallenge` | base 0xD9 | `crates/base/src/base/dispatch/duel.rs`: duel rate limit (D-SS21), squad duels refused, target resolved online (D-SS13), a target whose Ignore list holds the challenger refused with the ignoring line and `reason = target_ignoring` (D-SS15, the SS-C1 cache: `IgnoreCache::ignores_player`), then `DuelBaseToCell::Challenge` to the cell |
| `sendDuelResponse` | 102 | `cell::duel::response`: acts only on the challenge addressed to the caller, consumed once, expired after 30 s |
| `duelForfeit` | 103 | `cell::duel::forfeit`: acts only on the caller's own engaged duel, else "You cannot forfeit a duel until you are engaged in one" (880) |

On the cell, `cell::duel::challenge` refuses a self-challenge (text 872), a target in another space or beyond 20 units (877), either side already in a challenge or duel (873), and the same pair within 60 s of a decline or expiry. Otherwise it stores the challenge and sends the target `onDuelChallenge` [143] with the challenger's entity id and an empty squad list. Decline or expiry tells both players "Duel aborted" (878). Accept starts a 5-second countdown. Every refusal is a feedback line to the challenger. The duel texts are sent as literal feedback lines, because the client has no path that renders a duel moniker by id (SS-E1 D-Q6).

The countdown is shown on both clients: at the accept each duelist gets `onTimerUpdate` with `Type = DuelTimer (14)` on their own entity, which the client turns into `Event_UI_DuelTimerStart` and the splash numbers 5, 4, 3, 2, 1 (SS-D2's trace, [duel-wire-formats.md](../reverse-engineering/findings/duel-wire-formats.md)). The 30 s, 5 s, 20-unit, 60 s and 10-minute values are project policy, not recovered data.

### The engaged duel (SS-D2)

When the countdown runs out, `cell::duel::engage` checks that both duelists are still connected in the duel's space; if one is not, the duel is dropped with 878 (`duel.engage_refused`, `reason = duelist_gone`). Otherwise, for each duelist:

1. `onDuelEntitiesSet([challenger, target])` [151] to their own client, naming only the two duelists;
2. the PvP flag, `onEntityProperty(GENERICPROPERTY_PvPFlag = 4, 1)` on their entity, to their own client and every witness; a player who comes into range later gets it on the AoI enter path, and a `requestEntityUpdate` re-emit replays it;
3. the other duelist becomes a combat source, so `BSF_InCombat` turns on (sent to self and witnesses) and turns off at the end unless a mob still holds them;
4. the line "The duel has begun."

The flag is presentation only: the unit frames flash the PvP indicator from it (`UnitFrames.lua`). Harm is decided by `combat::player_may_attack`, which admits a player target only when `DuelRegistry::can_harm` says the two are an engaged pair in the same space. Every hostility gate goes through it: the single-target launch, the warmup re-check at fire, and the ground-AoE and cone collectors, which scan every NPC plus the caster's engaged partner. A bystander, NPC-versus-duelist and duelist-versus-NPC combat are unchanged.

### The end of a duel (SS-D3)

`cell::duel::end_engaged` is the one clear every end path uses. It removes the duel from the registry, so neither duelist can harm the other from that instant. Then, for each duelist still at the entity that was engaged, it:

1. sets the PvP flag back to 0, for them and every witness;
2. sends `onDuelEntitiesClear` [153];
3. removes every effect the other duelist applied to them (a DoT, a stun, a snare);
4. drops the other duelist as a combat source.

A decided duel sends "You won the duel" (879) to the winner and a feedback line to the loser. An aborted duel sends "Duel aborted" (878) to both. The loser lines are Cimmeria's own wording, because the client has no loss moniker. Nothing is awarded or recorded (D-SS22).

| Path | Where | Loser | `EDuelDefeatReason` | Loser's line |
|------|-------|-------|---------------------|--------------|
| Forfeit (CM 103) | `duel::forfeit` | the caller | `Forfeit` (7) | "You forfeited the duel." |
| Partner damage that would kill | `damage_apply` and the effect pulse, through `duel::clamp_partner_lethal` | the duelist held at 1 HP | `Health` (1) | "You lost the duel." |
| Killed by anyone else | the death resolver (`duel::on_death`); the tick for a player at 0 HP the resolver never saw | the dead duelist | `Health` (1) | "You lost the duel." |
| Disconnect | `SpaceManager::disconnect_entity` (`duel::on_disconnect`) | the disconnecting duelist | `Connection` (3) | none (the client is gone) |
| Teleport, gate travel, any space change | every `TeleportPlayer` / `GateTravel` site (`duel::on_travel`); the tick for a path with no hook | the traveller | `Teleport` (5) | "You lost the duel: you left the area." |
| Range (D-SS19) | the duel tick | outside 40 units of the arena centre for 5 s | `Range` (4) | "You lost the duel: you stayed outside the duel area." |
| Safety limit, both duelists gone, GM `.duel_end` | the duel tick; SS-U2 | none: aborted | — | 878 to both |

The non-lethal end (D-SS20) holds for abilities and for effects. Damage from the partner that would take a duelist to 0 HP leaves them at 1 HP, before the stat update goes out, so the client never sees 0. The duel then ends with them as the loser. Nothing reaches the death path, so there is no corpse, loot, XP, Defeat Window or respawn. The duel ends only after the rest of the same ability has resolved, so a script bleed or a DoT from the same hit is also held at 1 HP and then removed by the end. The effect pulse never re-checks hostility, so the clamp sits in the pulse as well. Damage from anyone else stays lethal: a duelist killed by an NPC dies normally and loses the duel.

Leaving the arena sends the duelist "You are outside the duel area. Return within 5 seconds or you lose the duel." once; coming back stops the clock. The arena is a 40-unit sphere around the midpoint of the two duelists at the accept.

A challenge still waiting for an answer, or a duel still in its countdown, is withdrawn on the same leave paths (disconnect, travel, death): the other side hears 878 at once, and no pair cooldown starts. The 10-minute limit and the tick's departed-duelist check remain as backstops.

151 and 153 are safe beside AoI's use of 152 for interactable NPCs: SS-E1 D-Q5 showed all three only edit a client-side set that the interactability check never reads.

| Feature | Status | Notes |
|---------|--------|-------|
| Duel marker entity | NOT PORTED | `SGWDuelMarker` is defined (client type 6) but not needed: the registry holds the state and the arena is checked on the duel tick (D-SS24) |
| Duel response | IMPLEMENTED | `sendDuelResponse` (CM 102): accept, decline and expiry (SS-D1) |
| Duel forfeit | IMPLEMENTED | `duelForfeit` (CM 103), engaged only, else 880 (SS-D3) |
| Defeat detection | IMPLEMENTED | Server-side, without `SGWDuelMarker`: the 1 HP clamp, death, disconnect, travel and range each end the duel with the client's defeat reason (SS-D3) |
| Duel challenge issue | IMPLEMENTED | `sendDuelChallenge` (base 0xD9) and `onDuelChallenge` [143] (SS-D1) |
| Countdown display | IMPLEMENTED | `onTimerUpdate` type 14 at the accept (SS-D2) |
| Engaged duel, PvP flag | IMPLEMENTED | `onDuelEntitiesSet`, PvP flag to self and witnesses, combat pair (SS-D2) |
| Duel harm gate | IMPLEMENTED | `combat::player_may_attack` at all four gates (SS-D2) |
| Duel end paths | IMPLEMENTED | Forfeit, the non-lethal clamp, death, disconnect, travel, range, plus the safety limit (SS-D3) |
| Duel area enforcement | IMPLEMENTED | A 40-unit arena checked on the duel tick (D-SS19), not the marker's proximity controller |
| Win/loss tracking | NOT PLANNED | D-SS22: no rewards, rating or stats; the result is a feedback line and the `duel.ended` log row |
| Squad duels | NOT PLANNED | `aSquadDuel = 1` is refused with a feedback line (out of scope for the campaign) |
| Pets in a duel | NOT PLANNED | A pet never joins its owner's duel: it cannot harm the opponent or be harmed by them (`combat::player_may_attack_pve`); whether pets may join is an open owner question |

### Known client-side limits

- After a challenge or countdown is withdrawn (the other player left), the client's countdown splash keeps counting down; "Duel aborted" (878) is the only notice, because no cancel form of the type-14 timer is known.
- The unit-frame PvP indicator does not refresh live while a frame is already showing, because of a typo in the client's own Lua; the flag itself is sent correctly. Fixing it needs a client patch.

## Testing and GM tools

- **`sparbot`**, a small `cimmeria-wireclient` binary, logs in a second account, accepts any duel challenge, stands still and forfeits after a set time, so one tester can duel alone. It needs its own account and, against the colo, the colo's `--auth-url`. See [wireclient.md](../architecture/wireclient.md#sparbot-a-duel-partner-for-solo-testing).
- **`.duel_status [name]`** shows a player's duel or challenge: the other player, the duel number, the stage and the time left. **`.duel_end <name>`** ends it in any stage as an abort (878 to both, no loser). Both are GameMaster and above; see [commands.md](../commands.md).
- **UAT.** The owner's checklist is [SS-UAT](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release), steps 11-13.
- **SigNoz.** Everything logs on the `duel` target: `duel.challenge_sent`, `duel.accepted`, `duel.engaged`, `duel.lethal_clamped` and `duel.ended` (with `reason`, `winner_player_id` and `loser_player_id`), plus a `duel.*_refused` row with a `reason` for every refusal. Query `scope_name = 'duel' AND duel_id = <id>` for one duel's story.

## Entity Definition (SGWDuelMarker.def)

**Parent**: `SGWSpawnableEntity`

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `duelDetectorID` | CONTROLLER_ID | CELL_PRIVATE | Proximity detector controller |
| `duelEntities` | ARRAY\<MAILBOX\> | CELL_PRIVATE | Entities participating in the duel |

### Cell Methods

| Method | Args | Purpose |
|--------|------|---------|
| `onEntityDefeated` | entityId (INT32) | Notify marker that a participant was defeated |

## Expected Protocol (from Client References)

Based on client-side event names referenced in the README:

### Client -> Server (NetOut)

| Event | Purpose |
|-------|---------|
| `DuelChallenge` | Challenge another player to a duel |
| `DuelResponse` | Accept or decline a duel challenge |
| `DuelForfeit` | Forfeit an active duel |

### Server -> Client (NetIn)

| Event | Purpose |
|-------|---------|
| `onDuelChallenge` | Incoming duel challenge notification |
| `onDuelEntitiesSet` | Duel participants established |
| `onDuelEntitiesRemove` | Participant left duel area |
| `onDuelEntitiesClear` | Duel ended, clear all participants |

## Duel Flow (as implemented)

```text
Player A: sendDuelChallenge("B", 0)                  base 0xD9
  |-> Base: duel rate limit, B online, B not ignoring A, not a squad duel
  |-> Cell: same space, within 20 units, neither busy, no pair cooldown
  |-> Player B: onDuelChallenge(A's entity id, [])   [143], the Yes/No prompt

Player B: sendDuelResponse(1)                        CM 102 (0 declines; 30 s to answer)
  |-> Both: onTimerUpdate(DuelTimer = 14)            5-second countdown splash
  |-> Countdown ends, both still present:
       |-> Both: onDuelEntitiesSet([A, B])           [151]
       |-> PvP flag onEntityProperty(4, 1) to each duelist and their witnesses
       |-> A and B may damage each other; a bystander, NPC or pet is unchanged

Any end path: forfeit (CM 103), partner hit that would kill (held at 1 HP),
death from anyone else, disconnect, travel, 5 s outside the 40-unit arena
  |-> duel removed from the registry: no more harm between A and B
  |-> PvP flag 0 for each duelist and their witnesses; onDuelEntitiesClear [153]
  |-> the partner's effects removed, combat between them dropped
  |-> winner: "You won the duel" (879); loser: a feedback line
```

The legacy design's `onDuelEntitiesRemove` [152] on leaving the area, and a death-based `onEntityDefeated`, are not used: the arena is checked on the duel tick and partner damage never kills.

## Data References

- **Entity type**: `SGWDuelMarker` (extends SGWSpawnableEntity)
- **World placement**: Duel markers are placed in game world as spawnable entities
- **Detector**: `CONTROLLER_ID` references a BigWorld proximity controller

## RE Priorities

1. ~~**Duel protocol**~~ - Resolved by SS-E1 and SS-D1 ([duel-wire-formats.md](../reverse-engineering/findings/duel-wire-formats.md))
2. ~~**PvP flag handling**~~ - Resolved by SS-D2: the flag rides `onEntityProperty(4, v)` and is presentation only; the server's duel registry decides harm
3. **Duel area bounds** - How `duelDetectorID` defines the valid duel region. Moot for the server: the arena is a 40-unit sphere around the duelists' midpoint at the accept (D-SS19, project policy)
4. **Death handling** - Whether duel defeat uses normal death or special "downed" state. Moot for the server: D-SS20 makes partner damage non-lethal whatever the original did (SS-E1 D-Q3)
5. **Rewards/penalties** - Any XP, rating, or currency effects from duel outcomes. None exist in the client data; D-SS22 awards nothing

## Related Docs

- [combat-system.md](combat-system.md) - Combat mechanics used during duels
- [stat-system.md](stat-system.md) - Stats applied during PvP
- [chat-system.md](chat-system.md) - The Ignore list, which also blocks duel challenges
- [duel-wire-formats.md](../reverse-engineering/findings/duel-wire-formats.md) - Client evidence for every duel method
- [Social-systems ledger](../analysis/social-systems/README.md) - Decisions D-SS18 to D-SS25 and the owner questions
