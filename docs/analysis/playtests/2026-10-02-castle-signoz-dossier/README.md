# CellBlock and Castle: recent SigNoz sessions against the Lomiada dossiers

> Investigation dated 2026-10-02. All times below are UTC. This is a server-log
> analysis, not a client visual or audio acceptance test.

## Evidence and scope

SigNoz logs were queried for `service.name = cimmeria-server` over the preceding
14 days, then narrowed to the session windows below. The main anchors were
`Gate travel: sending RESET_ENTITIES for world transition`, `player entered world`,
`Mission state persisted`, `fire_*: matched`, `Minigame victory`, and `Sent
stargate onSequence`. Mission status `2` means completed. All Castle arrivals in
this sample report `access_level=2`, so these are privileged playtests, not a
measure of general player completion rates. Repeated `player entered world`
events can be relogs or other transfers; they were not counted as distinct
CellBlock-to-Castle journeys.

The comparison is with `WORLDS/RECOVERED/Castle_CellBlock/` and
`WORLDS/RECOVERED/Castle/` inside `SGW_FINAL_DEVELOPER_HANDOFF.zip`, especially
their `00_WORLD_CONTENT_DOSSIER.md`, `02_MISSIONS.csv`, and
`04_KISMET_EVENTSET_ACTORS.csv`. The existing
[CellBlock](../../castle-cellblock-rebuild/README.md) and
[Castle](../../castle-rebuild/README.md) campaign records provide implementation
context. The dossiers are source indexes; zero rows in a relation table are
not evidence that gameplay cannot run.

The 14-day search found 16 Castle `Gate travel: sending RESET_ENTITIES` records
(3 colo, 13 dev) and three accepted Castle-to-Harset stargate dials, all in dev.
The September 29 colo arrivals include repeated world entries from one
high-level character with only two missions, so those are not evidence of a
fresh story playthrough.

## Session A — September 28, dev: full journey to Harset

| UTC | Observed server event | Dossier comparison |
|---|---|---|
| 08:43:21 | Entered `Castle_CellBlock` with zero missions. | A clean progression anchor. |
| 08:48:02–08:50:57 | Completed 622, 638, 639, 640, 641. The Prisoner 329 door, Ambernol vial, retrieval unit, ring switch, Marsh and Preparation terminal interactions fired; Livewire launched for the door, rings and terminal. | Matches the dossier's 622–641 mission rows and CellBlock ring/door actors. The log proves mission and transport events, not the exact client animation. |
| 09:00:32–09:06:45 | Completed 680–686. This included Mess Hall and five hallway controller missions, with a world re-entry at 09:02:34 after deaths. | The dossier lists 680, 681, 684 and 686, but its moniker-based mission list omits completed 682, 683 and 685. It is not a runtime completeness list. |
| 09:18:51–09:20:27 | Completed 687 and 688; Terminal X and Armory ring switch fired; entered `Castle` with 16 missions. | Confirms the CellBlock-to-Castle handoff. The dossier's separate mission 1654 named “Aftermath” should not be conflated with 687. |
| 09:22:07–09:23:32 | Gerschon dialog 2573 accepted 701; Copplemann dialog and Livewire victory advanced it through 2400, 2401 and 2421; dialog 2576 completed 701 and accepted 702/703. | Matches the Castle narrative beats. The simultaneous acceptance of 702/703 is Cimmeria's documented reconstruction. |
| 09:25:04–09:25:24 | InterrogationBlock region activated both missions; Romney's death completed 703, then Zuritska dialog 2577 completed 702 and started 704. | Both objectives work even when Romney is resolved before Zuritska. The September 29 session below resolves them in the opposite order. |
| 09:26:23–09:28:51 | CommsRoom region, terminal Livewire victory, dialog 2581, ThroneRoom region and access panel completed 704 and 706; started 708. | The dossier lists 704/708 but omits 706. The current Castle seed and this trace both show 706 as a live bridge. |
| 09:28:57–09:38:58 | Access-panel diagnostic (dialog 5004), Muelbach death and crystal acquisition, Marsh dialog 5008, DHD Livewire victory; 708 advanced to 4462 and granted Harset gate address 3. | Supports one of the dossier's two diagnostic routes and the Muelbach crystal route. It does not exercise the surrendering guard, Bravo-officer alternative, or Jaffa report. |
| 09:39:02–09:39:25 | Dial accepted; MakeGate sequence 10145/event 6100 emitted; CrossGate sequence 10158/event 6113 emitted; `stargate_crossed` completed 708; deferred reset followed; `Harset` world entry succeeded. | Extends beyond the Castle workbook's deliberate hard stop at step 4462. The Castle dossier's zero map-linked Kismet rows did not prevent the seeded gate EventSet 10011 from being dispatched. |
| 09:40:41–09:40:43 | Harset CommandCenterTransition region fired and `Harset_CmdCenter` world entry followed. | Corroborates arrival beyond the Castle boundary, but does not validate Harset's 73 mission rows. |

