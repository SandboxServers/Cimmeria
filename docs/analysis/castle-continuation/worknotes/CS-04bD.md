# CS-04bD seal: guard-post and defense side stories

> **Type:** How-to (design seal)
> **Audience:** Castle mission-systems reviewers, npc-ai-spawn reviewers and the CS-04bD coder packets
> **Last updated:** 2026-10-18
> **Companions:** [mission catalog and ID reservation](../evidence/mission-catalog.md), [reconciliation](../evidence/reconciliation.md), [Castle work packets](../../four-zone-planning/castle-work-packets.md)

**Status: draft design seal, corrected by the R3a review.** Three quests are premised below. No quest is `SEALED`. Every implementation verdict is `BlockedDecision (D-CA16)` first and `BlockedEvidence (catalog proof)` second, per the verdict table in `r3-packets-2.md`. Nothing here is client-proven. Every client-only claim is `UATPending (end UAT)` (D-CA11).

Labels: `CIMMERIA_AUTHORING` marks an authored premise, copy or number. `REPO` marks a fact read from this worktree. `OPEN` marks a fact this seal does not prove.

## Scope

- Quests: `defense_q1` (guard-post intelligence), `defense_q2` (prisoner safety), `defense_q3` (defense). None is the catalog proof. The first authored catalog proof is Cellblock's CB-N05 or Castle's `medical_q1` (D-CA16), so this seal does not touch `CS-00B client proof`.
- The "s" and "c" packets split later: `castle_side_defense` is the test module name (`castle-work-packets.md`, CS-04a/b/c row).
- Authority: O14-10/11 (D-CS11 in `README.md`) delegate lore-safe authored premises, copy and modest rewards. No owner premise approval is needed for the three premises below. The gate that remains is D-CA16: the first authored mission's journal, dialog, advance and relog proof.

## Source join

| Fact | Source | Limit |
|---|---|---|
| Chamber guards 257-259 sit on isolated navmesh component 468; infirmary/Ogilvie on component 250; Bravo, bunker and Alpha on 116 | `REPO` `evidence/reconciliation.md:56` | Prior ledger. No `nav_inspect` run in this seal. |
| `every_castle_patrol_leg_is_routable` documents a partial-path regression when a patrol leg enters the chamber | `REPO` `reconciliation.md:56` | Package identity does not prove door or ring access. |
| A content `SpawnEntity` puts the NPC in the acting player's space and tags it for `entity_dead_tag` / `interact_tag` / `despawn_entity` | `REPO` `crates/cell-content/src/cell/content/executor/spawn/mod.rs:23-25` | The seed row never names a space. |
| A content spawn is one-shot even when the template opts into respawn | `REPO` `executor/spawn/tests.rs:230-235` | So a content spawn can never "respawn where it was placed". |
| A content spawn into a non-instanced world is refused (`shared_world_refused`) unless the action row sets `allow_shared: true` | `REPO` `executor/spawn/mod.rs:40-44,135-150` | Castle is `Instanced="false"` (`entities/spaces.xml:9`); only `Castle_CellBlock` is instanced (`spaces.xml:10`). |
| `fire_entity_death` credits the **killer** only: the event is raised on the killer entity, so mission and step context and `IncrementCounter` use that player. A pet's kill credits its owner | `REPO` `event_dispatch/lifecycle/mod.rs:189-244`; `cell-combat/src/cell/abilities/use_ability/kill_credit.rs:98-108` (`credited_player`) | No witness or group fan-out exists in that path. |
| Existing placed Castle hostiles are already killable by tag: `entity_dead_tag` chains on `Castle_BravoOfficer1..3` and `Castle_Muelbach` use scope `space` | `REPO` `db/resources/Content/Seed/castle_706_708_chains.sql:524-626` (chains 1346-1350) | A new quest chain may key the same tags with its own chain id. |

## Shared actors

