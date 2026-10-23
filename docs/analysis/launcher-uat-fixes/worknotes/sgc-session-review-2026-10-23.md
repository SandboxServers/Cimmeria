# SGC player-session review, 2026-10-23

> **Type:** Reference · **Audience:** Content engineers and playtest reviewers · **Updated:** 2026-10-23
> **Companions:** [Campaign ledger](../README.md), [phase-timing design](../design-phase-timing.md), [telemetry operations](../../../operations/telemetry.md)

## Scope and evidence

This anonymized review describes one observed Windows player session using
launcher 2026.10.2304. Times below are October 23, America/Chicago. Server
mission/combat/lifecycle events, client performance reports and 14 player
bookmarks provide different kinds of evidence. Bookmarks describe the player's
experience; they do not prove the cause. No raw telemetry or identifying
account, machine, install or session values are retained here.

The player completed three introductory missions and progressed into Locate
Col. Carter. This is a partial SGC playthrough, not evidence of full map
completion, first-ever completion or a completed UAT checklist. No gameplay
changes or live reproductions were performed for this review.

## Observed progression

| Local time | Server-observed event | Interpretation |
|---|---|---|
| 11:56:48 | Entered the world | Start of the observed world session |
| 11:58:53 | Completed Welcome to Stargate Command (1559) and Secure a Firearm (1571) | Both introductory completions reached the server |
| 11:58:53 | Dialog 5358 was replaced by 5882 after approximately 15 ms | A concrete dialog-overlap investigation target; not proof of what remained visible |
| 12:00:45.616 | Livewire result recorded as victory; Security Office (1561) advanced to step 4623 | Confirms a win, not that the selected difficulty behaved correctly |
| 12:01:09 | Completed Security Office (1561) | Third recorded completion |
| 12:01:12–12:01:17 | Accepted Locate Col. Carter (1562); advanced step 4624 to 4625 | Carter progression started and remained unfinished |
| 12:02:10 | Died to SGC Ba'al Jaffa, ability 592 | Combat death is confirmed |
| 12:02:12 | Revived using respawner 0 at the starting/Marsh-office area | Mission 1562 remained on step 4625 |
| 12:02:23 | Player reported being unable to leave after revival | The blockage is a player report; navigation/interactions need reproduction |
| 12:02:29 | Normal exit | Approximately 361 seconds in the world; no crash established |

## Measured delays and coverage gaps

| Measurement | Observed value | Limit |
|---|---|---|
| Adoption confirmation to completion | 11:07:57.177–11:37:21.562: 29 minutes 24.385 seconds; 31,605 files, including 25,619 extras | Existing events do not time individual scans, hashing, lock waits or application stages |
| Admitted Play to game process started | 11:55:26.847–11:55:30.110: 3.263 seconds | Excludes pre-admission work, login and world loading |
| Telemetry DLL attach to inline hooks complete | 7.163 seconds | Separate client initialization interval; do not add it to launcher process-start timing |
| Full data resynchronization | 11:56:28–11:58:44: 135.845 seconds; 16,287,216 bytes in eight categories | World entry occurred during this work, so this was mostly background synchronization |
| Dominant resynchronization categories | Dialogs 42.067 seconds; items 46.972 seconds | Durations identify expensive categories, not disk, network or server causality |
| Client performance reports | 41 hitches; 11 at least one second; maximum 4.739 seconds | Correlated with level streaming; cannot assign every reported delay to one cause |
| Startup Lua errors | 12 received rows clustered at 11:56:45–11:56:48 on the client clock | A governed stream can coalesce or suppress events; 12 rows is not a total underlying error count |

The Lua rows include a C stack overflow at `UnitFrames.lua:268`, eight foreign
exceptions at `UnitFrames.lua:267`, one foreign exception in `Squad.lua`, and
two `Minimap.lua:117` failures from `getMapMissionBlips` reporting no player.
These coincide with initial world loading; investigate startup ordering and
recursive unit-frame updates without assuming they explain later gameplay.

The final telemetry health report records zero ring drops, zero upload drops,
zero upload failures and HTTP status 200. In governed mode it reports 8,381
forwarded events and 482,041 rolled-up events. This supports successful delivery
of the governed stream, not capture of every underlying event.

The server's session-end snapshot reports smoothed round-trip time of 153 ms,
227 retransmissions, 43 transmit holes and zero outstanding transmissions. The
disconnect was normal. These cumulative counters have no traffic denominator
here, so they are not a packet-loss rate or proof that networking caused lag.

Coarse adoption and Play rows were available under `client.native` with
`client.launcher.*` targets. The earlier absence of launcher-summary rows refers
to the separate summary stream; it does not mean launcher events were absent.

The client-patches boot report says `nothing_installed` with a fingerprint
mismatch. Confirm the executable build and expected patch fingerprint before
attributing any gameplay or performance symptom to it. The report establishes
a refusal, not its root cause or impact.

## Player reports and follow-up priorities

Priorities are investigation order, not confirmed bug severity. Reproduce each
report with server and client evidence before changing behavior.

| Priority | Player observation, paraphrased | Follow-up |
|---|---|---|
| P1 | After dying, revival returned the player to the Marsh-office area and they could not escape | Reproduce death during Carter step 4625; check respawn destination, door/elevator state and available interaction targets together |
| P1 | The game felt more laggy at 11:58:14; the player also considered their own machine | Compare hitches with streaming and overlapping data resync; collect phase timings on a future launcher build |
| P1 | Main-quest markers remained on looted corpses at two locations | Check corpse loot completion, mission-marker lifetime and client refresh after loot |
| P1 | Startup client logs contain unit-frame stack overflow and related Lua exceptions | Reproduce initial world entry; check recursive callbacks and whether player-dependent UI initialization runs before a player exists |
| P2 | Elevator panel lacked an interaction cue; doors and the usable target were confusing | Inspect the panel's first-interaction feedback and target placement on both sides of the doors |
| P2 | Disable tutorials checkbox appeared ineffective | Reproduce saving/reopening the preference and the next tutorial trigger |
| P2 | An enemy icon jittered | Correlate entity position updates, UI marker refresh and client hitches |
| P2 | Livewire felt like the highest difficulty | Current seed chain 3023 calls `start_minigame` without a difficulty parameter. Inspect executor defaults and the actual request before comparing client presentation; the recorded victory does not establish difficulty correctness |
| P2 | Medkits were needed | Review healing availability and combat outcomes separately from the Livewire minigame; one attempt does not establish balance |
| P3 | Jaffa should emerge from the gate; an airman should walk toward the elevator | Confirm intended staging against authored content before treating these requests as missing behavior |
| P3 | Many quests appeared in the log | Review the visible mission set at each introductory step and its presentation |

The two corpse-marker reports and the two elevator reports account for separate
bookmarks within the 14 observations. The dialog replacement is server evidence
in addition to those bookmarks and merits a focused conversation-flow check.

## Next evidence to collect

Use the [native phase-timing contract](../../../../crates/launcher/desktop/docs/launcher-telemetry.md)
to separate adoption scans and verification from lock waits, and Play admission
from setup, previous-session settlement and helper handshake. Existing historical
rows cannot reconstruct those missing phase durations. Keep any session-specific
query values outside this public report. Gameplay fixes, live reproduction and
a new launcher release are outside this telemetry change and were not performed.
