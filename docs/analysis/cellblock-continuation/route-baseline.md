# Cellblock route baseline

> **Type:** Reference. **Audience:** Cellblock coordinators and UAT reviewers.
> **Last updated:** 2026-10-15. **Companions:** [Ledger](README.md), [CB-N05 design seal](worknotes/cb-n05-seal.md), [UAT spec](../../guides/uat-specs/castle-cellblock.toml).
> Static baseline `d3efb7d41aad0b6008397f112d224ce7430d7acf`, audited 2026-10-14.
> CB-N00 performed no fresh route run, deaths, duration, screenshots, packet tap or lab actions.

## Observations versus current faults

No current runtime fault was reproduced by CB-N00. This does **not** mean no fault
exists. The following observations retain their original scope and date.

| Evidence row | Source and observation | Current disposition |
|---|---|---|
| CB-B01 | [2026-10-02 dossier, Session A](../playtests/2026-10-02-castle-signoz-dossier/README.md): 2026-09-28 dev entry 08:43:21 UTC, Castle entry 09:20:27, 37m06s and five logged Cellblock deaths. Completed 622/638–641/680–688; privileged access_level=2. | T: one historical observation, not target duration, average or normal-player success rate. Fresh timing/death count UATPending. |
| CB-B02 | Same Session A: sequence 1751 dispatch at 09:06:45 after 686. Session B had client event telemetry, not visual/audio acceptance. | T: dispatch does not prove camera/rift/blood/playback or first-press delivery. N01/asset UATPending. |
| CB-B03 | [Lab reliability follow-up](../lab-uat-reliability/README.md), 2026-10-13: corpse click/equip progressed first mission, first guard defeated character; pistol showed 0/15 and no damage. [Issue #1667 comments](https://github.com/SandboxServers/Cimmeria/issues/1667) add a fresh zero-Focus observation with an “out of focus points” warning and a God-mode/full-Focus retest: first shot missed, second did 48 damage and consumed ammo. Walkthrough did not reach Livewire/destination ring. | C as reported, not independently rerun. #1667 is OPEN as of 2026-10-14. The warning does not establish a Focus-cost refusal: ability 592 requires ammo, and current server code has no Focus gate. The full-Focus shot disproves a blanket server damage failure; normal ammo/grant path and visible fire/recoil remain unproven. N02 needs a bounded current reproducer and combat/items ownership. |
| CB-B04 | Same source: respawn at Mess Hall preceded helper observation (#1668); dialog-close helper reported success while dialog remained (#1706); unnamed corpse point hit avatar (#1708). | Harness/product distinction required. Lab campaign owns helper behavior; no content defect inferred from a tool success return. |
| CB-B05 | Current seed 1172 delays dialog 5859 by 26100ms after scene start; 1161 delays 2516 by 10100ms. `mission_686_straegis.rs::live_db_straegis_followup_allows_aftermath_dialog_to_be_read` exists. Rebuild UAT guide cites #1721. | S: old short-gap repair already present; do not open duplicate N02 repair. Real dialog reading/closure remains UATPending. |
| CB-B06 | Current seed 689 chains 1022–1025; all six 681–686 controllers hidden in missions.sql; hidden/backstop/flank tests exist. | S: earlier inert-boots/visible-controller claims cannot establish current defects. No compile or live-DB pass claimed here. |
| CB-B07 | Current 1107 advances to 80688; 1109 completes 688 then cross_world_teleport; `mission_688.rs::live_db_chain_1109_completes_688_and_cross_world_teleports` guards fallback. | S: ring campaign owns replacement. N00 does not prove native ring animation or arrival placement. |

Sources of planned probes:
[rebuild UAT](../castle-cellblock-rebuild/uat-guide.md),
[current spec](../../guides/uat-specs/castle-cellblock.toml),
[asset UAT](../cellblock-asset-audit/uat-guide.md).
The current `starter_kit_live_db.rs` guard fires loaded ability 592 at zero
Focus and rejects an empty magazine; it is a server simulation, not a native
client pass or proof that the normal tutorial grant/reload path worked. A
native reload fills the magazine only after its completion tick. CB-U02 must
observe that completion before attributing no damage to combat resolution.
The old zone-audit count ending at 687 is a historical summary, not this baseline;
688 and internal 689 are reconciled in the [matrix](evidence-matrix.md).

## 2026-10-14 bounded lab follow-up

This follow-up does not attest any route row. The first-session FS-P1–FS-P5 run
stopped before world entry when the lab bridge could not connect to the default
instance. Its gameplay rows are blocked; it supplies no current route result.

On the separate `p2` instance, an existing level-1 Praxis Soldier character in
world 12 equipped the backpack 9mm Pistol into bandolier slot 1 through the native
client UI, and mission 622 completed. The pistol initially read 0/15; a native
reload changed it to 15/15. The character walked to a living Cellblock Guard,
targeted it at 7.78 m, and remained at full health and Focus. Native hotbar and
Abilities-window attempts found no Pistol Shot (ability 592), so no shot was fired:
ammunition, Focus and guard health stayed unchanged. This does not test damage,
recoil or the first-guard fault in #1667. The character was pre-existing, and its
creation path, server revision and ability-grant provenance were not established.
The next CB-U02 attempt must use a known fresh character and capture the actual
1005 grant before attributing the absent ability to server or client behavior.

A further p3 attempt held the lab lease from 20:47:13 to 20:53:20 UTC on
2026-10-14, then released it. The
`lab_ensure_in_world` call for a fresh Praxis character gave no result after about
six minutes; read-only p3 status still showed `not_started`, so the call was
interrupted. The daemon later logged a 394-second Start error because the lease
had already been released; it never launched SGW.exe. No fresh character
entered the world and no shot was attempted.
At that time the default and p2 clients still reported world 19 and world 12
respectively, while `server_sessions` returned zero and the queried server
database had none of those client character names. Client-to-probed-server
binding was therefore unverified; no server packet/log result is attributed to
either client. CB-U02 remains UATPending for a fresh normal-character run.

A second p3 diagnostic held the same coordinated lease from **21:00:22 to
21:02:26 UTC** on 2026-10-14 and released it. The low-level `lab_client_start`
call stayed pending for about two minutes while read-only p3 status remained
`not_started`; the call was terminated. The daemon later logged a 45-second
build-lane wait, then refused launch because the lease had been released.
No client process or world entry resulted, no pistol step ran, and no server
or gameplay result is inferred. This was a bounded pre-launch cancellation.

On p4, low-level `lab_client_start` returned successfully around 21:03–21:04
UTC. Read-only status at 21:06 showed a running client process but an
unreachable bridge heartbeat and `login_state=not_started`. A fresh Praxis
Soldier `lab_ensure_in_world` failed after 106.8 seconds with
`WaitReady: no client running`; the coordinated lease was released. No world
entry, tutorial grant or combat proof came from p4. Its process-start result
does not establish a usable client bridge. The daemon log reports an idle
main thread with no CPU or I/O; the watchdog terminated the process after
122 seconds without a heartbeat. Post-run status reported
`running=false`, `login_state=crashed`, one crash in ten minutes and an
unreachable heartbeat; the crash report had no dump or command paths.

On p5, the coordinated lease began at **21:07:57 UTC**. A fresh
`lab_ensure_in_world(stop_at="running")` launched a client process, then
returned `WaitReady: no client running` after 145 seconds including launch.
The bridge heartbeat was unreachable, `login_state=crashed`, and status counted one crash
in ten minutes. The daemon log reports an idle main thread with no CPU or
I/O; the watchdog terminated the process after about 120 seconds without a
heartbeat. The crash report had no dump or command paths. The lease was
released. No character selection, world entry or combat occurred. Both p4
and p5 minted telemetry for the selected local server but never logged in or
established a server session. A subsequent read-only `lab doctor` passed
daemon, install, account/profile and stray-process checks but did not
explain the idle main-thread stall.

After the operator confirmed the desktop was unlocked, a p4 retry held a lease from
21:33:42 UTC. The daemon waited about 90 seconds for an active build-lane
slot, logged `lane_busy_launch`, and started a process. The bridge stayed
unreachable, and `lab_ensure_in_world(stop_at="running")` returned
`WaitReady: the client has no main window after 120291 ms`. The process was
stopped and the lease released. No character selection or gameplay occurred.
This run alone does not isolate the window stall from concurrent build load.

A final p4 isolation began at 21:46:42 UTC with the build lane idle and the
desktop unlocked. Direct `lab_client_start` launched a process promptly, but
no main window or bridge heartbeat appeared. The daemon observed process CPU
and I/O throughout its load grace, then terminated it at 21:51:50 after
305 seconds with `heartbeat unreachable`. The lease was released at 21:52:27.
No login, world entry or combat occurred. This confirms a lab startup blocker
even without concurrent build-lane activity; it does not implicate Cellblock
content or prove any CB-U02 gameplay outcome.

## 2026-10-15 fresh Human lab run

The desktop screensaver was still active after the operator reported the screen
unlocked. The first p2 launch returned `D3DERR_NOTAVAILABLE`; stopping the
screensaver process allowed p2 and then p3 to start with live bridge heartbeats.
P2 had no free character slot, so its client and lease were released. P3's
unprotected lab fixture slot was freed and a **fresh Praxis Soldier Human**
character was created. It entered world 12 at the stock character-creation
position `(-334.23, 73.47, -228.03)` at about 02:00 UTC. This was a live client
run, not a staged mission or GM progression.

The first dialog rendered the two screens of 2982. Native movement and corpse
clicks reached Frost, then the Guard's body. Frost's Letter appeared once in
the Mission bag, and the Guard click put the SI 3 9mm Pistol in Main. The client
reported learning abilities 592, 594, 597 and 1218, placed them on the action
bar, and opened the Equipping a Weapon tutorial. Native inventory equip moved
the pistol to bandolier slot 1 and completed 622. The pistol read **0/15**;
clicking the native Reload button still read 0/15 immediately, then **15/15**
after a 2.5-second completion wait.

The character reached the living Cellblock Guard by native movement and
targeted it at **3.1 m**, with 15/15 ammo, 760/760 Health and 470/1570 Focus
at the last pre-shot snapshot. The guard had already depleted Focus from the
earlier full 1570 while the probe read its state. Two native hotbar Pistol Shot
presses each sent `useAbility`: the first missed and spent one round (15→14),
and the second, at zero Focus, crit for **48 Health** and spent another round
(14→13; guard 250→202 Health). The client also displayed the zero-Focus
warning, so that warning did not mean the shot was refused. Four more native
presses killed the guard; a seventh post-kill press also spent a round. The
final magazine read 8/15 and the character survived. This narrows #1667: a
fresh character's normal grant/reload/fire/damage path worked in this run, but
one miss and severe incoming damage remain relevant to first-guard balance.
No screenshot or verified visual recoil frame was captured.

A character-select relog preserved the one equipped pistol, Frost's Letter,
8/15 ammo and the four action-bar abilities. The mission tracker showed 622
complete. The relog chat capture warned that its hook attached after world
load, so absence of duplicate grant text is unproven; the persisted inventory
contained no duplicate pistol or letter.

The Codex `lab-server` MCP connection for this session was configured for a
different server. Its `server_sessions` returned zero throughout; those reads
cannot be attributed to this client. The lab daemon's local server setting and
the client/server halves of the gameplay socket identified the matching local
server. A direct read of that server's MCP endpoint found this character in
world 12 after relog (entity 15, player 105). Its read-only database showed
the character row; a live entity snapshot agreed with the client's later
position. There was no packet tap covering the first guard, and the exact
server source revision and client patch revision were not captured. The live
server executable belonged to a separate AOI build target, not the campaign
checkout. **CB-U02 remains UATPending** for the full revision-bound,
packet-correlated and visual acceptance row.

After the client stopped, a read-only query against the matching local server
found mission 622 completed, mission 1360 active on step 4037, and mission 638
active on step 2115. It found exactly one `type_id=55` pistol in Bandolier with
8 rounds, one `type_id=3730` letter in Mission, and one tutorial-source grant
row each for 592, 594, 597 and 1218. This corroborates the relog persistence
and lack of duplicate item/ability rows, not the uncaptured first-guard packet
sequence.

The character then walked natively to Prisoner 329. Region2 accepted mission
638; the Human dialog opened with the stock “You feel uncomfotable” text and
advanced the tracker to the cell-door controls. A native click launched
Livewire. The matching local server retained a `ready` board with two goals
and no rollover/cut events. `client_minigame_play` did not return after more
than two minutes; a separate `lab_screenshot` call also did not return. The
client heartbeat remained alive, but after those calls the character was at
about `(-79.21, 45.18, -163.82)` in the same world while mission 638 still
required hacking the door. The matching server's live entity position agreed;
its last persisted character position still preceded the door. No successful
Livewire input, mission advance or legitimate route travel is inferred from
that relocation. Both stalled calls were cancelled, the p3 client was stopped,
and its lease released. **CB-U09 has no valid core-duration measurement**, and
the minigame/screenshot stall plus unexpected relocation must be isolated
before another fresh timing run.

## Bounded fresh acceptance rows

All rows below are **UATPending**. O14-08 grants standing approval for the four
zone sessions to lease/start/restart the lab and select a reviewed local server,
subject to one-lease coordination. Record server/client/patch versions, player archetype,
scenario, timestamps and client-visible result beside server action evidence.
Do not attest first-press feedback from `content.resolve` or a helper's return alone.

| Row | Probe and required observation | Owner / dependency |
|---|---|---|
| CB-U01 | Fresh Human and Jaffa boot lock; use 3438, native Livewire, unlock; relog before/after. Show actual movement/UI plus 1022–1025 state. | N02; minigame/items, existing T25 |
| CB-U02 | On a fresh normal character, capture chain 1005 granting pistol 55 and ability 592, equip, native reload from 0/15 to loaded, Focus before guard acquisition, at least two native shots at about 3–4 m, ammo/guard HP and visible fire/recoil. Correlate server launch/refusal, ammo consumption, effect/sequence and packet results; repeat/relog without duplicate grants. A zero-Focus warning alone does not prove an ability refusal. | N02; #1667 combat/items/class-start owner |
| CB-U03 | 329 dialog/door and med-station cover+kill in both orders; real cure feedback and sickness state. | N02/N03; T07 and existing route rows |
| CB-U04 | Rings to Preparation/topside, Marsh first/repeat/relog escort/barks, terminal/pods. A completed mission alone cannot attest opening. | N01 + ring/asset/class-start |
| CB-U05 | Mess Hall/Hallway05 guard acquires cover; player flanks with threat; objective credit and kill-only progression; out-of-order dead-guard restoration. | N03; existing T29 |
| CB-U06 | Straegis scene, Marsh departure, 2516 then 5859, actual readable interval; death/relog/late entry. | N01 + asset CA-U rows |
| CB-U07 | Class crate once, terminal 2356→80688, skip optional armory kill, arrival in Castle; 1360 retained and no double reward on relog. | N02/N08 + ring 1109 and Castle CS-08 |
| CB-U08 | Each candidate cast pocket: walk/collision/nav/LoS, camera and two-client AoI; no tutorial interaction occlusion or unintended aggro. | N04 seal; NPC/presentation |
| CB-U09 | **Pre-N05 dispatch gate:** run a fresh normal Human/Jaffa first-run core escape with optional content skipped; record each character's start/end, core duration, deaths and measured free time against O14-14's 30–45 minute core and 10-minute optional limit. Hold N05 implementation until this baseline supports its estimated detour. After integration, repeat complete routes with selected optional content taken/skipped and compare the added time. The old 37m06s privileged run does not satisfy this row. | N02/N08 baseline before N05; repeat after selected packets |
| CB-U10 | First authored mission 62050: native Human and Jaffa first click on Prisoner 329 after 638, offer marker, category-3 journal/title and A→B advance, category-5 dialog 60150 render, Marsh B completion without losing core 1051/1052/1152, then repeat click, relog before/after completion, abandon/reaccept and skip. Record server/client/patch revisions, per-character mission/DB state, exact `(3, 62050)` and `(5, 60150)` transfer evidence, content/packet logs and optional elapsed time; blank dialog/crash/remote mutation/core briefing regression blocks. | N05 after CB-U09 pacing gate and integrated local review; CB-N05a first-catalog proof |

## Sibling ownership reconciliation

Read-only `git worktree list`, branch tips and `git merge-base --is-ancestor <tip>
HEAD` were checked on 2026-10-14. Registered does not mean active, and non-ancestry
can result from squash merging: it is not proof that a change is missing from main.
No peer branch or worktree was changed. Local agent-board launcher was absent at
its documented location; no board state or peer approval is claimed.

| Sibling | Observed status / branch evidence | Exclusive ownership / required handoff |
|---|---|---|
| [Cellblock rebuild](../castle-cellblock-rebuild/work-packets.md) | C00–C08/GC1 implementations recorded merged; seed confirms escort, boots, flank/backstops and current dialog timing. Old UAT guide's pending language is not a fresh verdict. | Existing route intent/tests. N02/N03 only reproduce current defects. |
| [Asset audit](../cellblock-asset-audit/README.md) | Static E01–E15 and 2026-10-07 knowledge gaps; no new live pass established here. | Pod, door, barrier, Ambernol/weapon visual, Straegis presentation. N01 obtains exact CA packet verdicts before allocating changes. |
| [Ring campaign](../ring-transport-cellblock-castle/README.md) | Ledger: Phase 0 client load passed 2026-09-19; Phase 1 built awaiting client test. Registered `feat/client-patch-007-castle-armory-ring` tip `6a9818c09` dated 2026-09-29; not an ancestor of baseline. | Chain 1109, ring map/patch/FSM and Castle arrival. Exact current deployment/patch acceptance UNSEALED. |
| [Class-start v6](../class-start-v6/README.md) | Ledger status dated 2026-10-06; current 622/687 seed confirms later tutorial grants and five-way rewards plus holding branch. | Starting gear, reward policy; do not resurrect historical two-way rewards from old docs. Current class acceptance UNSEALED. |
| [Autoplay](../cellblock-autoplay/README.md) | README says nothing built at 2026-09-29, but registered route-tools `013a48cb3` and AP-20 spec `baf1f3f59` dated 2026-10-14 exist; neither tip ancestor of baseline. | Driving/route tooling and walkthrough spec. Obtain live status from coordinator; no duplicate automation or assumption of readiness. |
| [Lab reliability](../lab-uat-reliability/README.md) | Ledger branch `09bc61640` dated 2026-10-13, non-ancestor; committed observations identify product vs helper failures. | Lab helper corrections. Native feedback/fixture validity must be checked before blaming content. |
| Castle / Harset / Tollana continuation gates | Registered `castle/cs-00-contract`, `harset/ha-00b-ledger`, `tollana/t00-evidence` at common initial `d3efb7d41`. | Cross-zone IDs and recipient tags unclaimed here; Castle CS-08 owns 1360 Castle leg. Coordinator reconciles new gate commits. |

No sibling's stale Ready/Done row is adopted as current acceptance. N01 still owns
the exact presentation/branch sign-off. Known runtime work is not declared unowned
until that handoff completes.

## World-12 population and safe-pocket register

Source: `db/resources/Worlds/Seed/spawnlist.sql` at baseline. All world-12 INSERT
rows are enumerated below; coordinates/headings are seed values, **not validated
placement recommendations**. Headings retain seed units. Safe status for every
new placement is UNSEALED. DebugHub, CraftHub and BlackMarket rows are development
conveniences; they do not authorize a prison service hub or prove historical cast.
The unused ring switch is a separate existing prop, not a safe new destination.

| Spawn ID | Template | Tag | Position x, y, z | Heading | Category |
|---|---|---|---|---|---|
| 6 | 17 | `Prisoner_329` | -289.808014, 65.473999, -113.139 | 3.1414969 | Existing route/prop |
| 7 | 10 | `Preparation_ColMarsh` | -191, 54.7199974, -138.587997 | 3.1414969 | Existing route/prop |
| 8 | 13 | `Cellblock_WoodenCrate` | -130.449997, 24.6709976, -92.0699997 | 1.57079637 | Existing route/prop |
| 10 | 4 | `ArmYourself_PrisonerRetrievalUnit` | -220.257004, 66.7440033, -121.375 | 4.71238899 | Existing route/prop |
| 11 | 8 | `Preparation_SMG1A` | -201.25, 56.079998, -131.610001 | 1.57079637 | Existing route/prop |
| 12 | 18 | `ArmYourself_AmbernolVial` | -234.039993, 66.5199966, -124.699997 | 1.57079637 | Existing route/prop |
| 15 | 21 | `ArmYourself_GuardBody` | -322.509979, 73.4720001, -209.830002 | 1.57079637 | Existing route/prop |
| 16 | 19 | `Preparation_Terminal` | -187.714005, 55.848999, -141.503998 | 4.71238899 | Existing route/prop |
| 17 | 3 | `Preparation_RingSwitch` | -193.919998, 56.3199997, -152.160004 | 1.10139823 | Existing route/prop |
| 19 | 14 | `ArmYourself_FrostBody` | -328.299988, 73.4720001, -210.269989 | 1.57079637 | Existing route/prop |
| 20 | 15 | `ArmYourself_NIDGuard` | -289.464996, 68.5419998, -154.275986 | 3.1414969 | Existing route/prop |
| 22 | 22 | `329_CellDoorButton` | -287.290009, 67.2600021, -115.040001 | 0 | Existing route/prop |
| 23 | 3 | `HackTheRings_Switch` | -218.080002, 67.0400009, -122.719994 | -0.56776464 | Existing route/prop |
| 25 | 24 | `Barracks_Guard1` | -118.409996, 24.6700001, -118.349998 | 1.57079637 | Existing route/prop |
| 26 | 24 | `Barracks_Guard2` | -131.479996, 24.6700001, -116.969994 | 0 | Existing route/prop |
| 27 | 24 | `Cellblock_ArmoryGuard1` | -49.6899986, 24.6700001, -127.110001 | 3.1414969 | Existing route/prop |
| 28 | 24 | `MessHall_Guard2` | -95.8899994, 34.5909996, -98.8079987 | 2.09426737 | Existing route/prop |
| 29 | 24 | `MessHall_Guard1` | -96.25, 34.5909996, -91.5899963 | 2.09426737 | Existing route/prop |
| 30 | 24 | `Hallway01_Guard` | -128.852997, 39.5519981, -73.5339966 | 3.1414969 | Existing route/prop |
| 31 | 24 | `Hallway03_Guard` | -98.5799942, 39.5499992, -77.0899963 | 0 | Existing route/prop |
| 32 | 24 | `Hallway05_Guard1` | -100.159996, 24.6700001, -43.8950005 | 1.57079637 | Existing route/prop |
| 33 | 24 | `Hallway05_Guard2` | -101.5, 24.6700001, -51.2999992 | 1.57079637 | Existing route/prop |
| 34 | 19 | `Cellblock_TerminalX` | -52.6599998, 25.763998, -151.12999 | 0 | Existing route/prop |
| 36 | 24 | `Barracks_Guard3` | -136.384995, 24.6709976, -135.671005 | 0 | Existing route/prop |
| 79 | 3 | `Cellblock_ArmoryRingSwitch` | -54.8799973, 26.0799999, -163.839996 | 1.04607904 | Existing route/prop |
| 80 | 3 | `Cellblock_UnusedRingSwitch` | -87.0400009, 46.2399979, -160.319992 | 2.59664607 | Existing route/prop |
| 82 | 24 | `Hallway02_Guard` | -113.485001, 39.5519981, -63.0419998 | 0 | Existing route/prop |
| 86 | 24 | `Hallway04_Guard` | -61.348999, 34.5909996, -69.0319977 | 0 | Existing route/prop |
| 400 | 300 | `DebugHub_Vendor` | -343.9, 73.472, -228.49 | 1.4638 | Development convenience |
| 401 | 301 | `DebugHub_Trainer` | -341.26, 73.472, -229.92 | 1.2651 | Development convenience |
| 402 | 302 | `DebugHub_DialogNpc` | -338.63, 73.472, -231.35 | 0.9473 | Development convenience |
| 403 | 303 | `DebugHub_LivewireTerminal` | -333.35, 73.472, -234.21 | 0.0464 | Development convenience |
| 404 | 304 | `DebugHub_LootCrate` | -330.72, 73.472, -235.64 | -0.271 | Development convenience |
| 405 | 305 | `BlackMarket_Auctioneer` | -328.48, 73.472, -222.52 | -2.3829 | Development convenience |
| 410 | 310 | `CraftHub_Station_BioMedical` | -340.43, 73.472, -224.01 | 1.9913 | Development convenience |
| 411 | 311 | `CraftHub_Station_Electronics` | -339.2, 73.472, -221.72 | 2.3079 | Development convenience |
| 412 | 312 | `CraftHub_Station_PowerSystems` | -337.96, 73.472, -219.44 | 2.583 | Development convenience |
| 413 | 313 | `CraftHub_Station_Materials` | -336.72, 73.472, -217.15 | 2.7937 | Development convenience |
| 414 | 314 | `CraftHub_Supplies` | -341.67, 73.472, -226.29 | 1.6889 | Development convenience |
| 430 | 330 | `DebugHub_TeamRegistrar` | -327.40, 73.472, -233.91 | -0.7086 | Development convenience |
| 431 | 331 | `DebugHub_CommandRegistrar` | -330.04, 73.472, -232.48 | -0.5276 | Development convenience |
| 450 | 360 | `DebugHub_PetTrainer` | -328.08, 73.472, -237.07 | -0.4698 | Development convenience |
| 470 | 370 | `DebugHub_Banker` | -325.92, 73.472, -231.18 | -1.0739 | Development convenience |
| 471 | 371 | `DebugHub_TeamBanker` | -327.65, 73.472, -228.09 | -1.4294 | Development convenience |
| 472 | 372 | `DebugHub_CommandBanker` | -325.84, 73.472, -224.74 | -1.9148 | Development convenience |
| 490 | 390 | `DebugHub_MailClerk` | -324.11, 73.472, -227.84 | -1.5123 | Development convenience |

Census: 46 world-12 spawns, including 18 development convenience rows.

| Candidate pocket | Seed/route anchor | Why it remains UNSEALED / closure |
|---|---|---|
| Stasis side/cells | Frost/Guard bodies 19/15 and development cluster 400–490 | Tutorial targets, boot movement and debug crowd already compete. Native walking, camera and two-client AoI before any cast; no added aggro. |
| Prisoner 329 cell edge | 6/22 and door progression 638 | Door swing, interaction reach, path and release state not physically checked. Asset/NPC UAT first. |
| Preparation side wall | Marsh 7, SMG 11, terminal 16, switch 17 | Locker/dialog/terminal/ring bottleneck; no safe coordinate selected. Walk/escort/LoS and first-click tests first. |
| Barracks after clearance | Crate 8, guards 25/26/36 | Safety is state-dependent; mission/relog suppression and aggro/collision tests required. |
| Armory side space | 27/34/79 | Exit bottleneck under ring ownership; do not place an NPC at the switch/pad. Ring layout/arrival and two-client tests required. |

Med station, Mess Hall, Hallway05 and all ring pads are protected tutorial/combat
surfaces; no new safe pocket is asserted there. Exact added cast transforms,
dialogue, count and rewards remain in the [decision log](decision-log.md).

## ID audit and limitations

A declaration-only scan of every `db/resources/Content/Seed/*.sql` at the
CB-N00 baseline found 120 Cellblock-file chains and no duplicated chain IDs
across seed files. No seeded chain occupied 11000–11499 at that baseline.
This historical scan alone was not an all-branch collision clearance.
The later proposed [CB-N05 individual claims](../four-zone-planning/id-allocation.md#cb-n05-individual-claims--2026-10-14)
record a separate stock/seed/four-branch audit for 11000–11005 and the mission,
dialog and screen IDs; recheck immediately before coder dispatch.
