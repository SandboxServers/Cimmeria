# Four-zone ID allocation

> **Type:** Reference
> **Audience:** Zone campaign coordinators and the ID-lock packets (CB-N00, CS-00, HA-00b, T-01)
> **Last updated:** 2026-10-18
> **Companions:** [four-zone index](README.md), [execution contract](execution-contract.md), [2026-10-14 owner decisions](owner-decisions-2026-10-14.md), [Harset chain ranges](harset-work-packets.md#contract-and-dispatch-boundary), [Tollana packets](tollana-work-packets.md)

Owner decision, 2026-10-13: the four zone campaigns share one content-chain ID table so that no two campaigns, branches or workers pick the same `chain_id`. This page is that table. A zone's ID-lock packet reads it, claims individual IDs inside its own block, and records the claim in its zone ledger before any coder receives an ID.

## Chain IDs in use today

Measured on 2026-10-13 at `origin/main` 885bbae05 from the first value of each `INSERT INTO content_chains` statement in `db/resources/Content/Seed/*.sql`. Every file uses single-row inserts; read only the `chain_id` column, because description strings such as `change_stat(8, 0, 100)` contain numbers that are not chain IDs. No `chain_id` appears in two files. Chain 4001 exists only in the one-off script `db/scripts/add_health_slappack_use_chain.sql`, not in the seed; it collides with nothing here.

| Seed file | Chains | IDs in use |
|---|---:|---|
| `effects_chains.sql` | 7 | 2001, 2011, 2021–2025 |
| `castle_cellblock_chains.sql` | 145 | 1001–1195 (sparse: 1001–1028, 1031–1034, 1036–1037, 1041–1046, 1051–1066, 1071–1074, 1081–1094, 1097–1112, 1121, 1131–1133, 1141–1142, 1151–1152, 1154, 1161–1162, 1171–1178, 1181–1195) and the single outlier 5030, and 11010, and 11000–11005, 11020, 11030–11041 |
| `castle_701_chains.sql` | 18 | 1201–1205, 1231–1243 |
| `castle_702_704_chains.sql` | 23 | 1261–1265, 1271–1276, 1291–1302 |
| `castle_706_708_chains.sql` | 28 | 1321–1323, 1341–1365 |
| `sgc_w1_chains.sql` | 41 | 3001–3037, 3041–3044 |
| `harset_space_chains.sql` | 7 | 6001–6007 |
| `harset_goauld_chains.sql` | 27 | 6101–6127 |
| `harset_jaffa_chains.sql` | 18 | 6301, 6303–6308, 6331, 6333–6342 |
| `harset_opcore_chains.sql` | 21 | 6501–6505, 6511–6516, 6518–6523, 6525–6528 |
| `debug_hub_chains.sql` | 8 | 7001–7005, 7010–7011, 7020 |
| `tutorial_chains.sql` | 1 | 7101 |
| `dakara_e1_space_chains.sql` | 6 | 8001–8006 |
| `debug_area_plaza_chains.sql` | 9 | 13000–13008 |
| `debug_area_ring_chains.sql` | 9 | 13810–13818 |
| `debug_area_lineup_chains.sql` | 6 | 14090–14095 |

The planning review's working figure of "castle_cellblock 1001–5030" was a min/max span, not a block. The real Cellblock chains stop at 1195 plus one outlier, 5030, so they do not overlap the Castle (1201–1365), effects (2001–2025) or SGC W1 (3001–3044) chains. Treat every listed ID as taken even where it sits inside another zone's span.

**Free and cited:** 1381–1400, the range the old Castle CA15 draft proposed, had no chain at the 2026-10-13 scan. The owner separately approved 1366 for CS-08a and delegated individual Castle restoration-chain allocations in 1367–1400 on 2026-10-14, subject to the current collision audit in O14-04/O14-22. A free range is not a claim on every ID; CS-00 records each allocated ID here before dispatch.

### Castle 1520/1521 entry claims — 2026-10-14

The Castle coordinator claims these **content-chain** IDs under O14-22 for the
reviewed Infirmary-entry reconstruction. This allocation freezes identifiers;
it does not mark CS-02/03 runtime or client behavior complete.

| Chain ID | Castle packet | Exact use |
|---:|---|---|
| 1367 | CS-02 entry | Post-704 `Castle.Infirmary` offer of mission 1520 to archetypes 1–4 and 6. |
| 1368 | CS-02 entry | First-step region replay credits 1520 objective 5171 only. |
| 1369 | CS-03 entry | Post-704 `Castle.Infirmary` offer of mission 1521 to archetype 8. |
| 1370 | CS-03 entry | First-step region replay advances 1521 to step 4456, crediting sole objective 5175 without completing the mission. |

Collision audit at `origin/main` `817d3792c`, Castle `583a85f47`, Cellblock
`70605d817`, Harset `02c67f8e9`, Tollana `319d768e2`, plus the root main
checkout `f53be3f8e`: anchored `content_chains` first-tuple scans of the
content seed found zero **chain** uses of 1367–1370. Each active zone
integration worktree and the shared chain table were checked; no other
individual chain claim exists. The same numbers may exist in dialog or entity
template namespaces and do not collide with a chain ID. The stock cooked
mission catalog is a distinct namespace: missions 1520/1521 are existing
records, while this claim introduces no mission ID. Recheck the four live
branches immediately before a coder writes the seed, since this audit is a
point-in-time check rather than a lock.

## Reserved blocks

| Block | Owner | Use |
|---|---|---|
| 1366 | Castle CS-08a | Individual reservation owner-approved 2026-10-14; Frost's Letter bridge. See dated reservation below. |
| 1367–1400 | Castle restoration | Seeded by Castle (final pass below): 1367–1383 (missions 1520/1521), 1386 (1521 implant), 1394–1397 and 1400 (567 Castle leg). 1398 grants 1664 (FZ-18). Chains 1384, 1385, 1387–1393 and 1399 are unseeded; 1387–1393 stay reserved, 1399 is dropped (CD-1, O18-02). |
| 6001–6800 | Harset | Reserved by the old Harset ledger: space 6001–6099, Goa'uld 6101–6300, Jaffa 6301–6500, OP-CORE 6501–6800. Per-mission sub-ranges are in [harset-work-packets.md](harset-work-packets.md). |
| 8001–8499 | Dakara_E1 | Dakara_E1 rebuild ([ledger](../dakara-e1-rebuild/README.md)): space and travel 8001–8039 (8001–8006 in use), mission 1570 8040–8074 and 8160–8179, mission 1645 8080–8119, mission 1646 8120–8159, later missions 8180–8499 (8075–8079 unassigned). Per-file ranges: [Dakara work packets, C3](../dakara-e1-rebuild/work-packets.md#c3-seed-files-and-chain-ranges-d-dk14). |
| 9000–9799 | Tollana | Tollana restoration, population, authored and repeatable chains. |
| 9800–9999 | Tollana (Harset recipients) | Owner decision, 2026-10-13: Tollana authors the Harset-side recipient chains for 603, 611, 618, 621, 1626, 1632 and 1643 here. Harset HA-00b signs off the NPC tags they use. |
| 10000–10999 | Castle | Castle authored stories, repeatables and hub chains (CS-04a/b/c, CS-05). Seeded and reserved ranges are in the Castle final pass below. |
| 11000–11499 | Cellblock | Cellblock authored stories (CB-N05, CB-N05b, CB-N06) and any new restoration chain CB-N00 cannot fit beside the existing Cellblock chains. |
| 11500–12999 | Cross-zone / infrastructure | Reserve. A packet claims IDs here only with the coordinator's amendment to this page. |
| 23000–23499 | Union Beta E1 | Beta_Site_Evo_1 (world 23) first-visit chains. Claimed 2026-10-18 by the coordinator's amendment (D-UB9); per-mission sub-ranges and the non-chain tables (spawns, point sets, templates 2300–2349, respawners, ring regions 2300–2309, authored 64000–64499) are in [id-amendment-request.md](../union-beta-e1-continuation/id-amendment-request.md). |

13000–14999 holds the debug-area chains listed above; leave it alone.

### T-01 individual Tollana claim — 2026-10-14

Tollana T-01 claims **chain 9001** for the first mission-568 implementation
chain (T-03a), subject to the sealed trigger/action design and a recheck before
seeding. The [Tollana ID register](../tollana-continuation/tollana-id-register.csv)
also records its separate server-only template and spawn claims. A first-tuple
`INSERT INTO content_chains` scan on 2026-10-14 found no 9001 in
`origin/main` or the Cellblock, Castle, Harset and Tollana integration
worktrees. The Tollana block is not otherwise consumed by this claim; no
dialog or item ID has been assigned. This is a point-in-time collision check.

### W1 Tollana claim — 2026-10-17

Tollana W1 ([packets](../tollana-continuation/w1-packets.md)) extends the T-01
claim to **chains 9001–9018** (mission 568: 9001–9010 Copplemann, accept,
turn-in, restore and abandon; 9011–9018 food-unit props), entity templates
**420–421** and spawns **19001–19004**, with 19005–19010 reserved and unused.
Dialogs 2153, 2154, 4211, 4213 and 4214 and dialog-set-maps 2656, 4959 and
6393 are reused; no dialog or item ID is allocated. A per-table seed scan on
2026-10-17 found none of these ids or the four `Tollana_*` spawn tags at
`origin/main` `b2dd72c66` or the Castle, Cellblock, Harset (three branches)
and Tollana campaign branches. This is a point-in-time collision check; the
[Tollana ID register](../tollana-continuation/tollana-id-register.csv) holds
the rows.

## Harset-proposed restoration subranges (inactive)

HA-00b's documentation repair records the [exact Harset candidates](../harset-continuation/id-allocation.md). These are proposed internal subranges of Harset's existing 6001–6800 block and **are not active claims until coordinator signoff**, sibling-claim reconciliation and per-graph sealing. HA-00c separately owns authored/repeatable mission, step, objective, task, dialog, item and chain allocation; HA-17/18 cannot bypass HA-00c or CB-N05a using this restoration lock.

| Restoration owner | Chain candidates |
|---|---|
| HA-01b 1664 (FZ-20: 6008–6016 seeded; FZ-25: 6017 seeded; FZ-27: 6018–6019 seeded, 6033–6034 seeded; 6020 reserved) | 6008–6020, 6033–6034 |
| HA-01a doors / follow-up capacity | 6021–6024 / 6025–6032 |
| HA-03 | 6311–6330 |
| HA-04 / HA-05 | 6346–6380 / 6381–6400 |
| HA-06a/b/c | 6401–6415 / 6416–6440 / 6441–6470 |
| HA-07 | 6531–6570 |
| HA-08a/b/c | 6571–6590 / 6646–6665 / 6691–6710 |
| HA-09 | 6666–6680 / 6621–6645 |
| HA-10a/b/c | 6591–6605 / 6606–6620 / 6681–6690 |
| HA-11 / HA-12 | 6131–6165 / 6166–6200 |
| HA-13a/b/c / HA-14 | 6201–6245 / 6246–6275 |

Other restoration candidates: spawn IDs 360–379, templates 260–269, point sets 2130–2159 and points 2560–2679; existing Bar/pens reservations 2108/2109 retained. Local ledger rows bind these IDs to consumers and evidence gates. At `d3efb7d41aad0b6008397f112d224ce7430d7acf`, all 495 chain candidates and those other candidates are absent from globally scanned seed primary keys, with no overlap between exclusive ranges. The recorded registered-branch committed-snapshot scan found zero candidate collisions; see the bounded [collision record](../harset-continuation/id-allocation.md#collision-verification-and-limits) and fixer recheck in [HA-00b worknote](../harset-continuation/worknotes/HA-00b.md). Uncommitted and later sibling claims require coordinator reconciliation before activation.

## Other IDs

### CB-N05 individual claims — 2026-10-14

These IDs belong only to the proposed first authored world-12 mission [*The Remaining Pods*](../cellblock-continuation/worknotes/cb-n05-seal.md). They are individual claims, not a new range reservation. The seal passed review; W3-INT (2026-10-17) moved every claim to in use. Recheck immediately before implementation because the audit is point-in-time.

| Namespace | Claimed ID | Exact use | Status |
|---|---:|---|---|
| Mission | 62050 | *The Remaining Pods*; one-time, itemless, zero reward | In use (W3-INT) |
| Mission step | 62051, 62054 | Prisoner report; Marsh warning | In use (W3-INT) |
| Mission objective | 62052, 62055 | One required objective per step | In use (W3-INT) |
| Mission task | 62053, 62056 | One task per objective | In use (W3-INT) |
| Dialog root | 60150 | One-screen, zero-button, speaker-0 narrator dialog | In use (W3-INT) |
| Dialog screen | 96110 | Sole screen of dialog 60150 | In use (W3-INT) |
| Content chain | 11000–11005 | Offer, login restore, abandon restore, Prisoner click, nested accept, Marsh click; six individual IDs | In use (W3-INT) |

The coordinator checked mission 62050 and nested IDs 62051–62056 against stock `CookedDataMissions.pak`, dialog 60150 against stock `CookedDataDialogs.pak`, and screen 96110 against stock dialog XML, with zero hits. On 2026-10-14, first-tuple checks of mission/dialog/content seed rows and resource registries in the four integration worktrees found zero candidate hits: Cellblock `71b470774`, Castle `3cdc609dd`, Harset `ce2ac3293`, Tollana `319d768e2`. The chain scan checked each `content_chains` first tuple; numbers inside action text and unrelated dialog screens are not chain claims. Existing dialog-set-map 2795 is reused, not claimed here. Castle's independently claimed chains 1367–1370 are outside Cellblock's block. The [Cellblock ledger](../cellblock-continuation/README.md) records review and UAT status.

### CB-N02 restore claims — 2026-10-16

Three individual Cellblock chain claims for the wave 1b relog restores in the [Cellblock ledger](../cellblock-continuation/w1b-packets.md). Each sits in the seed's own declared range for its mission (638: 1011–1030, 639: 1031–1040), so none comes from the 11000–11499 reservation.

| Chain | Packet | Exact use | Status |
|---:|---|---|---|
| 1028 | CB-N02b | 638 `player_loaded` restore of the `329_CellDoorButton` Livewire bit at step 2115 | In use (W1b-INT) |
| 1036 | CB-N02c | 639 `player_loaded` restore of the `ArmYourself_AmbernolVial` interact bit at step 2145 | In use (W1b-INT) |
| 1037 | CB-N02d | 639 `player_loaded` re-arm of `ArmYourself_PrisonerRetrievalUnit` at step 2144 | In use (W1b-INT) |

Collision check, 2026-10-16: the first tuple of every `INSERT INTO content_chains` under `db/resources/Content/Seed/` at Cellblock `e10fd1f0a` uses none of the three, and a `git grep` of the same statements across all 703 local branches (every zone integration branch and packet branch) found zero hits. Point-in-time; W1b-INT moves the rows to the in-use table when it integrates them.

### CB-N04 claims — 2026-10-17

Individual claims for the wave 2 ambient cast (cluster 1) in the [Cellblock ledger](../cellblock-continuation/w2-packets.md). The chains come from the 11000–11499 Cellblock authored block, beside CB-N05's 11000–11005. The seal (CB-N04s) uses the lowest IDs it needs; W2-INT marks the rest released.

| Namespace | Claimed IDs | Exact use | Status |
|---|---|---|---|
| Content chain | 11010, 11011, 11012 | One `enter_region` ambient `npc_bark` chain per bark (1 or 2 expected) | In use (W2-INT): 11010. Released (W2-INT): 11011, 11012 |
| Spawn (world 12) | 520, 521, 522 | Static friendly Op-CORE prisoners tagged `Cellblock_Ambient_*` (1 to 3) | In use (W2-INT): 520, 521. Released (W2-INT): 522 |
| Dialog root | 60160 | Seed-only bark holder, never displayed, no override | In use (W2-INT): 60160 |
| Dialog screen | 200160, 200161, 200162 | One bark line each, under dialog 60160 | In use (W2-INT): 200160. Released (W2-INT): 200161, 200162 |

Collision check, 2026-10-17, at Cellblock `0fca3114f`: a `git grep` across all 740 local branches for these IDs as the first value of a `VALUES (` tuple (chains and triggers under `db/resources/Content`, spawns in `spawnlist.sql`, dialogs and screens under `db/resources/Dialogs`) found zero hits. The same patterns found known rows (chain 1177, spawns 400–499, screens 200000–200003, dialog 60102), so the search works. Dialog 60160 sits in the Cimmeria-authored 60100–60199 range, clear of the bank's 60110–60119 and CB-N05's 60150. The screens are at 200000 or above, clear of the client PAK's screens, which run to 120383. Point-in-time; CB-N04s rechecks before it seals.

### CB-N07 claims — 2026-10-17

Individual claims for the wave 4 barracks escape response (CB-N07a) in the [Cellblock ledger](../cellblock-continuation/w4-packets.md). The chain comes from the 11000–11499 Cellblock authored block. Holder dialog 60160 (CB-N04) is reused, not claimed.

| Namespace | Claimed ID | Exact use | Status |
|---|---|---|---|
| Content chain | 11020 | `enter_region('Castle_Cellblock.Region12')` during 687 step 2355 -> one NID Guard `npc_bark` | In use (W4-INT) |
| Dialog screen | 200170 | The alarm line, under dialog 60160 at `index` 1 | In use (W4-INT) |

Collision check, 2026-10-17, at Cellblock `634b76558`: a `git grep` across all 731 local branches for `VALUES (11020,` under `db/resources/Content` and for a `dialog_screens` tuple with screen 200170 under `db/resources/Dialogs` found zero hits. The same patterns found chain 11010 and screen 200160 on the two branches that carry them, so the search works. Point-in-time; CB-N07s rechecks before it seals, and W4-INT moves the rows to In use.

### CB-N06 claims — 2026-10-17

Individual claims for the wave 5 optional choice *Before the Patrol Returns* (CB-N06, new Cimmeria content under D-CC18) in the [Cellblock ledger](../cellblock-continuation/w5-packets.md), with every literal in [w5-frozen.md](../cellblock-continuation/w5-frozen.md). The chains come from the 11000–11499 Cellblock authored block; the mission IDs follow CB-N05's 62050 pattern. Reused, not claimed: dialog-set-maps 3862 (slot 176) and 7509 (slot 22), item 2893, spawns 520, 521 and 22.

| Namespace | Claimed ID | Exact use | Status |
|---|---|---|---|
| Mission | 62060 | *Before the Patrol Returns*; one-time, two Health Slappacks, no XP | In use (W5-INT) |
| Mission step | 62061 | The choice step | In use (W5-INT) |
| Mission objective | 62062 | Help the prisoners or jam the panel | In use (W5-INT) |
| Mission task | 62063 | One task for the objective | In use (W5-INT) |
| Dialog root | 60151 | One-screen, zero-button, speaker-0 monologue shown on accept | In use (W5-INT) |
| Dialog screen | 96111 | Sole screen of dialog 60151 | In use (W5-INT) |
| Content chain | 11030–11041 | Offer, two login restores, two accepts, follow-up, aid x2, sabotage, abandon, 641 withdrawal, abandon after 641; twelve individual IDs | In use (W5-INT) |

Collision check, 2026-10-17, at Cellblock `12fcfb5db`: a `git grep` across all 744 local branches for these IDs as the first value of a `VALUES (` tuple under `db/resources/Content`, `db/resources/Missions` and `db/resources/Dialogs`, and for screen 96111 anywhere under `db/resources/Dialogs`, found zero hits. The same patterns found chain 11010, mission 62050, dialog 60150 and screen 96110 on this branch, so the search works. Mission 62060–62063 and dialog 60151 are absent from the stock paks (CB-N06-1 and CB-N06-2 prove it in tests). Point-in-time; CB-N06s rechecks before it seals, and W5-INT moves the rows to In use.

### CS-08a individual reservation — 2026-10-14

Owner explicitly approved **chain 1366**, 2026-10-14, reserved exclusively for Castle **CS-08a**, the [Frost's Letter bridge](../castle-continuation/worknotes/CS-08a.md). G-ID is closed for this single claim after collision recheck. The owner's later O14-22 decision delegates the remaining 1367–1400 restoration block to Castle's coordinator for individual audited claims; it does not allocate any particular ID in that block. Recheck live claims immediately before implementation; another packet cannot reuse 1366.

First-tuple resource-seed checks found zero candidate-1366 hits at `origin/main` `19eab88c9`, Castle `3222e6b50`, Cellblock `457b4f75a`, Harset `72ad79648` and Tollana `93eae8cfd`. Content-chain INSERT scans of all four integration worktrees also found zero. Live continuation ledgers contain no individual chain-1366 claim; unrelated mission ID 1366 is a different namespace. This is a point-in-time collision check, not an atomic reservation. O14-22 does not waive the later per-ID recheck for 1367–1400.

### Castle CS-07R-ID template claims — 2026-10-18

Castle claims entity templates **424** (`Castle_HubSupplyOfficer`, the repeatable_q1 hub giver, spawn 13809) and **425** (`Castle_DefenseQ3_Giver`, the defense_q3 giver, spawn 14096). They were first authored as 420 and 421, which the W1 Tollana claim of 2026-10-17 already held, so Castle renumbered ([CS-07R-ID](../castle-continuation/worknotes/CS-07R-id-collision.md)). Each template id is also that NPC's per-player `add_dialog_set` slot. Table-scoped seed scans on 2026-10-18 found no entity template, spawn template or dialog slot 424 or 425 at `origin/main` `dcf736016`, the four-praxis campaign `b28b4fa11`, or the Castle, Cellblock, Harset and Tollana campaign branches. Templates 422 and 423 stay free next to Tollana's 420–421. This is a point-in-time collision check.

### Castle final pass (CS-07a) — 2026-10-18

Castle records decisions D-CA19 (bark and hub sub-ranges) and D-CA20 (per-quest ranges for R4) here, with the ids the branch seeds at `a8753bbe5`. A template id doubles as the NPC's per-player `add_dialog_set` slot. Checked against the Castle worktree seeds; point-in-time, not an atomic reservation.

**Seeded (in use on the Castle branch).**

| Kind | Ids |
|---|---|
| Missions | 10300 (Q1), 10301 (`medical_q2`), 10310 (`defense_q1`), 10312 (`defense_q3`), 10320 (`return_q1`), 10321 (`return_q2`), 10330 (`repeatable_q1`) |
| Chains, 1520 and 1521 | 1367, 1368, 1371–1373, 1378–1383 (`castle_1520_chains.sql`); 1369, 1370, 1374–1377, 1386 (`castle_symbiote_chains.sql`; 1376 is held per D-CS06); 1366 in `castle_carry_in_chains.sql` |
| Chains, 567 Castle leg | 1394–1397, 1400, 10240–10243 (`castle_567_chains.sql`) |
| Chains, hub barks (D-CA19) | 10200, 10201 |
| Chains, repeatable | 10230–10237 |
| Chains, side stories | 10000–10006 (Q1), 10010–10017 (`medical_q2`), 10100–10106 (`defense_q1`), 10120–10129 (`defense_q3`), 10130–10133 (`return_q1`), 10140–10146 (`return_q2`) |
| Dialogs and screens | 10700, 10701, 10702, 10709, 10717, 10718, 10721, 10725, 10729, 10730, 10733, 10734, 10740, 10741 |
| Dialog sets | 10760, 10761, 10762, 10765, 10769, 10771, 10773, 10775, 10776 |
| Dialog set maps | 1000002, 1000003, 1000006, 1000010, 1000012, 1000014, 1000016, 1000017, 1000018 |
| Entity templates | 415 `Castle_Quartermaster`, 416 file cabinet, 417 `Castle_Overlooked_Prisoner`, 419 `Castle_GuardPostSentry`, 424 `Castle_HubSupplyOfficer`, 425 `Castle_DefenseQ3_Giver` |
| Spawns | 13804 (415), 13805 (416), 13806 (417), 13808 (419), 13809 (424), 14096 (425) |

CS-08bS used dialog ids 10733/10734 and screens 10865/10866, which sit outside the D-CA20 allocation, and dialog 2043 instead of 2042; they are recorded here so no later claim reuses them.

**Reserved and unseeded (do not reuse).**

| Item | Reserved ids |
|---|---|
| `medical_q3` (deferred, CD-8) | mission 10302; steps 10348–10353; objectives 10468–10473; tasks 10588–10593; dialogs 10705–10708; sets 10763–10764; screens 10809–10816; buttons 10924–10927; chains 10020–10029; set maps 1000004–1000005; template 418; spawn 13807 |
| `defense_q2` (deferred, CD-8) | mission 10311; steps 10360–10365; objectives 10480–10485; tasks 10600–10605; dialogs 10713–10716; sets 10767–10768; screens 10825–10832; buttons 10932–10935; chains 10110–10119; set maps 1000008–1000009 |
| Hub medic (OD-4) | template 414; spawn 13803 |
| 1521 implant overflow | chains 1387–1393 |
| 567 overflow | chains 10240–10249 (10240–10243 seeded) |
| Hub services | chains 10220–10229 (none seeded) |
| Free next to Tollana | templates 422, 423 |

### FZ-27 claims — 2026-10-19

Individual claims for the mission 1664 step 5226 debrief (four-zone-review FZ-27a/b), made by the architect under the owner's delegation on D-PU3 (decision D-FZ14 in the [four-zone-review decisions](../four-zone-review/decisions.md)). HA-00c nominally owns Harset authored dialog allocation; these are individual claims, not a range reservation.

| Namespace | Claimed IDs | Exact use | Status |
|---|---|---|---|
| Content chain | 6018, 6019 | `interact_tag` report to Col. Marsh and to Moh'katan: bark, drop the cue, `complete_objective` 6061 / 6062 | In use (FZ-27a) |
| Content chain | 6033, 6034 | `player_loaded 'Harset_CmdCenter'` restore of the Marsh / Moh'katan debrief cue | In use (FZ-27a) |
| Dialog set map | 120302, 120303 | NULL-dialog interaction-bit cues on templates 10 and 54 | In use (FZ-27a) |
| Dialog root | 60170 | Seed-only bark holder, never displayed | In use (FZ-27a) |
| Dialog screen | 200180, 200181 | One provisional bark line each, under dialog 60170 (D-FZ15) | In use (FZ-27a) |

Collision check, 2026-10-19: a `git grep` of every local branch and worktree for these IDs as the first value of a `VALUES (` tuple (chains and triggers under `db/resources/Content`, dialog set maps, dialogs and screens under `db/resources/Dialogs`) found zero hits across 892 refs. The same patterns found known rows (chain 6012: 5 hits; screen 200160 and dialog 60160: 46 each), so the search works. Chains 6033–6034 sit in Harset's 6001–6800 block, outside every HA candidate range above (HA-01a ends at 6032). Dialog 60170 sits in the Cimmeria-authored 60100–60199 range, clear of 60150 and 60160. The screens are at 200000 or above, clear of the client PAK's screens, which run to 120383. Point-in-time.

### Non-chain namespaces

Mission, step, objective, dialog, spawn, template, point-set and item IDs are not block-reserved here. Each zone's ID-lock packet (CB-N00, CS-00, HA-00b, T-01) allocates them against the current seed and every other zone's ledger, runs a no-collision check across all seed files and all active campaign branches, and records the result in its zone ledger. For Harset, HA-00b covers restoration proposals and HA-00c covers authored/repeatable allocations. A new authored mission ID must also be absent from the stock `CookedDataMissions.pak`, since CB-N05a inserts it as a new catalog entry.

## Keeping this table true

- Recount before each zone's ID-lock packet and again at FZ-99, and update the "in use" table with the date and commit.
- A claim is a reviewed amendment to this page in the claiming campaign's branch. Two campaigns never claim from the same block.
- An ID that a merged PR has used moves from "reserved" to "in use"; an abandoned claim is released by amendment, never silently reused.
