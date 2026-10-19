# US-U2: Lab UAT, the 1651 round trip, 1654, and 1655 for both races

> **Type:** How-to (lab UAT packet) · **Wave:** S4 · **Status:** UATPending (needs US-33 and US-34 integrated, the Dakara and Omega crossings, and the user's yes before any lab use)
> **Index:** [sgc-work-packets.md](../sgc-work-packets.md)

| Field | Value |
|---|---|
| Who runs it | The coordinator, with the `lab-uat` skill. Driving goes to fresh `lab-driver` agents (Haiku), one per run below. Grading is the coordinator's own work. |
| Size | three driver runs, each under ~100k tokens |
| Depends on | US-30 to US-34 merged; Dakara DK-30 (the 1651 offer and C2) and DK-33 (C3) merged; Omega's C4 receiver (UO-00's pin) merged, or the Omega rows are graded on the teleport log row alone |
| Writes | the result table in `docs/analysis/union-sgc-continuation/uat-U2.md` (CRLF), filled by the coordinator |

## Rules

1. **Ask the user and wait for a yes** before `lab_lease_acquire`, a client start, or pointing the lab at a server.
2. The lab runs against a local server built from the campaign branch with the Dakara and Omega packets it needs, never the colo release. A DB reload deletes the lab characters: create fresh ones.
3. A `lab-driver` brief carries the lease id, the standing autonomy, the gotcha sheet from the `lab-uat` skill § 4, the steps of **one run**, and the report fields. No file edits, no builds.
4. Grade every row from server evidence (`server_log_tail`, `server_entity_get`, read-only `server_db_query`, packet tap, SigNoz), never from the driver's summary.
5. Teleport only with `.gotolocation <World> x y z`. Close every dialog before a chat command or a jump.
6. A FAIL becomes a fix packet in the ledger.

## Setup (coordinator)

- **Human:** the US-U1 Soldier, standing on Dakara with 1570 done, or a fresh Soldier played to that point. A Human who has not reached Dakara cannot be offered 1651.
- **Free Jaffa:** a new Free Jaffa (archetype 7) with a 3+ letter name, played on Dakara to the 1654 hand-off (DK-33). If `lab_create_character` cannot make that race, record the row NOT RUN and say why.
- Start a packet tap. SigNoz filter: `service.name = cimmeria-cell AND chain_id >= 3133 AND chain_id <= 3175`.

## Rows

### Run A: the 1651 round trip (Human; C2, the generator, C2b)

