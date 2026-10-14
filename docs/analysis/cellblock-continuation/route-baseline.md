# Cellblock route baseline

> Static baseline `d3efb7d41aad0b6008397f112d224ce7430d7acf`, audited 2026-10-14.
> No fresh route run, deaths, duration, screenshots, packet tap or lab actions.

## Observations versus current faults

No current runtime fault was reproduced by CB-N00. This does **not** mean no fault
exists. The following observations retain their original scope and date.

| Evidence row | Source and observation | Current disposition |
|---|---|---|
| CB-B01 | [2026-10-02 dossier, Session A](../playtests/2026-10-02-castle-signoz-dossier/README.md): 2026-09-28 dev entry 08:43:21 UTC, Castle entry 09:20:27, 37m06s and five logged Cellblock deaths. Completed 622/638–641/680–688; privileged access_level=2. | T: one historical observation, not target duration, average or normal-player success rate. Fresh timing/death count UATPending. |
| CB-B02 | Same Session A: sequence 1751 dispatch at 09:06:45 after 686. Session B had client event telemetry, not visual/audio acceptance. | T: dispatch does not prove camera/rift/blood/playback or first-press delivery. N01/asset UATPending. |
| CB-B03 | [Lab reliability follow-up](../lab-uat-reliability/README.md), 2026-10-13: corpse click/equip progressed first mission, first guard defeated character; pistol showed 0/15 and no damage. Walkthrough did not reach Livewire/destination ring. | C as reported, not independently rerun. Issue #1667 checked by gh on 2026-10-14: OPEN, updated 2026-10-13T22:50:06Z. Preserve known blocker; N02 needs bounded current reproducer and combat/items ownership. |
| CB-B04 | Same source: respawn at Mess Hall preceded helper observation (#1668); dialog-close helper reported success while dialog remained (#1706); unnamed corpse point hit avatar (#1708). | Harness/product distinction required. Lab campaign owns helper behavior; no content defect inferred from a tool success return. |
| CB-B05 | Current seed 1172 delays dialog 5859 by 26100ms after scene start; 1161 delays 2516 by 10100ms. `mission_686_straegis.rs::live_db_straegis_followup_allows_aftermath_dialog_to_be_read` exists. Rebuild UAT guide cites #1721. | S: old short-gap repair already present; do not open duplicate N02 repair. Real dialog reading/closure remains UATPending. |
| CB-B06 | Current seed 689 chains 1022–1025; all six 681–686 controllers hidden in missions.sql; hidden/backstop/flank tests exist. | S: earlier inert-boots/visible-controller claims cannot establish current defects. No compile or live-DB pass claimed here. |
| CB-B07 | Current 1107 advances to 80688; 1109 completes 688 then cross_world_teleport; `mission_688.rs::live_db_chain_1109_completes_688_and_cross_world_teleports` guards fallback. | S: ring campaign owns replacement. N00 does not prove native ring animation or arrival placement. |

Sources of planned probes:
[rebuild UAT](../castle-cellblock-rebuild/uat-guide.md),
[current spec](../../guides/uat-specs/castle-cellblock.toml),
[asset UAT](../cellblock-asset-audit/uat-guide.md).
The old zone-audit count ending at 687 is a historical summary, not this baseline;
688 and internal 689 are reconciled in the [matrix](evidence-matrix.md).

## Bounded fresh acceptance rows

All rows below are **UATPending**. Obtain separate owner approval before any lab
lease/start/server change. Record server/client/patch versions, player archetype,
scenario, timestamps and client-visible result beside server action evidence.
Do not attest first-press feedback from `content.resolve` or a helper's return alone.

| Row | Probe and required observation | Owner / dependency |
|---|---|---|
| CB-U01 | Fresh Human and Jaffa boot lock; use 3438, native Livewire, unlock; relog before/after. Show actual movement/UI plus 1022–1025 state. | N02; minigame/items, existing T25 |
| CB-U02 | Frost/guard search, pistol equip and loaded ammo, first guard damage/fire/recoil. Capture refusal or combat trace correlated to visible action. Repeat/relog without duplicate grants. | N02; #1667 combat/items owner |
| CB-U03 | 329 dialog/door and med-station cover+kill in both orders; real cure feedback and sickness state. | N02/N03; T07 and existing route rows |
| CB-U04 | Rings to Preparation/topside, Marsh first/repeat/relog escort/barks, terminal/pods. A completed mission alone cannot attest opening. | N01 + ring/asset/class-start |
| CB-U05 | Mess Hall/Hallway05 guard acquires cover; player flanks with threat; objective credit and kill-only progression; out-of-order dead-guard restoration. | N03; existing T29 |
| CB-U06 | Straegis scene, Marsh departure, 2516 then 5859, actual readable interval; death/relog/late entry. | N01 + asset CA-U rows |
| CB-U07 | Class crate once, terminal 2356→80688, skip optional armory kill, arrival in Castle; 1360 retained and no double reward on relog. | N02/N08 + ring 1109 and Castle CS-08 |
| CB-U08 | Each candidate cast pocket: walk/collision/nav/LoS, camera and two-client AoI; no tutorial interaction occlusion or unintended aggro. | N04 seal; NPC/presentation |
| CB-U09 | Complete routes with optional stories skipped/taken, separate archetypes, record duration and deaths individually. | N08 after selected packets and local review |

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

A declaration-only scan of every `db/resources/Content/Seed/*.sql` at baseline
found 120 Cellblock-file chains and no duplicated chain IDs across seed files.
No seeded chain occupies 11000–11499. This is a **current-seed check only**;
no new IDs were allocated and no all-branch collision clearance is claimed.
The shared allocation table is unchanged. Other-zone gate commits and stock mission
catalog absence must be reconciled by the coordinator before any claim becomes Ready.