This session took 37 min 6 s from CellBlock entry to Castle entry and 18 min
58 s from Castle entry to Harset entry. It included five logged CellBlock
player deaths and a CellBlock re-entry, yet the mission chain reached 708.
`Content: playing sequence` logged CellBlock Straegis sequence 1751 at
09:06:45, immediately after 686 completed. This aligns with the dossier's
EventSet 747 camera-only `StraegisAttack` map root. It proves dispatch, not
camera playback on the client.

## Session B — September 29, colo: recent Castle arrival

This run entered CellBlock at 18:48:52 with zero missions and reached Castle at
19:05:02 with 16 missions, a 16 min 9 s journey. It completed 622, 638–641,
680–688. Three CellBlock Livewire victories, two ring destination selections,
Straegis sequence 1751 after 686, and the Armory ring exit were logged. The
delayed aftermath dialogs 2516 and 5859 were fired after the sequence. The
player died once in CellBlock and continued.

In Castle, 701 completed at 19:07:40 after a Copplemann Livewire victory;
InterrogationBlock region entry fired at 19:10:56; Zuritska dialog completed
702 at 19:11:14; Romney's death completed 703 at 19:11:32. The CommsRoom region
fired at 19:20:46 while 704 was active. Four guard-caused Castle deaths were
logged between 19:13 and 19:27, with repeated Zuritska interactions. No 704
completion, 706, 708 or Castle gate dial was found for this character through
the queried following hours. This shows a successful arrival and early Castle
mission path; it does not establish a 704 soft-lock or completion of the Castle
arc.

## Successful Castle gate crossings and timing

| Run (UTC) | Accepted dial | MakeGate 6100 | CrossGate 6113 and 708 completion | Harset entry |
|---|---|---|---|---|
| Sep 25, first dev run | 20:23:40 | 20:23:44 | 20:23:51 | 20:23:58 |
| Sep 25, second dev run | 21:06:29 | 21:06:33 | 21:06:41 | 21:06:43 |
| Sep 28 dev run | 09:39:02 | 09:39:02 | 09:39:17 | 09:39:25 |

All three accepted dials target Harset address 3 from the Castle origin
EventSet 10011. The Sep 28 run additionally logs a 1.59 s gap from CrossGate
dispatch to `RESET_ENTITIES`, consistent with the current provisional 1.5 s
crossing hold in
[`crossing_hold_state.rs`](../../../../crates/cell-world/src/cell/space_manager/crossing_hold_state.rs).
Its MakeGate dispatch occurred 0.162 s after dial acceptance, consistent with
the current 100 ms tick-based dial in
[`gate_dial_state.rs`](../../../../crates/cell-world/src/cell/space_manager/gate_dial_state.rs).
The earlier Sep 25 runs used the previous roughly 4 s dial and transitioned
immediately after crossing. The log message still says “gate opens in 4s” on
the Sep 28 build; it is stale relative to the 100 ms implementation.

The `Sent stargate onSequence` logs record a server send with one witness;
they do not prove the rendered vortex, cinematic duration, audio, or chevron
animation. The Sep 28 `stargate region entered with no gate dialled` entries
occur after the accepted crossing consumed the dial and Harset arrival; they
are not evidence of a failed Castle crossing.

## What this changes for restoration

1. **Core path is operational in the observed dev run.** The exact 701–708
   mission state, three Castle Livewire victories, gate address grant, both
   gate sequence dispatches and Harset entry are stronger runtime evidence
   than the dossier's missing Castle Kismet/runtime binding tables.
2. **The dossiers remain useful as source indexes.** CellBlock's exact door,
   ring and Straegis event roots and Castle's mission step/objective text help
   check fidelity. The mission lists are moniker-limited: they omit live
   CellBlock controller missions 682/683/685 and Castle mission 706.
3. **The optional and faction content is still a separate restoration question.**
   The Castle dossier names Human 1520, Jaffa 1521, auxiliary 1376 and 1664;
   the older Castle workbook also calls out optional Romney's Files 567.
   None appears in `db/resources/Content/Seed/castle*.sql`, and these sampled
   archetype-1 sessions did not exercise them. The core route's success does
   not verify those branches or the alternate 708 diagnosis/crystal sources.
4. **Presentation still needs client evidence.** Server logs establish the
   Straegis and gate sequence sends, ring destinations and mission state. A
   short captured client pass should confirm the camera, ring and gate visuals
   and the provisional timing before declaring visual fidelity complete.

No runtime or seed change was made for this investigation.
