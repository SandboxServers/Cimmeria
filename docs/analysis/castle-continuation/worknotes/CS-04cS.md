# CS-04cS: return-visit route and Harset contract seal

> **Type:** Reference. **Audience:** Castle orchestrator, mission-systems and movement reviewers.
> **Status:** partial seal, corrected by the R3a review (2026-10-18). **Base:** `553d8128d` (CS-00Be evidence in `evidence/mission-catalog.md`).

Scope: part 2 section CS-04cS (`r3-packets-2.md`), with the same output shape as CS-04aD. No seed edit and no live-DB run. The route proof ran `nav_inspect` read-only against `data/spaces/castle.nav` through the build lane; the Castle server (PID 36540) and DB `sgw_castle_cs_01b` were not touched.

Labels: `REPO` is a fact read from this worktree. `CIMMERIA_AUTHORING` is an authored premise. `OPEN` is not proven here.

## Point 1. The route (movement-teleport-advisor)

Facts, `REPO`:

- **Harset to Castle arrival row:** `db/resources/Worlds/Seed/stargates.sql:83`, `stargate_id` 2, name `The Castle`, `world_id` 8, `x_pos` 761.677, `y_pos` 63.466, `z_pos` 551.716, `yaw` 2.152, `event_set_id` 10011, address `3,22,4,20,9,7`. A gate arrival lands on the row's own x/y/z/yaw.
- **Harset side:** `stargates.sql:65`, `stargate_id` 3, name `Harset`, world 57, `(-0.076, -67.274, 38.011)`, yaw 3.141.
- **Castle DHD:** `spawnlist.sql:272`, spawn 2, `Castle_DHD`, template 162, `(806.27, 55.10, 517.24)`. About 56 m in XZ from the arrival.
- **Other actors at the DHD (mission 708 cast):** `Castle_ColMarsh` spawn 118, template 10, `(810.73, 55.20, 515.01)`; `Castle_Mohkatan` spawn 120, template 54, `(807.48, 55.21, 515.40)` (`spawnlist.sql:264`; chain tags in `castle_706_708_chains.sql`).

`nav_inspect` proof, run 2026-10-18 on `data/spaces/castle.nav` (the earlier draft said `nav_inspect` is not a binary here; it is `crates/navmesh-extractor/src/bin/nav_inspect.rs`, documented in `docs/engine/navmesh-build-pipeline.md` section 5, run through `lane.ps1 cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect`):

| Probe set | Result |
|---|---|
| arrival, DHD, Col Marsh, Moh'katan | all `ok`, `h=0.00 m`, component **116**, **exit 0** |
| the same plus Ogilvie (spawn 197) and Castle Medic (spawn 198, infirmary) | Ogilvie and the medic are on component **250**; **exit 3** |

So the Harset arrival reaches the DHD, Marsh and Moh'katan on one component, and does **not** reach the infirmary actors. `--gaps --gap-h 6 --gap-v 6` finds no direct link between 116 and 250 and a three-hop chain: 116 to 181 at `(725.6, 30.4, 462.9)` (h 3.23, dy +3.40), 181 to 216 at `(675.6, 18.6, 488.3)` (h 3.12, dy +2.38), 216 to 250 at `(622.4, 24.0, 508.2)` (h 1.32, dy -1.30); widest hop 3.40 m. The agent climb is 0.6 m, so the mesh as built does not join them. The mesh bounds server NPC routing; this seal does not claim a player cannot walk 116 to 250 (client physics is `UATPending`).

Consequences for the return quests:

- A return quest NPC must be on component 116 for the arrival-reaches-NPC proof. Col Marsh and Moh'katan qualify with no new spawn.
- An infirmary NPC (Ogilvie, the medic) does not qualify without a nav bridge, which is a navmesh change outside this packet.
- **Path leg not run.** The spec asks for a path as well as the component. The lab-server `nav_path` tool needs the running Castle server, which this wave must not touch. `OPEN`.

Live arrival: `UATPending (end UAT, D-CA11)`.

## Point 2. The return trigger contract

