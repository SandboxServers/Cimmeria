# Dakara_E1 Union work packets

> **Type:** How-to (dispatch contract)
> **Audience:** Union Dakara coordinator, DK campaign coordinator, SGC architect, packet workers and reviewers
> **Last updated:** 2026-10-18 (architect pass against `origin/main` @ `d1bba0400`)
> **Status:** Planning. Nothing dispatched. Wave U1 is gated (see [Waves](#waves)).
> **Companions:** [Dakara campaign](dakara.md), [DK ledger](../dakara-e1-rebuild/README.md), [DK work packets](../dakara-e1-rebuild/work-packets.md), [Free Jaffa route](free-jaffa-route.md), [SGC packets](sgc-work-packets.md), [Union execution contract](execution-contract.md), [packet folder](dakara-packets/)

## Short answer

Almost all of Dakara_E1 already belongs to the **DK campaign** (`docs/analysis/dakara-e1-rebuild/`). It owns every world-61/62 step of missions 1570 and 1645 to 1654, the chain block 8001-8499, the cast and the placements. This file claims only what nobody owns yet:

1. **The crossings contract (U-D04).** One table fixes, for each SGC/Dakara crossing, who departs the player, the mission state at the moment of the teleport, who receives them and where. Packet UD-00 writes it.
2. **The 1651 step 4930 departure.** No ledger names it. The proposal hands it to DK-30 under the contract, as decision D-UD2. Union writes no chain for it.
3. **The Free Jaffa bridge (U-D09).** A short authored intro for the level-1 Free Jaffa that leads into DK's story (UD-01a/b/c). It waits on D-UD1, which settles D-DK1.
4. **A Human exit valve.** A Human who arrives from the SGC through 1570 has no address on Dakara and is stuck there until DK-33. UD-04a is one chain plus its tests. It waits on D-UD3.
5. **Handshake tests** that prove each departure's end state matches the receiving chain's gates (UD-04b/c), plus the joint two-race acceptance run (UD-05) and the close-out (UD-99).

The families UD-02 (peaceful crisis) and UD-03 (defense and climax) in [dakara.md](dakara.md) have **no Union packets**. They map one-to-one onto DK-10 to DK-33, listed in [What DK owns](#what-dk-owns-do-not-touch).

## In-flight Dakara work found

State found on 2026-10-18 by checking local branches, `.claude/worktrees/`, the DK RESUME and `git log origin/main`:

| DK item | State | Evidence |
|---|---|---|
| DK-00 to DK-05 | **Merged** (#1282 to #1288). DK-01 and DK-03 to DK-05 owe UAT M1/M2. | DK README, `git log` |
| DK-06a, DK-07, DK-08, DK-12a (wave 1) | **Started, unshipped.** Local branches `dakara/dk-06a-radio-evidence`, `dakara/dk-07-chain-block` (one commit, `5b35e77f6`), `dakara/dk-08-any-order-ruling`, `dakara/dk-12a-drop-locations`, with worktrees. No PRs. The DK RESUME line "In flight: nothing" is stale. | `git branch`, worktree list |
| DK-06b, DK-09, DK-10a to DK-10e, DK-11, DK-12b/c | Waves 2-3. BlockedDecision on D-DK1 / D-DK17, or BlockedDependency on wave 1. | DK RESUME |
| DK-13 to DK-33, DK-99 | Waves 4-6, stubs. BlockedDecision on D-DK5 (after the M2 playtest). | DK work-packets.md |
| Remote `dakara/dk01-dead-end-fix`, `dakara/dk03-cast-templates` | Stale remotes of merged packets. | `git branch -r` |

None of these is Union work. Union adopts nothing from them and reassigns nothing.

## What DK owns (do not touch)

A Union packet never edits these and never seeds IDs in these ranges:

- `docs/analysis/dakara-e1-rebuild/**`, the test module `chain_replay_tests/dakara_e1_space.rs`, `dakara_e1_tent_travel.rs`, and `crates/cell-world/src/cell/gate_dakara_e1_tests.rs`. Union tests only *run* that last one.
- `db/resources/Content/Seed/dakara_e1_space_chains.sql`, and the planned `dakara_e1_arrival_chains.sql`, `dakara_e1_betrayal_chains.sql` and `dakara_e1_climax_chains.sql`.
- Chains 8001-8499, entity templates 440-479, spawns 8000-8099, point sets 2123-2199, respawners 611-619. Source: `.github/instructions/content-chains.instructions.md` § Dakara_E1 and the DK packets.
- Mission steps owned by DK packets:

| DK packet | Mission steps it owns |
|---|---|
| DK-10a to DK-10e | 1570: 4906, 4904, 4902, 4901, 4905, 4903 |
| DK-11, DK-12 | 1645, 1646 |
| DK-13 to DK-21 | 1647 to 1650 |
| DK-30 | 1651: 4932, 5171 |
| DK-31, DK-32 | 1652, 1653 |
| DK-33 | 1654: 4939, with dialog set 1841 on the Dakara side |

- Owned by SGC packets: US-03 (1570 4641/4642, and 1651 4928/4931/4929 with the generator item 6784) and US-04 (1654 Hammond dialogs 5852/5853, and 1655).

Union packets **build on** DK-01 (chain 8001, the Free Jaffa's Omega grant), DK-10b (the Tau'ri receiving rows 8046-8056 for a Human at 1570/4906), DK-11 (the 1645 offer), DK-30 and DK-33.

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | DK-30's scope reads "1651 Dakara steps 4932, 5171; SGC address grant; SGC steps with the route campaign". Step **4930**, "Gate to the SGC to request support" (objective 5954, tasks 7493/7494), is in neither DK-30 nor US-03. | DK work-packets.md wave 5; `db/resources` 1651 rows | UD-00 (D-UD2) |
| F2 | DK-30's "SGC address grant" cannot work as written. SGC_W1 (58) is instanced and has no DHD, and its gate 27 sits under the floor. The way-back guard `live_db_every_address_a_player_loaded_chain_grants_has_a_way_back` would fail for a grant of gate 27. The departure must be a content `cross_world_teleport`. | `gate_dakara_e1_tests.rs:218`; SGC placement notes | UD-00 (D-UD2) |
| F3 | Under D-DK1 option C, DK-10a to DK-10d drop as a block. But DK-10b holds the **Tau'ri** receiving rows, so plain C also loses the Human's 1570 Dakara steps. Only the Jaffa entry chain 8040 needs to go. | DK `packets/wave-3.md` DK-10b | UD-01a (D-UD1) |
| F4 | A Human who arrives by crossing C1 learns no address on Dakara. The only Dakara address grant (8001) is `archetype eq 7`. DK-U10 requires that a Human *visitor* with no mission gets nothing, so a fix needs a 1570 gate. | chain 8001; DK-U10; `live_db_dakara_arrival_sends_a_visitor_nothing` | UD-04a (D-UD3) |
| F5 | `cross_world_teleport` already logs INFO `Content: cross-world teleporting entity` with entity, account, player, world, position, chain_id and chain_name, and logs `base_channel_closed` when the send fails. SGC scratchpad item H7 ("no telemetry") is wrong. No engine packet is needed. | `executor/transport.rs:166-187` | — (D-UD8) |
| F6 | The rotation of a `cross_world_teleport` is fixed at `[0,0,0]`. The action does no arrival validation and leaves mission rows alone (they are rebuilt on arrival). The departure owner must advance the step **before** the teleport. | `transport.rs` | UD-00 contract |
| F7 | The Free Jaffa starts at level 1 (`char_creation.sql`, CS-02 `start_level`). Lomiada's D-CS12 "Dakara_E1 L3" predates CS-02 (#1273) and U-D09, so it is superseded. | class-start-v6 README lines 83-87 | — (D-UD4) |
| F8 | The world-61 map has no PlayerStart, no stargate actor and no spawn actor, and the merchant tent has no vendor data. Nothing shows ambient life or vendors were intended. | DK `placements/C-landmark-census.md:12` | — (D-UD5, rejected) |
| F9 | The authored-mission precedent (Cellblock 62050 with dialog override 60150) lives only on `content/cellblock-campaign`. PR #1795 closed unmerged on 2026-10-18, and client journal proof is UATPending. `MISSION_ADDITIONS` on main is empty apart from the test fixture 61000-61003. | `crates/resources/src/base/mission_additions.rs` | UD-01b/c gate |
| F11 | `Condition::Archetype` reads a missing archetype as `-1`, so a gate of `archetype lte 6` alone also passes an entity whose archetype was never stamped. DK-10b's Tau'ri rows are written `lte 6`. Chain 63500 adds a `gte 1` floor, and UD-00's handoff tells DK. | `crates/content-engine/src/conditions/mod.rs:333-338` | UD-04a, UD-00 |
| F10 | Block 63500-63799 has no seed tuple head and no doc mention on `origin/main`, `content/harset-campaign`, `content/castle-campaign`, `content/tollana-campaign` or `content/cellblock-campaign`. It is not yet checked against the sibling Union architects' drafts or the stock pak. | scratchpad `idcheck.py` run | UD-00 (D-UD6) |

## Decisions

| ID | Decision | Owner | Status |
|---|---|---|---|
| D-UD1 | Does U-D09 settle D-DK1? Recommendation: **yes, it rules out B.** The Free Jaffa enters through the Union bridge (UD-01). Completing it is what lets DK-11 offer 1645, through a C-shaped gate on "bridge mission completed". DK-10a to DK-10d **keep** their Tau'ri rows; only the Jaffa entry chain 8040 drops. | Owner, with the DK coordinator | **BlockedDecision** |
| D-UD2 | The 1651 step 4930 departure goes to DK-30 under the contract: a world-61 chain gated on step 4930 and archetype runs `advance_step 1651 → 4928`, then `cross_world_teleport` to SGC_W1. No SGC address grant (F2). | Orchestrator + DK coordinator | **BlockedDecision** |
| D-UD3 | The Human exit valve, three options. **A (recommended):** chain 63500 grants Omega (gate 5) to a Human on Dakara_E1 holding 1570 active or completed. **B:** a content return to the SGC. **C:** accept the strand until DK-33. If A, also decide whether Union ships it (UD-04a) or DK absorbs it into DK-10b. | Owner | **BlockedDecision** |
| D-UD4 | Level 1 stands. D-CS12 "L3" is superseded (F7). A non-blocking note for Lomiada. | Architect | Decided |
| D-UD5 | Ambient population, vendors and trainers are rejected for lack of evidence (F8). This also matches DK's non-goals. | Architect | Decided |
| D-UD6 | The Union Dakara ID block is **63500-63799**, sub-blocked below. UD-00 records it after the re-check. | Architect | Decided (record pending) |
| D-UD7 | Crossing C3 (1654): the Dakara chain runs `complete_objective 1654/5961` and never `advance_step`. The SGC completes 5962 at Hammond, which completes the mission (F7 of the engine: the last required objective completes the mission). | Architect | Decided |
| D-UD8 | The crossing telemetry event is the existing `Content: cross-world teleporting entity`. H7 is closed (F5). | Architect | Decided |

**Gate question for the orchestrator (not the owner):** U-D03 holds the Union campaigns until FZ-99. May the docs-only UD-00 run before FZ-99? It writes only new Union files plus one row in `id-allocation.md`.

## ID block 63500-63799 (D-UD6)

| Kind | Range | First use |
|---|---|---|
| Content chains | 63500-63549 | 63500 Human exit valve (UD-04a); 63510-63549 bridge (UD-01c) |
| Missions | 63550-63559 | 63550 bridge mission (UD-01b) |
| Mission steps | 63560-63589 | bridge |
| Objectives | 63590-63619 | bridge |
| Tasks | 63620-63649 | bridge |
| Dialogs | 63650-63669 | bridge |
| Dialog sets | 63670-63679 | bridge |
| Dialog screens | 63680-63729 | bridge |
| Dialog buttons | 63730-63769 | bridge |
| dialog_set_maps | 63770-63799 | bridge |

Mission IDs stay at or below 65535 and outside the stock pak range 38-1826. The block is valid only once UD-00 merges the `id-allocation.md` row.

## Crossings contract (U-D04), draft

UD-00 freezes this table in `docs/analysis/union-dakara-e1-continuation/crossings-contract.md`. Both sides then build against it.

| X | Crossing | Direction | Departure owner | Departure actions, in order | State on arrival | Receiving owner | Arrival point |
|---|---|---|---|---|---|---|---|
| C1 | 1570 | SGC_W1 58 → Dakara_E1 61 | SGC US-03 | `advance_step 1570 → 4642`, `advance_step → 4906`, then `cross_world_teleport Dakara_E1` | 1570/4906 active | DK-10b (Tau'ri rows 8046-8056) | Gate 25 pin (96.87, -16.75, 243.23) |
| C2 | 1651 step 4930 | 61 → 58 | DK-30 (proposed, D-UD2) | `advance_step 1651 → 4928`, then `cross_world_teleport SGC_W1` | 1651/4928 active | SGC US-03 | SGC arrival coordinate (HELD H3, owned by US-03) |
| C2b | 1651 step 4929 | 58 → 61 | SGC US-03 | item 6784 granted at 4931; `advance_step → 5171`, then teleport | 1651/5171 active, item 6784 held | DK-30 | Gate 25 pin |
| C3 | 1654 step 4939 | 61 → 58 | DK-33 | `complete_objective 1654/5961`, then teleport | 1654/4939 active, objective 5962 open | SGC US-04 (dialogs 5852/5853) | SGC arrival coordinate (H3) |
| C4 | 1655 | 58 → Omega 18 | SGC / Omega | — | — | — | Out of Dakara scope |

Contract rules:

1. Guard every departure chain with `step_status` on the departing step, plus `archetype` where the variants differ.
2. Advance the mission **before** the teleport (F6). The receiving side gates on the advanced state.
3. Each crossing names an explicit arrival coordinate. No crossing relies on the gate default.
4. Each departure owner adds a recovery chain: `player_loaded` on the *departure* world while the *receiving* step is active re-runs the teleport. It catches a player stranded by a dropped send or a relog mid-crossing. This is CIMMERIA_RECONSTRUCTION.
5. Who offers what: SGC offers 1570 (after 1569), DK-30 offers 1651, DK-33 offers 1654, SGC offers 1655.
6. The telemetry each crossing must show is `Content: cross-world teleporting entity` with the departing `chain_id` (D-UD8). The receiving side shows its chain's grant or dialog line.

## Packets

| Packet | Role | Size | Deliverable | Status | File |
|---|---|---|---|---|---|
| UD-00 | `mission-systems-advisor`; `movement-teleport-advisor` signs off C1 to C3 | docs, ~30k | Continuation ledger, frozen crossings contract, ID block row, handoff notes to DK | BlockedDependency (U-D03 gate question) | [UD-00](dakara-packets/UD-00.md) |
| UD-01a | `mission-systems-advisor` | docs, ~35k | Bridge design seal: beats, binds, evidence, labels, handoff to DK-11 | BlockedDecision D-UD1 | [UD-01a](dakara-packets/UD-01a.md) |
| UD-01b | `packet-coder` | skeleton | Bridge mission catalog and mission rows | BlockedDependency UD-01a, F9 precedent | [UD-01bc](dakara-packets/UD-01bc.md) |
| UD-01c | `packet-coder` | skeleton | Bridge chains, dialogs and replay tests. Droppable. | BlockedDependency UD-01b | [UD-01bc](dakara-packets/UD-01bc.md) |
| UD-04a | `packet-coder` | ~35k, prescriptive | Human exit valve chain 63500 and its tests | BlockedDecision D-UD3; needs UD-00 | [UD-04a](dakara-packets/UD-04a.md) |
| UD-04b | `packet-coder` | ~30k | C1 handshake live-DB test | BlockedDependency DK-10b, US-03 | [UD-04bc](dakara-packets/UD-04bc.md) |
| UD-04c | `packet-coder` | ~35k | C2/C2b/C3 handshake tests | BlockedDependency DK-30, DK-33, US-03, US-04 | [UD-04bc](dakara-packets/UD-04bc.md) |
| UD-05 | `lab-driver` steps, owner approval | lab UAT | Two-race walk, wrong-race, relog rows UD-U1 to UD-U6 | BlockedDependency UD-04b/c | [UD-05](dakara-packets/UD-05.md) |
| UD-99 | `documentation-writer` | docs, ~25k | Close-out: content docs, unified UAT rows, ledger | BlockedDependency all | [UD-99](dakara-packets/UD-99.md) |

## Waves

| Wave | Packets | Gate |
|---|---|---|
| U1 | UD-00, UD-01a, UD-04a | U-D03 (or the orchestrator's waiver for UD-00); D-UD1 for UD-01a; D-UD3 and UD-00 for UD-04a |
| U2 | UD-01b, UD-01c, UD-04b, UD-04c | UD-01a and F9; DK-10b, DK-30, DK-33 and SGC US-03/US-04 merged |
| U3 | UD-05, UD-99 | U2 merged; owner approves lab use |

Three or four packets per wave, so one `packet-review-fixer` reviews each wave.

## HELD items

| ID | Question | Who answers | Blocks |
|---|---|---|---|
| H-U07 | Rewards for the bridge mission (XP, cash, items). U-D07 is open. | Owner | UD-01b reward columns |
| H-LBL | Label policy for authored player-facing copy (dialog text, journal text) | Owner | UD-01c copy; UD-01a writes placeholders only |
| H1 | Does the SGC show a visible dial before the C1/C2b/C3 teleports? | SGC architect (US-03) | UD-04b/c expectations only |
| H3 | The SGC_W1 arrival coordinate for C2 and C3 | SGC architect (US-03) | C2/C3 rows; UD-04c |
| H5 | What state an SGC instance shows a player who returns through C2 | SGC architect | UD-04c |
| H-RELOG | Where a relog inside the instanced SGC_W1 puts the player | `movement-teleport-advisor` via the SGC architect | Recovery-chain rule 4 on the SGC side |

## Rejected additions

| Addition | Reason |
|---|---|
| Ambient population, vendors or trainers on Dakara_E1 | No evidence (F8, D-UD5) |
| An SGC stargate address for 1651 | No DHD; gate under the floor; the way-back guard fails (F2) |
| A level-3 Free Jaffa start | Superseded (F7, D-UD4) |
| Union chains for any step DK owns | DK's scope; duplicating it contradicts in-flight work |
| A new radio or minigame beat | DK-06 / D-DK10 owns the radio beat |
| Work on Dakara worlds 24, 63, 64 or 65 | Outside this zone; different map problems ([dakara.md](dakara.md)) |
| Showing 1570's SGC steps as done for a Free Jaffa | U-D09 forbids claiming unplayed missions |

## Provenance labels

Every seed row or doc claim a Union packet adds carries one label in a comment or a column:

- **RESTORED_SOURCE**: rows recovered from the client pak or the legacy seeds (missions 1570, 1651, 1654, their steps, objectives and dialogs).
- **CIMMERIA_RECONSTRUCTION**: glue that restores intended behaviour the source implies but does not ship (the crossings, recovery chains, the Human exit valve).
- **CIMMERIA_AUTHORING**: new content with no source (the bridge mission). It lives in its own packet (UD-01b/c) and its own seed file, so it can be dropped whole.

## Stop conditions

Inherited from [dakara.md](dakara.md). Stop the Union Dakara work if:

- a Free Jaffa can complete 1570 without a truthful start;
- the 1651 SGC loop drops mission or generator state;
- one player's betrayal phase changes another player's cast;
- 1654 departs without an SGC receiving state;
- a Union packet needs to edit a DK-owned file.
