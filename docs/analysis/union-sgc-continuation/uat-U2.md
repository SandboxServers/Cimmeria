# UAT U2: the 1651 round trip, 1654, and 1655 for both races

> **Type:** How-to (owner-run UAT steps)
> **Audience:** The owner (or a tester the owner names), at the game client
> **Last updated:** 2026-10-19 (US-99)
> **Status:** Not run. No step below has been run in a client.
> **Companions:** [README.md](README.md) (ledger and release readiness), [uat-U1.md](uat-U1.md), [uat-U3.md](uat-U3.md), [packet US-U2](../union-zone-planning/sgc-packets/US-U2.md), [unified-uat.md § SGC_W1](../../guides/unified-uat.md#sgc_w1-human-union-start), [crossings.md](crossings.md)

You play a Human through the return to SGC_W1 from Dakara (mission 1651) and back, then 1654 and 1655 as a Human, and 1654 and 1655 as a Free Jaffa. Agents do not run this: no lab lease, no driver, no client start by an agent.

## Preconditions (all steps)

- A local server built from `union-sgc/sgc-final` **with the Dakara packets that send players back** (the 1651 offer and its departure, and the 1654 hand-off), never the colo release. Without them the arrivals from Dakara cannot be staged by play; see [Known limitations](README.md#known-limitations-and-release-notes) for the gate 27 dependency. A database reload deletes your characters.
- **Human:** the U1 Soldier standing on Dakara with 1570 done, or a fresh Soldier played to that point. A Human who has not reached Dakara is never offered 1651.
- **Free Jaffa:** a new Free Jaffa (archetype 7) with a name of three or more letters, played on Dakara to the 1654 hand-off. If you cannot create that race, record the Free Jaffa steps NOT RUN and say why.
- Teleport only with `.gotolocation <World> x y z`. Close every dialog before a chat command or a jump.
- SigNoz filter: `service.name = cimmeria-cell AND chain_id >= 3133 AND chain_id <= 3175`. No WARN from those chains may fire during any step.

## Run A: the 1651 round trip (Human)

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U2-01 | Human on Dakara with 1651 offered | Take 1651 and trigger its departure (the Dakara hand-off). | You load into SGC_W1 on the Gate Room floor in front of the gate, not 2.4 m above it. Evidence: `Content: cross-world teleporting entity` with `world = SGC_W1`, position (167.00, -10.80, 18.00); 1651 on step 4928 | PASS if you land on the floor. Record where you stand and which way you face (heading is not carried, see U3) |
| U2-02 | Just arrived | Look at General Hammond. | `SGCW1_GenHammond` shows "!". Evidence: chain 3133 on `player_loaded` | PASS if the marker shows without a press |
| U2-03 | Hammond "!" | Press Hammond; page through the briefing (5844, ten screens, no buttons) and close it. | Carter appears at (164.50, -10.80, 18.00) with "!", and Hammond's "!" clears. Evidence: chains 3134 then 3135; step 4931 | PASS if Carter shows "!" |
| U2-04 | On step 4931 | **Relog.** | Carter stands in the Gate Room with "!". Evidence: chain 3139 | PASS if she is there after the relog |
| U2-05 | Carter "!" | Press Carter; on 5845 press the item button. | The Naquadah Generator (item 6784) is in the bag, Carter's "!" clears and Harriman shows "!". 1651 stays active. Evidence: chains 3136 and 3137; `complete_objective 1651 '5956'`; one item-grant row for 6784 | PASS if the generator is in the bag exactly once |
| U2-06 | Harriman "!" | Press `SGC_W1_Harriman`. | The gate dials (10047, then 10060). About 3 s after the press you load into Dakara_E1 at the gate. Evidence: chain 3138, `advance_step '4929'`, both sequences, `advance_step '5171'`, teleport to (96.87, -16.75, 243.23); 1651 on 5171; item 6784 still held | PASS if you arrive with 1651 on 5171 and the generator in the bag. FAIL if the generator is lost |
| U2-07 | A second fresh Human staged to U2-05 (skip if none) | Press Harriman a second time inside the 3 s window. | Record what the client does. The known risk is an early load into Dakara_E1, because recovery chain 3142 matches on 4929. Evidence: `fire_chain_by_id` on 3142 inside the window; expect two `cross-world teleporting entity` rows | PASS-with-note if two teleport rows end in a clean arrival. FAIL if the client crashes, item 6784 is lost, or 1651 is not on 5171 after the load |

## Run B: 1654 and 1655 for the Human

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U2-08 | Human on Dakara at the 1654 hand-off | Reach the hand-off and let it send you. | You load into SGC_W1 at (167.00, -10.80, 18.00), with 1654 still active and one objective open. Evidence: `complete_objective 1654 '5961'`, then the teleport row | PASS if you arrive and 1654 is still active |
| U2-09 | Just arrived | Look at Hammond. | A turn-in marker shows on Hammond. Evidence: chain 3148 | PASS if it shows |
| U2-10 | Hammond marker | Press Hammond and close 5852. | 1654 completes. Woolsey appears at about (197.5, 1.311, 43.6) with "?". Evidence: chains 3149, 3151, 3152; spawn INFO for `SGC_W1_Woolsey` | PASS if Woolsey appears with "?" |
| U2-11 | Woolsey "?" | **Relog.** | Exactly one Woolsey, still with "?". Evidence: chain 3152 respawns her on `player_loaded`; one `SGC_W1_Woolsey` in `server_entity_query` | PASS if there is one Woolsey. FAIL if there are two |
| U2-12 | Woolsey "?" | Press Woolsey; press Accept on 5854. | 1655 is accepted. Woolsey's "?" clears and Hammond shows "!". Evidence: chains 3156 and 3157; step 4943 | PASS if Woolsey's marker clears on the accept |
| U2-13 | Hammond "!" | Press Hammond and close 5856. | The Letter to Daniel Jackson (item 6786) is in the bag and Harriman shows "!". Evidence: chains 3158 and 3159; one item-grant row for 6786; step 4944 | PASS if the letter is in the bag exactly once |
| U2-14 | Harriman "!" | Press Harriman and close 5857. | The gate dials and 3 s later you load into Omega_Site. Evidence: chains 3160 and 3161; teleport to (34.70, -51.28, -142.00); 1655 on step 4940 | PASS if you arrive in Omega. Check position and facing in [U3](uat-U3.md) |

## Run C: the Free Jaffa (no tutorial, no letter)

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U2-15 | Free Jaffa on Dakara at the 1654 hand-off | Reach the hand-off and let it send you. | You load into SGC_W1 at the same arrival point and see **no** story Jaffa anywhere in the SGC. Evidence: chain 3172 on `player_loaded`; no mission 1563 to 1571 in the character's missions | PASS if the SGC is empty of story Jaffa and you hold no tutorial missions |
| U2-16 | Just arrived | Press Hammond and close 5853. | 1654 completes and Woolsey appears with "?". Evidence: chains 3150, 3151, 3152 | PASS if Woolsey appears |
| U2-17 | Woolsey "?" | Press Woolsey. | Dialog 5855 shows and 1655 is accepted on that press. Hammond shows "!". Evidence: chain 3166; no `Dialog(5854)` | PASS if you get 5855, not 5854 |
| U2-18 | Hammond "!" | Press Hammond and close 5875. | Harriman shows "!". **No** letter is in the bag. Evidence: chains 3167 and 3168; no item-grant row | PASS if the bag has no letter. FAIL if item 6786 appears |
| U2-19 | On step 4944 | Press Carter, Teal'c and Hammond once each, closing any dialog. | No Human tutorial mission is offered. Evidence: no `Content: accepting mission` for 1563 to 1571 | PASS if nothing is offered. FAIL if a Free Jaffa gains Human tutorial credit |
| U2-20 | Harriman "!" | Press Harriman and close 5857. | The gate dials and you load into Omega_Site. Evidence: chains 3160 and 3161; 1655 on step 4940 | PASS if you arrive on step 4940 |

## Stop conditions

Mark the run FAIL and stop if:

- the 1651 generator or its state is lost on the round trip (U2-05, U2-06);
- a Free Jaffa gains Human tutorial credit (U2-15, U2-19);
- Omega cannot tell the two 1655 recipients apart (U2-13 against U2-18: one holds item 6786, the other does not, and both arrive on step 4940);
- any WARN fires from chains 3133-3175 during a step.

A FAIL becomes a fix packet in the ledger. Do not fix inside the UAT session.

## Result

Fill in under each step: PASS, FAIL or NOT RUN, the evidence you read and a screenshot path where you took one. Record the server build sha, both characters' player ids and what you did not test. Then update the US-U2 row in [README.md](README.md).