- **Harset-published contract:** none sealed. The Harset ledger (`content/harset-campaign` at `0f0ce57c2`, `docs/analysis/harset-continuation/`) records chain 1366 (Castle to Harset, advances 1360 from 4037 to 4038; `castle_carry_in_chains.sql:7`) and the 1664 interface (HA-D05 `Provisional`; HA-01b `BlockedEvidence`: grant edge, Kaerlia placement and dialog graph unproved). It has no "back in Castle" state.
- **The earlier draft said no Harset mission ID is named. That is wrong.** Two Harset-side completions exist in the seeds: mission **1360** (Frost's Letter, one-shot carry-in; its Harset receipt is 6501/6502/6505 in world 68, integrated at the Harset campaign tip, owner-manual UAT pending) and mission **1664** (Castle exit grant, Harset debrief steps 5224-5226, HA-01b). Both rows are in `missions.sql`.
- **Trigger semantics (`REPO`):** `stargate_crossed` fires from `on_stargate_passage` just before the world-transition teardown (`castle_706_708_chains.sql:828-832`), with `event_key` = the DESTINATION world name. The chain context's `world` is the **origin** world (`event_dispatch/stargate.rs:89-95`), which is why chain 1366 gates `world 8` for Castle to Harset. A return chain keyed `stargate_crossed 'Castle'` therefore gates `world 57` (Harset), not `world 8`, and its actions run before the player lands. The earlier draft's "pair `stargate_crossed` Castle with `mission_status`" was missing that gate.
- **Castle-only fallbacks, no Harset change:**
  1. `stargate_crossed 'Castle'`, conditions `world 57` and `mission_status 1360 completed` (or 1664). Acts on the origin side.
  2. `player_loaded 'Castle'` with `world 8`, `mission_status <1360 or 1664> completed` and the return quest `not_active`. Acts after arrival, and is the form chain 1399 uses (`evidence/1664.md`). It fires on every Castle login, so the `not_active` guard is what makes it once.
  Either way the marker is a mission state the player earns in Harset: **1360** covers only players who ran the Cellblock carry-in; **1664** is the general exit beat but its completion needs HA-01b.

## Quests (design, `CIMMERIA_AUTHORING`)

IDs are not picked: CS-07's `id-allocation.md` has not assigned per-packet sub-ranges, and the sibling CS-04aD and CS-04bD proposals overlap the same CS-00Be sub-blocks. Authority: O14-10/11 (D-CS11) delegate the premises, copy and modest rewards.

- **`return_q1`:** after the player has completed the Harset marker mission, Col Marsh (spawn 118) asks for a short report on what the player saw at Harset. One dialog, one talk step, zero item. `OPEN`: Marsh carries 708 dialog bindings (template 10); the `s` packet must check the dialog-slot map for a clash and bind per player only.
- **`return_q2`:** Moh'katan (spawn 120) asks the player to confirm the Harset gate address to him at the DHD. One dialog, one step at the DHD (region or interaction), zero item.
- Rewards: zero XP, zero naquadah, no item, matching the Castle baseline (`reward_naq` and `reward_xp` are 0 for all twelve Castle missions in `missions.sql`).
- Shared actors: Marsh and Moh'katan are 708 actors. A return quest changes only per-player dialog bindings and step state. It never despawns or retags either NPC.

## Verdict

- **Route:** `BlockedEvidence (path leg: nav_path against a running Castle server)`. The component proof is done: arrival, DHD, Marsh and Moh'katan share component 116 (exit 0); the infirmary actors are on 250 (exit 3).
- **Return contract:** `BlockedDependency (Harset return contract: 1664 completion needs HA-01b; the 1360 fallback covers only carry-in players)`. Castle-only fallbacks are named above.
- **`return_q1`, `return_q2`:** `BlockedDecision (D-CA16)`, then `BlockedEvidence (catalog proof; text-ID claim)`.

## Frozen rows

None. No seed, Rust, template or spawn is frozen by this partial seal.

## Test list

- `s`/`c` packets (module `castle_side_return`): a chain replay for the chosen fallback, with a wrong-origin crossing (a `stargate_crossed 'Castle'` event from world 8 does not fire a `world 57` chain), a repeat login after completion (no re-offer), and relog at the step.
- Route: `nav_inspect data/spaces/castle.nav --probes <arrival, DHD, Marsh, Moh'katan> --quiet` exits 0 (this review's run); the path leg is `nav_path` in the lab.

## End-UAT rows

- Arrival: dial Castle from the Harset gate; confirm the position is the row (761.7, 63.5, 551.7) and the DHD area is walkable from it. `UATPending (end UAT, D-CA11)`.
- Return trigger: after the Harset marker mission completes, dial to Castle and confirm the return step offers. `UATPending (end UAT, D-CA11)`.
