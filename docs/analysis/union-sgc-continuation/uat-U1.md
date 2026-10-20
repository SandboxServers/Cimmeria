# UAT U1: the Human path from mission 1562 to the Dakara arrival

> **Type:** How-to (owner-run UAT steps)
> **Audience:** The owner (or a tester the owner names), at the game client
> **Last updated:** 2026-10-19 (US-99)
> **Status:** Not run. No step below has been run in a client. Every claim that a chain "does X" rests on database and executor tests only.
> **Companions:** [README.md](README.md) (ledger and release readiness), [uat-U2.md](uat-U2.md), [uat-U3.md](uat-U3.md), [packet US-U1](../union-zone-planning/sgc-packets/US-U1.md), [unified-uat.md § SGC_W1](../../guides/unified-uat.md#sgc_w1-human-union-start), [placement.md](placement.md)

You play one fresh Human from the end of the SGC_W1 tutorial (mission 1562) through the Gate Room fights and out through the gate to Dakara. Agents do not run this: no lab lease, no driver, no client start by an agent. You run it, and you write the PASS or FAIL line under each step.

## Preconditions (all steps)

- A local server built from `union-sgc/sgc-final` (or the PR that carries it), never the colo release. A database reload deletes your character: create a fresh one afterwards.
- A new Human Soldier (archetype 1) with a name of three or more letters. Note its player id for the evidence queries.
- Teleport only with `.gotolocation SGC_W1 x y z`, never `.gotoxyz`. Close every dialog before you type a chat command.
- "Press X" means stand 3 to 4 m from it, face it and click it. Every first press must show something (the repo's first-press feedback rule); a press that needs two clicks is a FAIL.
- For the evidence column, filter SigNoz with `service.name = cimmeria-cell AND chain_id >= 3051 AND chain_id <= 3199`. No WARN from those chains may fire during any step.

## Run A: 1562 to 1564 (Carter, the virus, the Failsafe)

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U1-01 | Fresh Human, in SGC_W1, 1562 active | Finish 1562 by its existing flow (the lab attackers, then Carter). | 1562 completes. Carter shows "!". Evidence: `mission_completed` 1562, chain 3051 executing | PASS if Carter offers the next mission with no extra click. FAIL otherwise |
| U1-02 | U1-01 done | Press Carter (`SGC_W1_SamCarter`); read and close her dialog. | 1563 is accepted. The terminal near (-49.48, 2.32, 36.38) glows. Evidence: `Content: accepting mission` 1563 | PASS if the terminal glows after the close |
| U1-03 | 1563 active | Press the glowing terminal (a plain interact), then press Carter again. | 1563 completes. 1564 is offered. Evidence: no WARN in chains 3051-3057 | PASS if 1564 is offered after the second Carter press |
| U1-04 | 1564 offered | Accept 1564, then close the radio report that follows (it has no buttons). | 1565 is offered with no further click. Evidence: chain 3085 `accept_mission` 1565 | PASS if the 1565 offer appears on the close alone |
| U1-05 | 1564 on step 4631 | Walk toward the Failsafe room (about (150.0, 1.3, -196.0)) with Carter following. | Carter follows and ends at about (147.0, 1.30, -197.0). See [the escort walk](#the-1564-escort-walk-judge-it) | See that section; the stall is accepted, you judge it |
| U1-06 | In the Failsafe room | Press the Failsafe Device (`SGC_W1_FailsafeDevice`). | The Failsafe step completes and 1564 completes. 1568 and 1569 are both accepted. Evidence: `final required objective completed the mission` 1564 | PASS if both follow-on missions appear |

## Run B: 1565 to 1568 (the route fights and the Gate Room)

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U1-07 | 1565 active | Kill the first two route Jaffa (`SGC_W1_TacticsJaffa1`, `...2`). | 1565 completes and a second pair appears near the sealed points P11 and P12. Evidence: chains 3086 and 3088 | PASS if the second pair appears. See also the lab check "P11/P12 out of sight" |
| U1-08 | 1566 active | Kill the second pair. Then **relog** before you kill the Clear group. | After the relog the five Clear Jaffa still stand and the Tactics pair does not. Evidence: on relog chain 3173 fires and 3175 does not | PASS if the Tactics pair is gone and the five Clear Jaffa remain |
| U1-09 | 1567 active | Kill the five Clear Jaffa (`SGC_W1_ClearJaffa1` to `5`). | 1567 completes. Evidence: chain 3096 | PASS if 1567 completes on the fifth kill |
| U1-10 | 1568 active | Walk into Gate Control (about (173.0, -6.4, 55.0)). | Monologue 5382 plays and the gate terminal glows. Evidence: chain 3101 | PASS if both happen |
| U1-11 | Gate terminal lit | Press `SGC_W1_GateTerminal1`, then kill the three Gate Room Jaffa (`SGC_W1_GroomJaffa1` to `3`). | Dialog 5383 shows, then the glow clears. After the third kill you, Carter and Hammond move to the departure points and dialog 5384 shows. Evidence: chains 3102, 3103, 3105 and `move_entity` INFO rows | PASS if the move and 5384 happen. Aggro of the Gate Room Jaffa on arrival is a lab check below |
| U1-12 | After U1-11 | **Relog.** | The Gate Room is empty and Carter stands at her departure point P16. Evidence: chains 3171 and 3107 on `player_loaded` | PASS if no Jaffa remain and Carter is in the Gate Room |

## Run C: 1569, 1570 and the departure to Dakara

| Step | Precondition | Do | You should see | Pass / fail |
|---|---|---|---|---|
| U1-13 | 1569 active | In the requisitions room press the six storage crates (`SGC_W1_Crate1` to `6`). | Five crates show a find dialog (5385 to 5389). The sixth only flips its marker. 1569 completes on the sixth. Evidence: chains 3110-3122, `final required objective completed the mission` 1569 | PASS if all six respond and 1569 completes. Crate clearance is a lab check below |
| U1-14 | 1568 and 1569 complete | Press `SGCW1_GenHammond` or `SGC_W1_Tealc` (both show "?"); accept on 5392 or 5393. | 1570 is accepted. Evidence: `Content: accepting mission` 1570 | PASS if either NPC offers it |
| U1-15 | 1570 active, Harriman shows "!" | Press `SGC_W1_Harriman`. | The gate dials (sequence 10047, then 10060 2.5 s later). About 3 s after the press you load into Dakara_E1. Evidence in order: `advance_step 1570 '4642'`, sequences 10047 and 10060, `advance_step 1570 '4906'`, `Content: cross-world teleporting entity` with `world = Dakara_E1` and position (96.87, -16.75, 243.23) | PASS if you arrive in Dakara with no client crash |
| U1-16 | Just arrived | Read your world and position. | You stand at the Dakara gate with 1570 on step 4906. Evidence: `server_entity_get` on the player, and the mission row (1570, step 4906) | PASS if both match. If the Dakara receiver is not in your build, record NOT RUN and grade U1-15 on the teleport log row alone |

## The 1564 escort walk (judge it)

Carter follows you on step 4631 over about 260 m, but the SGC_W1 navmesh has no route between her spawn, the Failsafe room and the Gate Room, so her follow can stall at the first gap. The owner accepted that (D-US13 = a) and left it to you to judge.

1. Walk the route in U1-05 at an ordinary pace and watch Carter.
2. Note where she stops following, if she does, and whether she is where the story needs her once you reach the room. The chain that ends the escort teleports her to the device point, so the mission should finish whether she walks or not.
3. Write one line: **Escort PASS** (she follows, or the stall is not noticeable), **Escort ACCEPT** (she stalls but the mission finishes and nothing looks broken), or **Escort FAIL** (the mission cannot finish, or she blocks you or stands somewhere absurd). Say where she stalled and use the player's position.

## Stop conditions

Mark the run FAIL and stop if:

- you cannot leave through an accepted 1570 (U1-14 to U1-16);
- any WARN fires from chains 3051-3131 during a step;
- a press needs two clicks to show anything.

A FAIL becomes a fix packet in the ledger. Do not fix inside the UAT session.

## Placement lab checks owed

These are open map and mesh checks from the placement seal. Nobody has walked them in a client. Each is read as you play, so note it beside the step named.

| Check | Where it shows | What to look for |
|---|---|---|
| Route from P13 to the navmesh groups 42, 3 and 26 (elevator rows 55 and 76) | U1-05, U1-06 | Whether a player can reach the Failsafe room and Carter's lab on foot, and which elevators they need |
| P11 and P12 out of sight | U1-07 | The second Jaffa pair appears where the player cannot see it pop in |
| P01 seated pose | U1-14, U1-15 | Harriman stands or sits sensibly at his console |
| P07 region reach | U1-05, U1-06 | Entering the Failsafe room triggers `SGC_W1.FailsafeRoom` where you expect |
| Crate clearance | U1-13 | All six crates can be pressed from a normal stance and none sits inside a wall |
| P13 and P08 floor height | U1-12, and the arrival in [U2](uat-U2.md) | You stand on the floor, not above or in it |
| Gate-axis strip nature | U1-15 | Whether the strip in front of the gate is walkable floor or something else |
| Gate Room Jaffa 65 to 67 aggro on arrival | U1-10, U1-11 | Whether they attack you the moment you enter, before the monologue |

## Result

Fill in under each step: PASS, FAIL or NOT RUN, the evidence you read, and a screenshot path where you took one. Record the server build sha, the character's player id and what you did not test. Then update the US-U1 row in [README.md](README.md).

## Known limitations

See [README.md § Known limitations and release notes](README.md#known-limitations-and-release-notes); the ones this run meets are the escort stall and the arrival heading.