| Row | Driver steps | Player sees | Server evidence |
|---|---|---|---|
| R1 | On Dakara, take 1651 and trigger its departure (DK-30's flow). | the player loads into SGC_W1 on the Gate Room floor in front of gate 27, not 2.4 m above it | `Content: cross-world teleporting entity` with `world = SGC_W1`, position P13 (172.79, -10.9, 12.0); 1651 step 4928 active |
| R2 | Look at Hammond. | `SGCW1_GenHammond` shows "!" | chain 3133 on `player_loaded` |
| R3 | Press Hammond; page through 5844 (ten screens, no buttons) and close it. | Carter walks to (or appears at) P08 (164.50, -10.80, 18.00) with "!"; Hammond's "!" clears | chain 3134 then 3135; step 4931 active |
| R4 | **Relog** while on 4931. | Carter stands at P08 with "!" | chain 3139 |
| R5 | Press `SGC_W1_SamCarter`; on 5845 press the item button. | the Naquadah Generator (item 6784) is in the bag; Carter's "!" clears; Harriman shows "!" | chain 3136, 3137; `complete_objective 1651 '5956'`; 1651 still ACTIVE (F13); one item-grant row for 6784 |
| R6 | Press `SGC_W1_Harriman` at P01. | the gate dials (10047, then 10060); 3 s after the press the player loads into Dakara_E1 at the gate | chain 3138: `advance_step '4929'`, both sequences, `advance_step '5171'`, `Content: cross-world teleporting entity` with `world = Dakara_E1`, (96.87, -16.75, 243.23); 1651 step 5171 active; item 6784 still held (`server_db_query` of the inventory) |
| R7 | On a second pass (re-stage R5 with a fresh Human, or skip if none), press Harriman again inside the 3 s window. | Record what the client does: the known F4 risk is an early load into Dakara_E1, because the recovery chain 3142 matches on 4929 | `fire_chain_by_id` on 3142 inside the window and the count of `Content: cross-world teleporting entity` rows. Expected 2 (US-30's characterisation test). GRADE: a client crash, a lost item 6784, or 1651 not on 5171 after the load is a FAIL; a double teleport row with a clean arrival is a PASS-with-note for the movement advisor |

### Run B: 1654 and 1655 for the Human (C3 and C4)

| Row | Driver steps | Player sees | Server evidence |
|---|---|---|---|
| R8 | On Dakara, reach the 1654 hand-off (DK-33). | the player loads into SGC_W1 at P13 | DK-33 completes O5961 (`complete_objective 1654 '5961'`), then the teleport row; 1654 still ACTIVE on arrival with O5962 open (the C3 contract) |
| R9 | Look at Hammond. | turn-in marker on Hammond | chain 3148 |
| R10 | Press Hammond; close 5852. | 1654 completes; Woolsey appears at P02 (197.5, 1.311, 43.6) with "?" | chains 3149, 3151, 3152; `final required objective completed the mission` 1654; spawn INFO for `SGC_W1_Woolsey` |
| R11 | **Relog.** | exactly one Woolsey, still with "?" | the relog rebuilds the per-player instance without her, so chain 3152 spawns her again on `player_loaded`; exactly one `SGC_W1_Woolsey` in `server_entity_query` (a second press of the same state must not add a second one) |
| R12 | Press `SGC_W1_Woolsey`; press Accept on 5854. | 1655 accepted; Hammond shows "!" | chains 3156, 3157; step 4943 active |
| R13 | Press Hammond; close 5856. | Letter to Daniel Jackson (item 6786) in the bag; Harriman shows "!" | chains 3158, 3159; item-grant row for 6786; step 4944 active |
| R14 | Press Harriman; close 5857. | the gate dials; 3 s later the player loads into Omega_Site | chains 3160, 3161; `Content: cross-world teleporting entity` with `world = Omega_Site` at Omega's published pin; 1655 step 4940 active |

### Run C: the Free Jaffa (no tutorial, no letter)

| Row | Driver steps | Player sees | Server evidence |
|---|---|---|---|
| R15 | On Dakara, reach the 1654 hand-off as the Free Jaffa. | the player loads into SGC_W1 at P13 and sees **no** story Jaffa anywhere in the SGC | chain 3172 on `player_loaded` (ten despawns or DEBUG not-found rows); no mission 1563-1571 in the character's missions (`server_db_query`) |
| R16 | Press Hammond; close 5853. | 1654 completes; Woolsey appears with "?" | chains 3150, 3151, 3152 |
| R17 | Press Woolsey. | 5855 shows and 1655 is accepted on that press; Hammond shows "!" | chain 3166; no `Dialog(5854)` |
| R18 | Press Hammond; close 5875. | Harriman shows "!"; **no** letter in the bag | chains 3167, 3168; no item-grant row |
| R19 | Still on 4944, press `SGC_W1_SamCarter`, `SGC_W1_Tealc` and Hammond once each, closing any dialog. | no Human tutorial offer appears | no `Content: accepting mission` for 1563-1571 |
| R20 | Press Harriman; close 5857. | the gate dials; the player loads into Omega_Site | chains 3160, 3161; `world = Omega_Site`; 1655 step 4940 active |

## Stop conditions (from the index)

Mark the run FAIL and stop if:

- the 1651 generator or its state is lost on the round trip (R5-R6: item 6784 missing on Dakara, or 1651 not on 5171);
- a Free Jaffa gains Human tutorial credit (R15, R19);
- Omega cannot tell the two 1655 recipients apart (R13 vs R18: one holds 6786, the other does not, and both arrive on 4940);
- any WARN fires from chains 3133-3175 during a row.

## Result

The coordinator writes `uat-U2.md`: one line per row (PASS, FAIL or NOT RUN, the evidence it read, the screenshot path when there is one), the server build sha, both characters' player ids, and what was never tested. Release the lease. Then update the US-U2 row in the ledger README.

## Return

The coordinator records the result; there is no agent return line for this packet.
