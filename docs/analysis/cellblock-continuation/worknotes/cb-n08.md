# CB-N08b: Cellblock continuation close-out

> **Type:** Reference (worknote)
> **Audience:** The campaign reviewer (CB-N08-R), FZ-99, and the owner running the lab rows
> **Last updated:** 2026-10-17
> **Companions:** [README.md](../README.md) (packet status, FZ-99 handoff), [cb-n08-acceptance.md](cb-n08-acceptance.md) (row-by-row state and test inventory), [owner-lab-checklist.md](../owner-lab-checklist.md), [decision-log.md](../decision-log.md), [unified-uat.md § Castle Cellblock tutorial](../../../guides/unified-uat.md#castle-cellblock-tutorial)

Branch `content/cellblock-campaign`, integrated through W5-INT `3a6b3cc64`. Docs only: no lab, no server, no build, no test run in this packet.

## Tested versus never tested

**Never tested in a client.** The owner has run no lab row (O14-09). Every client-dependent row, every CA-U row (except CA-U21, not applicable) and every route-baseline row is `UNTESTED: owner lab not run`. The 2026-10-17 CB-U08 agent runs are invalid (two servers on auth port 13001) and count as nothing. No pacing number exists (O14-14).

**Tested by server-side tests only.** 72 new tests in 19 new files (47 `live_db_`, 25 without a database) plus 7 extended files prove that the seeds, chains, dialogs, catalog rows, cooked-sync bytes and grants are what the seals say. They never show what the client drew or whether it survived the dialog. Per-row test names are in [cb-n08-acceptance.md](cb-n08-acceptance.md). The tests were not re-run by this packet; their pass state is the lane results in the wave reports and the coming CB-N08-CI run.

**Not covered by anything.** Death before and after key transitions has no owner row and no test. Only archetypes 1 (Human) and 8 (Loyalist Jaffa) are exercised on the new missions. [#1667](https://github.com/SandboxServers/Cimmeria/issues/1667) (pistol damage after pickup) is open; one 2026-10-15 run did not reproduce it, and CB-U02 decides.

## What the campaign added

| Packet | Content | Claimed IDs | Owner row |
|---|---|---|---|
| CB-N02a-d | Login restores for Prisoner 329's dialog, the cell-door panel, the Ambernol vial and the retrieval drone | Chains 1026, 1027, 1028, 1036, 1037 | CB-U03 |
| CB-N03t | Static test for the other three cover guards (no seed change) | none | CB-U05 |
| CB-N04a | Two stationary ambient cast rows, 520 and 521 (the Op-CORE prisoners), and a gated once-only bark | Spawns 520 and 521; holder dialog 60160; screen 200160; chain 11010 only | CB-U08 |
| CB-N05 / N05a | *The Remaining Pods*: itemless, zero-reward optional mission, served catalog entry | Mission 62050 (nested 62051-62056); dialog 60150; screen 96110; chains 11000-11005 | CB-U09, CB-U10 |
| CB-N06 | *Before the Patrol Returns*: one exclusive choice, two Health Slappacks (item 2893) once. New Cimmeria content | Mission 62060-62063; dialog 60151; screen 96111; chains 11030-11041 | CB-U12 |
| CB-N07a | One gated, once-only NID Guard alarm bark in the barracks; no spawn, aggression, threat or time added | Chain 11020; screen 200170 | CB-U11 |

Omitted or held: CB-N07b (difficulty tier) is `Done / omitted` (D-CC19). CB-N03a was never sealed (CB-N03s found no seed fault). CB-N05b is `BlockedDependency` on Harset HA-1360 and sits outside the PR.

## Provenance

CB-N05 and CB-N06 are Cimmeria's own content, labelled `CIMMERIA_AUTHORING` in the seeds and docs (CB-D09, D-CC18). CB-N06 is not 2009 content and the docs say so wherever it appears. The owner can veto either at the lab rows.

## Rollbacks

| Change | Rollback |
|---|---|
| Mission 62050 or dialog 60150 crashes or blanks the client | A fix packet moves 60150 into the dialog quarantine; delete the client's `Cache.en-US\CookedDataDialogs.pak`. Drop 62050 from `MISSION_ADDITIONS` to remove the beat. |
| Mission 62060 or dialog 60151 | A fix packet sets `enabled = false` on chains 11030-11041, quarantines 60151 and drops 62060 from `MISSION_ADDITIONS`; delete the client's dialog cache pak. CB-N06 then closes `Done / omitted`. |
| Barracks bark | `enabled = false` on chain 11020; CB-N07 closes `Done / omitted`. |
| Ambient cast bark | `enabled = false` on chain 11010, the only ambient chain seeded. |

## Gaps for the owner and FZ-99

- Run CB-U03, U05, U08-U12 from [owner-lab-checklist.md](../owner-lab-checklist.md) and record each result in its README row; until then every row keeps `Integrated, UATPending`.
- A blank dialog, client crash, missing first-press response, lost progress or doubled reward on CB-U10 or CB-U12 blocks the campaign PR. A slow optional beat does not by itself.
- FZ-99 takes the [FZ-99 handoff](../README.md#fz-99-handoff) into the shared status docs. This packet did not touch them.
- `docs/readme.md` still has the Cellblock row that says "first authored mission design passed review, while fresh client acceptance remains pending". That row is left to FZ-99 with the other shared status rows; the campaign did not change it.
- Follow-up, not done here: `crates/content-engine/tests/it/interact_tag_linter.rs` is 715 lines (713 on main), over the 700-line hard cap. The campaign added 2 lines against the spec allowance of +1. Split the table out in a later cleanup and add no more lines to it.

## Doc-update map rows touched

Content chains (README, decision log, this note), complete authored mission catalog entries (`mission-pak-overrides.md`), UAT steps (`unified-uat.md` CB1-CB7, the owner checklist) and project memory (`project_cellblock_continuation_closeout.md`). `docs/content/mission-chains.md` was touched: the CB-N04a, CB-N05, CB-N06 and CB-N07a sections were appended, and the new sections gained Confidence lines in CB-N08-R1. `docs/content/README.md` was checked and not changed (see the packet report).