- **Credit is per player, and the killer only.** `entity_dead_tag` credits the killing player (or a pet's owner), never every witness and never the group (source join above). The earlier draft marked this `OPEN`; it is answered from `fire_entity_death` and `credited_player`. The `c` packet still adds a replay test that two players in range of one kill leaves the non-killer's counter at 0.
- **A shared actor is never despawned or retagged for one player.** Chamber guards, Ogilvie and the Castle prisoners are shared. A quest changes only per-player state (dialog-set bindings, step state). It never calls `despawn_entity` or an `interact_tag` retag on a shared NPC.
- **A new hostile is a placed spawn, not a content spawn.** Castle is non-instanced, so a content `SpawnEntity` hostile is refused (`shared_world_refused`), and `allow_shared: true` would spawn it for every player in the world, one-shot, which breaks the per-player rule. The spec's rule ("respawns where it was placed") is met only by a `spawnlist` placement row. `defense_q3` therefore has two designs, and the first is preferred:
  1. **Reuse the existing placed hostiles** (the tags in the source join). No new template, spawn or `nav_inspect` is needed, and respawn is already the placement's own.
  2. **A new placed hostile** on component 116 (not 468): a new template and `spawnlist` row, outside every other aggro radius, with `nav_inspect` exit 0 and a path from the Castle arrival. `OPEN`: not run here. Needs the `## Shared actors` clearance numbers from the live navmesh.

## Quests

### defense_q1: guard-post intelligence (`CIMMERIA_AUTHORING`)

**Premise.** A chamber guard on post at the Castle route tells a player the change-over times, if the player has first reported to the infirmary. One paragraph is the whole story. The guard does not talk about the Goa'uld chain of command; the answer is a rota the guard already knows.

**Proposed IDs (not allocated).** Key `defense_q1`. Mission 10310. Steps 10340-10341. Objectives 10460-10461. Tasks 10580-10581. Dialogs 10700-10701. Dialog sets 10760. Screens 10800-10803. Screen buttons 10920-10923. Chains 10100-10104. Each lies inside its CS-00Be sub-block (`evidence/mission-catalog.md` section 2). A scan of `db/resources/**/*.sql` for these IDs as first column of `missions`, `mission_steps`, `mission_objectives`, `mission_tasks`, `dialogs`, `dialog_sets`, `dialog_screens`, `dialog_screen_buttons` and `content_chains` found no collision (R3a review). That scan did not cover the sibling CS-04aD and CS-04cS proposals, which are not committed yet, and CS-07's `id-allocation.md` has not assigned sub-ranges per packet, so the IDs stay proposals.

**Reward.** Zero XP, zero naquadah, no item. Comparator: all twelve Castle missions (701-704, 706, 708, 1360, 1376, 1520, 1521, 1664, 567) have `reward_naq` 0 and `reward_xp` 0 in `db/resources/Missions/Seed/missions.sql`. No crate reads those two columns, so the real reward numbers live in the mission XML and the `mission_rewards` groups, which this seal does not freeze.

**Verdict:** `BlockedDecision (D-CA16: authored quest after the first needs the owner answer; then BlockedEvidence (catalog proof) and a text-ID claim)`.

### defense_q2: prisoner safety (`CIMMERIA_AUTHORING`)

**Premise.** Ogilvie in the infirmary asks the player to carry a warning to a Castle prisoner before the guard change, so the prisoner is moved to the infirmary side. The prisoner is a shared actor; the quest only changes this player's dialog binding and step state.

**Proposed IDs (not allocated).** Key `defense_q2`. Mission 10311. Steps 10342-10343. Objectives 10462-10463. Tasks 10582-10583. Dialogs 10702-10703. Dialog sets 10761. Screens 10804-10807. Screen buttons 10924-10927. Chains 10110-10114. Same collision scan and caveats as `defense_q1`.

**Reward.** Zero XP, zero naquadah, no item (same comparator).

**Verdict:** `BlockedDecision (D-CA16)`, then `BlockedEvidence (nav proof: Ogilvie component 250 to the prisoner; catalog proof; text-ID claim)`.

### defense_q3: defense of the Bravo route (`CIMMERIA_AUTHORING`)

**Premise.** A bounded hostile group threatens the Bravo and bunker route (component 116). The player helps the guard post hold it for one wave by killing the placed hostiles tagged in the source join. The fight is local: aggro is limited to the placed hostiles, which respawn where they were placed. Kill credit is the killer's own (see Shared actors).

**Proposed IDs (not allocated).** Key `defense_q3`. Mission 10312. Steps 10344-10346. Objectives 10464-10466. Tasks 10584-10586. Dialogs 10704-10705. Dialog sets 10762. Screens 10808-10809. Screen buttons 10928-10929. Chains 10120-10126. Same collision scan and caveats as `defense_q1`.

**Reward.** Modest, within the O14-10/11 delegation. Proposed: zero item, and naquadah/XP numbers fixed at freeze against the XML of mission 1520 or 1521 as the Castle comparator. The numbers are not frozen here.

**Verdict:** `BlockedDecision (D-CA16)`, then `BlockedEvidence (catalog proof; per-player multi-killer replay test; for design 2 only, nav_inspect on component 116)`.

## Behavior (all three)

- **Offer conditions:** world 8, prerequisites by the prior quest's completion, and the player not already holding the mission. The offer guard (`cell-content/src/cell/missions/lifecycle.rs:89-100`) refuses ACTIVE and already-completed missions once `repeats > num_repeats`.
- **Once only:** a completed mission cannot reopen. Abandonment may reopen the offer only while the prerequisite stays completed.
- **Relog:** each step restores from its chain (`player_loaded` restore chain for each quest).
- **Refusals**, each with a telemetry reason (`OPEN`: the reason keys are proposed; the `s` packet pins them to existing mission logs):
  - wrong world: `castle_side.defense.refuse.world`;
  - prerequisite missing: `castle_side.defense.refuse.prereq`;
  - already completed: `castle_side.defense.refuse.done`;
  - target out of range or wrong space: `castle_side.defense.refuse.target`.

## Frozen rows

Not frozen in this seal. Items (a) to (f) of the spec need the D-CA16 answer, a text-ID claim, the reward numbers and, for design 2 of `defense_q3`, a `nav_inspect` proof. This seal produces no exact XML bytes, no byte counts against `MAX_MISSION_XML_BYTES` and no seed rows. The coder packet freezes them after those gates clear.

## Blockers

1. **D-CA16.** The owner answer on building authored quests before a live catalog proof (`RESUME.md:25`; recommended: hold).
2. **Text IDs.** `texts.moniker_id` and `TextStrings.pak` are 10000-10999 with 995 of 1000 taken (`evidence/mission-catalog.md` section 2), and the scan above shows the proposed ID values are used as text monikers. The CS-00Be sub-blocks do not cover texts, so the copy for all three quests needs a claim from the cross-zone reserve by amending `id-allocation.md` (CS-07). This seal does not edit that file.
3. **Dialog set maps.** Dialog set map rows are not in a sub-block; each seed uses an explicit id above the seed max (1000000) and checks four refs.
4. **Nav proofs.** `defense_q2`'s prisoner route needs `nav_inspect` on the live navmesh. `defense_q3` needs it only for design 2. The lane allowed `nav_inspect` for this wave and the coder did not run it, so this is an unrun check, not a missing tool.

## Test lists

- `s` packet: `cargo nextest run -p cimmeria-cell-content -E 'test(castle_side_defense)'`; live DB `live-db-test.ps1 live_db_castle_side_defense`.
- `c` packet: chains and replay through the real executor, with a two-player credit test (the killer's counter advances, a witness's does not).

## End-UAT rows

- `defense_q1`, `defense_q2`, `defense_q3`: journal title, dialog, advance, relog. `UATPending (end UAT)`, listed in `uat/end-uat.md` at the end. None of these rows is the CS-00B client proof.

## Verdicts

- `defense_q1`: `BlockedDecision (D-CA16)`, then `BlockedEvidence (catalog proof; text-ID claim)`
- `defense_q2`: `BlockedDecision (D-CA16)`, then `BlockedEvidence (nav proof for the component 250 route; catalog proof; text-ID claim)`
- `defense_q3`: `BlockedDecision (D-CA16)`, then `BlockedEvidence (catalog proof; per-player credit replay test)`

Count rule: this seal adds three quests. CS-04aD, CS-04bD and CS-04cS must seal 5-8 together. None of the three is `SEALED`, so the architect pass before R3d is expected.
