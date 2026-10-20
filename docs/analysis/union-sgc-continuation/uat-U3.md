# UAT U3: the Omega arrival from the 1655 crossing (C4)

> **Type:** How-to (owner-run UAT steps)
> **Audience:** The owner (or a tester the owner names), at the game client
> **Last updated:** 2026-10-19 (US-99)
> **Status:** Not run. No step below has been run in a client.
> **Companions:** [README.md](README.md) (ledger and release readiness), [uat-U1.md](uat-U1.md), [uat-U2.md](uat-U2.md), [crossings.md](crossings.md) (row C4), [packet US-32](../union-zone-planning/sgc-packets/US-32.md)

This is the owner's added check for the SGC to Omega crossing. You verify where a player lands in Omega_Site, which way they face, and that Woolsey and the letter behave per race. Agents do not run this: no lab use by an agent.

The published Omega pin is x 34.70, y -51.28, z -142.00, heading 3.1409. **The heading is not carried:** the `cross_world_teleport` action has no heading field, so only the position reaches the engine. The facing you see on arrival is whatever the engine gives a teleport without a heading. You record it; you do not expect 3.1409.

## Preconditions

- A local server built from `union-sgc/sgc-final`, never the colo release, with the Omega arrival packets merged.
- A Human on 1655 step 4944 (Harriman shows "!"), reached through [U2](uat-U2.md) steps U2-08 to U2-13. A second character, a Free Jaffa on step 4944, for U3-05.
- Know the pin before you start: x 34.70, y -51.28, z -142.00.

## Steps

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U3-01 | Human on 1655 step 4944, holding letter 6786 | Press Harriman and close 5857. | The gate dials and about 3 s later you load into Omega_Site. Evidence: `Content: cross-world teleporting entity` with `world = Omega_Site` and position (34.70, -51.28, -142.00); 1655 on step 4940 | PASS if you arrive in Omega |
| U3-02 | Just arrived | Read your position (the client position readout or the GM position command you normally use) and compare it with the pin. | You stand on the gate plaza at the pin, within about 1 m of (34.70, -51.28, -142.00), on the floor. Evidence: `server_entity_get` on the player | PASS if the position matches and you are not in a wall or in the air. FAIL otherwise, with the numbers |
| U3-03 | Just arrived | Look at which way your character faces. | Record the facing (toward the gate, away from it, or sideways). It is **not** expected to be 3.1409, so this is a record line, not a pass line | RECORD. Report as FAIL only if you arrive facing a wall or cannot see the plaza at all |
| U3-04 | A Human in SGC_W1 with 1654 complete and 1655 not yet accepted (the state just before U2-12) | Press Woolsey and press Accept on 5854. | Woolsey's "?" clears at once. Evidence: chain 3157 clears Woolsey's interaction bit (`op ~`, mask 8388608) and lights Hammond | PASS if the "?" is gone right after the accept and stays gone after a relog |
| U3-05 | Human holds 6786 after U2-13; Free Jaffa has no letter after U2-18 | Compare the two bags before each steps through the gate. | The Human holds Letter to Daniel Jackson (item 6786), exactly one. The Free Jaffa holds none. Both then arrive on step 4940. Evidence: one grant row for 6786 (chain 3159) and none from chain 3167 or 3168 | PASS if the split holds. FAIL if the Free Jaffa has the letter or the Human lacks it |
| U3-06 | Free Jaffa on step 4944 | Press Harriman, close 5857, and read the arrival as in U3-02 and U3-03. | You arrive at the same pin on step 4940 | PASS if the position matches |

## Pass line for the whole run

PASS if U3-01, U3-02, U3-04, U3-05 and U3-06 pass and U3-03 is recorded. Write the arrival facing as a line in the result so a later packet can decide whether the engine should carry a heading.

## Result

Fill in under each step: PASS, FAIL, RECORD or NOT RUN, the evidence you read and a screenshot path where you took one. Record the server build sha and the player ids. Then update the US-U3 row in [README.md](README.md).
