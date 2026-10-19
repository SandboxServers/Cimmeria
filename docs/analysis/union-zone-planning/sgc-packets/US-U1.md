# US-U1: Lab UAT, the Human path from 1562 to the Dakara arrival

> **Type:** How-to (lab UAT packet) · **Wave:** S4 · **Status:** UATPending (needs S3 integrated, and the user's yes before any lab use)
> **Index:** [sgc-work-packets.md](../sgc-work-packets.md)

| Field | Value |
|---|---|
| Who runs it | The coordinator, with the `lab-uat` skill. Driving goes to fresh `lab-driver` agents (Haiku), one per run below. Grading is the coordinator's own work. |
| Size | three driver runs, each under ~100k tokens |
| Depends on | every S1-S3 packet merged into the campaign branch; DK-10b (the Dakara receiver of C1) merged, or row R16 is NOT RUN and R15 is graded on the teleport log row alone |
| Writes | the result table in `docs/analysis/union-sgc-continuation/uat-U1.md` (CRLF), filled by the coordinator |

## Rules

1. **Ask the user and wait for a yes** before `lab_lease_acquire`, a client start, or pointing the lab at a server. No agent asks on its own.
2. The lab must run against a local server built from the campaign branch, never the colo release (`.claude/agent-memory/main-session/reference_lab_local_server.md`). A DB reload deletes the lab character: create a fresh one.
3. A `lab-driver` brief carries the lease id, the standing autonomy, the gotcha sheet from the `lab-uat` skill § 4, the steps of **one run**, and the report fields. No file edits, no builds.
4. A driver's summary is a claim. Grade every row from server evidence: `server_log_tail`, `server_entity_get`, read-only `server_db_query`, `server_packet_tap_*`, and SigNoz for anything older than the log tail.
5. Teleport only with `.gotolocation SGC_W1 x y z`, never `.gotoxyz`. Close every dialog before a chat command.
6. A FAIL becomes a fix packet in the ledger; do not fix in the lab session.

## Setup (coordinator)

- Character: a new Human Soldier (archetype 1) with a 3+ letter name, through `lab_ensure_in_world {create: true}`. Record its `player_id` from `server_db_query`.
- Start a packet tap for the session.
- SigNoz base filter: `service.name = cimmeria-cell AND chain_id >= 3051 AND chain_id <= 3199`.

## Rows

Each row: what the driver does, what the player must see, and the evidence the coordinator checks. "Press X" means stand 3-4 m away, face it, click by entity id. Positions are the US-01-sealed values in `placement.md`; the proposals are given here.

### Run A: 1562 to 1564 (Carter, the virus, the Failsafe)

| Row | Driver steps | Player sees | Server evidence |
|---|---|---|---|
| R1 | Finish 1562 by its existing flow (the lab attackers, Carter). | 1562 completes; Carter shows "!" | `mission_completed` 1562; `fire_chain_by_id: executing` on 3051 (US-12 offer) |
| R2 | Press `SGC_W1_SamCarter`; read and close her dialog. | 1563 accepted; the terminal by P04 (-49.48, 2.32, 36.38) glows | `Content: accepting mission` 1563; `set_interaction_type` on `SGC_W1_CarterTerminal` |
| R3 | Press `SGC_W1_CarterTerminal` (D-US7 default: a plain interact). Return to Carter and press her. | 1563 completes on the steps in US-12's report; 1564 is offered | step-advance rows for 1563; `Content: accepting mission` 1564; no WARN in 3051-3057 |
| R4 | Close the zero-button radio report 5375. | 1565 is offered with no further click | chain 3085: `accept_mission` 1565 |
| R5 | Walk to the Failsafe room (P07, about (150.0, 1.3, -196.0)), with Carter escorting on 4631. | Carter follows and stops at P05 (147.0, 1.30, -197.0) | US-16's escort rows; `enter_region 'SGC_W1.FailsafeRoom'`; Carter's position within 1 m of P05 in `server_entity_get` |
| R6 | Press `SGC_W1_FailsafeDevice`. | the Failsafe step completes; 1564 completes | `complete_objective` on 1564; `final required objective completed the mission` 1564; 1568 and 1569 accepted (H1) |

### Run B: 1565 to 1568 (the route fights and the Gate Room)

| Row | Driver steps | Player sees | Server evidence |
|---|---|---|---|
| R7 | Kill `SGC_W1_TacticsJaffa1` and `…2` (rows 57/58). | 1565 completes; a second pair appears near P11/P12 | chain 3086, 3088; spawn executor INFO for `SGC_W1_Tactics2Jaffa1/2` |
| R8 | Kill the second pair. Then **relog** before killing the Clear group. | after relog the five `SGC_W1_ClearJaffa1..5` still stand; the Tactics pair does not | chain 3089; on the relog 3173 fires (Tactics despawned) and 3175 does not |
| R9 | Kill the five Clear Jaffa. | 1567 completes | chain 3096 |
| R10 | Walk into Gate Control (P09, about (173.0, -6.4, 55.0)). | monologue 5382; the gate terminal glows | chain 3101, `Content: displaying dialog` 5382 |
| R11 | Press `SGC_W1_GateTerminal1`; kill `SGC_W1_GroomJaffa1..3`. | 5383; the glow clears; after the third kill the player, Carter and Hammond move to P15, P16, P14 and 5384 shows | chains 3102, 3103, 3105; `move_entity` INFO rows |
| R12 | **Relog.** | the Gate Room is empty; Carter stands at P16 | chains 3171 and 3107 on `player_loaded` |

### Run C: 1569, 1570 and the C1 departure

| Row | Driver steps | Player sees | Server evidence |
|---|---|---|---|
| R13 | Press `SGC_W1_Crate1` … `SGC_W1_Crate6` in the requisitions room (P10a-f). | five crates show 5385-5389; the sixth only flips its marker; 1569 completes | US-23 chains 3110-3122; `final required objective completed the mission` 1569 |
| R14 | Press `SGCW1_GenHammond` (or `SGC_W1_Tealc`), both showing "?". Accept on 5392 (or 5393). | 1570 accepted | chain from US-24; `Content: accepting mission` 1570 |
| R15 | Press `SGC_W1_Harriman` at P01 when he shows "!". | the gate dials (sequence 10047, then 10060 2.5 s later, D-US4 default); 3 s after the press the player loads into Dakara_E1 | C1 rows in order: `advance_step 1570 '4642'`, `Content: playing sequence` 10047 and 10060, `advance_step 1570 '4906'`, `Content: cross-world teleporting entity` with `world = Dakara_E1` and position (96.87, -16.75, 243.23) |
| R16 | Read the player's world and position after the load. | the player stands at the Dakara gate, 1570/4906 active | `server_entity_get` on the player; `server_db_query` of the mission row (1570, step 4906) |

## Stop conditions (from the index)

Mark the run FAIL and stop if:

- the Human cannot leave through an accepted 1570 (R14-R16);
- any WARN fires from chains 3051-3131 during a row;
- a press needs two clicks to show anything (the first-press feedback rule).

## Result

The coordinator writes `uat-U1.md`: one line per row (PASS, FAIL or NOT RUN, the evidence it read, and the screenshot path when there is one), the server build sha, the character's player id, and what was never tested. Release the lease. Then update the US-U1 row in the ledger README.

## Return

The coordinator records the result; there is no agent return line for this packet.
