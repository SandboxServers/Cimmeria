# Cellblock source and route matrix

> Audit: 2026-10-14; repository baseline `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
> Scope: static source reconciliation. All fresh client response rows are UATPending.

## Source tiers and confidence

| Tier | What it establishes | What it cannot establish |
|---|---|---|
| H — historical handoff CSV | Exact supplied row, named build where present, claimed relation and hash | Missing upstream extraction/database lineage; installed-client response |
| P — package evidence | Existing address/export-cited asset-audit findings; archive bytes verified here | Gameplay linkage or playback merely from asset names |
| L — legacy emulator | Giza/Atrea intended orchestration in deprecated/python | Original CME server truth or current behavior |
| S — current seed/code | Defined Cimmeria chain, mission, spawn and test assertions at this commit | Deployed database parity or passing client UAT |
| T — prior telemetry | The event recorded in the dated session | Delivered first-press feedback, visual/audio acceptance, current build parity |
| C — prior client observation | Only the specific dated observation reported by its source | Whole-route acceptance or present behavior after intervening changes |

High confidence below means the cited row exists, not that its historical lineage
or client playback is sealed. `RESTORED_SOURCE` applies only to an exact supported
historical beat; missing wiring/placement is `CIMMERIA_RECONSTRUCTION`. New story is
`CIMMERIA_AUTHORING`. Current seeds and deprecated scripts are emulator data,
not the original server database ([source rules](../../agents/rules-and-gotchas.md)).

## Audited corpus

The ZIP was read directly; the separate extracted Downloads folder was only a
navigation aid. No named SQLite database is inside the developer ZIP. Its absence
is not a blocker for the dispositions below. Mission CSV rows have **no Build
column**: their build is UNSEALED, not silently borrowed from a map row.

| Archive | SHA-256 computed in this audit |
|---|---|
| SGW_FINAL_DEVELOPER_HANDOFF.zip | `0317e0d4445f1c851bb60ee910fc8ea77b5661bfd589d315015dfcadc1cfb28a` |
| SGW_Historical_CellBlocks_SERVER_DEV_7VERSIONS_ENGLISH.zip | `ba7dc870557e4860c9240017063ddc1a5275566426bb0e92ce288573e1dca172` |
| SGW_Historical_CellBlocks_CLIENT_7VERSIONS_FINAL.zip | `36980aef098a72d3dc0fae391a51a6c607079d52086f49828b820e21e7469d06` |

All following CSV names are relative to developer ZIP
`WORLDS/RECOVERED/Castle_CellBlock/`. A row key is preferred to a physical line
number because quoted CSV fields can contain line breaks.

## Mission and room dispositions

S anchors below are in
[castle_cellblock_chains.sql](../../../db/resources/Content/Seed/castle_cellblock_chains.sql)
and [spawnlist.sql](../../../db/resources/Worlds/Seed/spawnlist.sql).
H row `M:n` means `02_MISSIONS.csv`, `MissionID=n`, relation
`PROVEN_MISSION_MONIKER_WORLD_EXACT`, build UNSEALED. That relation proves only
moniker-to-world association. L anchors are the matching mission sections in
[mission-chains](../../content/mission-chains.md) and
[Castle_CellBlock.py](../../../deprecated/python/cell/spaces/Castle_CellBlock.py).
Tests listed are existing files under
[chain_replay_tests](../../../crates/cell-content/src/cell/content/chain_replay_tests);
they were inspected, **not run** in this docs-only packet.

| Mission / room / action | H/L anchor and provenance disposition | Current tag / chain anchors (S) | Intended response / existing assertion surface | Confidence / owner |
|---|---|---|---|---|
| 622 stasis / search bodies, equip | M:622; legacy ArmYourself. Historical corpse-search goal; added 80623/80622 steps and dialog 3996 are reconstruction. | ArmYourself_FrostBody, ArmYourself_GuardBody; 1001–1007, 1010; Region8 guard 1008/1009 | Letter/pistol grant, equip completes and opens stasis door; mission_622.rs, mission_622_core_tutorial.rs. Fresh visibility/fire UATPending. | High S; N02, class-start owns ability split |
| 638 cells / release prisoner | M:638; legacy Prisoner_329; historical goal, current routing S/L | Prisoner_329, 329_CellDoorButton; 1011–1021 | Branch dialog, Livewire, advance 2115→2116, accept 639; mission_638.rs. Door motion UATPending. | High S; N02 + asset owner |
| 639 med station / vial, cover, drone, cure | M:639; legacy FindAmbernol; historical objective texts, current AND gate reconstruction | ArmYourself_AmbernolVial, ArmYourself_PrisonerRetrievalUnit; 1031–1034, 1112, 1131–1133 | Both cover 2484 and kill 2482 before 2343; item 19 completes; mission_639.rs and mission_639_cover.rs. | High S; N02/N03; prop visual asset owner |
| 640 cells / hack and ride | M:640; legacy HackRings; current restoration/reconstruction wiring | HackTheRings_Switch; 1041–1046, 1151 | Livewire then ring travel; relog bits by step; mission_640.rs. Native click/arrival not newly tested. | High S; N02 + ring owner |
| 641 Preparation / gear, Marsh, terminal | M:641; legacy Preparation; current equip-step insertion reconstruction | Preparation_ColMarsh, Preparation_SMG1A, Preparation_Terminal; 1051–1066, 1152 | Gear/equip, dialog, Livewire completes 641/accepts 680; mission_641.rs. Pod opening is not proved by completion. | High S; N02; class-start/asset owners |
| 680 Preparation to topside / escort | M:680; legacy EscapeTheCellblock; current escort/barks reconstructed | Preparation_RingSwitch, Preparation_ColMarsh; 1071–1074, 1171, 1173–1176 | Ring 2→3, follow, Region9 accepts 681; mission_680.rs. Current follow/first bark delivery UATPending. | High S; N02 + NPC/ring owners |
| 681 Mess Hall / guards and flank | M:681; H says visible; S is hidden. Current tracked-not-gating flank is project decision. | MessHall_Guard1/2; 1081, 1085–1087, 1141, 1177, 1181–1183 | Kill-only completion; flank 2725 credit if reachable; mission_681_686_flank.rs, backstop/hidden siblings. | High S; N03; historical flag differs |
| 682 Hallway01 / guard | Absent H mission CSV; L/S controller, not evidence of missing gameplay | Hallway01_Guard; 1088, 1184 | Hidden controller completes on kill or pre-dead backstop; mission_681_686.rs/backstop. | High S; N02/N03 |
| 683 Hallway02 / guard | Absent H; L/S controller | Hallway02_Guard; 1089, 1185; bridge 1082 | Hidden controller completes and advances 684; same replay family. | High S; N02/N03 |
| 684 Hallway03 / guard | M:684; H visible vs S hidden | Hallway03_Guard; 1090, 1186 | Kill advances 685; same replay family. | High S; N02/N03 |
| 685 Hallway04 / guard | Absent H; L/S controller | Hallway04_Guard; 1091, 1187; bridge 1083 | Kill advances 686; same replay family. | High S; N02/N03 |
| 686 Hallway05 / flank and Straegis aftermath | M:686; H visible vs S hidden. Camera export supports scene; Marsh disappearance/dialog timing are current wiring. | Hallway05_Guard1/2; 1092–1094, 1142, 1178, 1188–1190; scene 1161/1162, 1172/1175 | Kill-only progress; sequence 1751; dialog 2516 at 10100ms, 5859 at 26100ms; mission_686_straegis.rs and flank/backstop/hidden tests. Playback UATPending. | High S; N02/N03, asset owner |
| 687 barracks / crate and guards | M:687; historical Human/Jaffa goal; five-way class rewards explicitly project design | Cellblock_WoodenCrate, Barracks_Guard1/2/3; 1097–1104, 1191–1195 | Loot/equip resources and kill progression; mission_687.rs, mission_687_crate.rs. | High S; class-start owns reward split |
| 688 armory / terminal then exit | M:688; optional guard/current extra step 80688 and cross-world fallback reconstruction | Cellblock_TerminalX, Cellblock_ArmoryGuard1, Cellblock_ArmoryRingSwitch; 1105–1111, 1154 | Terminal advances, ring completes/teleports; guard 4647 optional; mission_688.rs. Does not prove door/ring animation. | High S; N02 boundary; ring owns 1109 |
| 689 boots / unlock | Absent H; internal Cimmeria lock controller, not a recovered mission catalog row | Item 3438, abilities 1597/1598; 1022–1025 | Accept/apply lock on load, Livewire on use, clear on victory; mission_689.rs, mission_689_hidden_frames.rs. | High S; N02; new client lock run pending |
| 1360 optional / Frost's Letter | Absent H; historical letter item 3730; accepting mission on corpse search is Cimmeria reconstruction per C04/D-CB03 | ArmYourself_FrostBody; 1121 alongside 1003; step 4037 | Offer once and carry state; mission_1360.rs. Castle 4037→4038 leg belongs to CS-08, not a presumed completed handoff. | High S; Cellblock acceptance / Castle CS-08 / Harset recipient |
| 642 archival / Escape the Cells | Absent H; hidden skeleton in mission-chains; no activation inferred | missions.sql 642; no route chain assigned | Keep archival; no implementation or client claim. | High seed existence; archaeology owns hold |
| 1654 off-route / Aftermath | M:1654 is level 5, SGC/Hammond objectives; moniker duplicates 687 | missions.sql 1654; no Cellblock route chain assigned | Exclude from escape; a compatible later arc needs separate source review. | High H/S disposition; offworld mission owner, not N02 |

None of these rows assigns recovered provenance to every action in a current chain.
Any future implementation seal must classify each changed beat separately and obtain
mission/NPC/items/presentation approval as applicable.

## Historical comparison worlds

`worlds.sql` 1201–1207, `entities/spaces.xml` and
`crates/resources/src/base/world_info_overrides.rs` agree on the aliases below.
They are instanced archaeology spaces with `has_script=false`; no import into 12.
The developer CSV reports 65 map rows per build (520 across seven builds + QA4046).
Its FULL_SNAPSHOT_EXACT label and the historical bundle's VPatch derivation are
separate claims: exact final bytes can be patched, but no missing lineage is inferred.

| Build / world / alias | Reported derivation | H original root SHA-256 | Computed aliased root SHA-256 |
|---|---|---|---|
| 43485 / 1201 / CellBlock43 / C43485_CellBlock | Full build (handoff claim) | `b5bbfd8f48b219a31c61ce6a34e7de602fb255272e8fe1aff18821027be373ea` | `90327c3848b7e584d3f3cc17ff60f5cff68838216285a4d6fdab069f9fbb7482` |
| 55124 / 1202 / CellBlock55 / C55124_CellBlock | Full build (handoff claim) | `fa6d3ad7d41f6ec7799b87425c3fea139331656c0a56d42fa859a0ebe9356a6b` | `fd6eddd3f9bd7c988158542cb1d6c558258366ea4f5a88dd9eb08792b743ea2a` |
| 57050 / 1203 / CellBlock57 / C57050_CellBlock | Full build (handoff claim) | `fa6d3ad7d41f6ec7799b87425c3fea139331656c0a56d42fa859a0ebe9356a6b` | `2077567c31b80c19cd79b7a9781f8304d85d8bd6a5dcb63d774e7f96e14a7f40` |
| 58674 / 1204 / CellBlock58 / C58674_CellBlock | Full build (handoff claim) | `fa6d3ad7d41f6ec7799b87425c3fea139331656c0a56d42fa859a0ebe9356a6b` | `4e7cae150a7c336e02dbd5976bb5c767dc968ebbb1ab303ed6152b8739e99a55` |
| 60130 / 1205 / CellBlock60 / C60130_CellBlock | VPatch 58674→60130 | `fa6d3ad7d41f6ec7799b87425c3fea139331656c0a56d42fa859a0ebe9356a6b` | `c42a2a6b832366bf15349b0cbe93090561cb3ef0afa604983d5d34e786b07a38` |
| 62429 / 1206 / CellBlock62 / C62429_CellBlock | VPatch 60130→62429 | `9b72f927e443696c2a1b2a31636d6dfac3ce8c97e4db00621666841766b22eff` | `b064f45a366eaa5d06fa112a0df90fa4bbee9511ff0a0230abfd60a85525b439` |
| 63682 / 1207 / CellBlock63 / C63682_CellBlock | VPatch 62429→63682 | `e8c71aad68d11b2972002b65c0ba7338064f74a51106a0a6d6e14dc478f36e1f` | `727faae355c237a6b0c72a0fd0ba2aca202e39dc6058069a89183535d6001834` |

The H root key is `01_MAP_PACKAGES.csv` `(Build, MapRelativeTail=Castle_CellBlock.umap)`.
Aliased root key is historical client ZIP `PAYLOAD/SGWGame/CookedPC/Maps/<alias>/<alias>.umap`.
Different output hashes are expected after alias replacement; they are not by themselves
proof of a different original layout. 55124/57050/58674/60130 share the reported original
persistent-map hash; this does **not** establish equality of their streamed tiles.

`CLIENT_MAP_MANIFEST.csv` has 262 rows: 65/65/66/66 for the first four builds.
All 262 OutputFile payload hashes match OutputSHA256 (verified here). The three later
roots match `NEW_BUILD_VALIDATION.json`; their per-file source joins are UNSEALED.
`PATCH_RECONSTRUCTION_REPORT.csv` lists file before/after MD5 and patched status but
has no build column, so do not assign each repeated filename to a transition by order.
No raw CAB/VPatch inputs were replayed. To seal lineage, obtain source inputs and
build-qualified patch manifests, reproduce each transform, then compare exact actor,
cover and mission bindings against QA. Geometry, collision, safe pockets and presentation
for all seven aliases remain UNSEALED/UATPending. No maps were installed or launched.

## Remaining ZIP relation tables

Every mission row is disposed above. Other tables are inventories, not implied
implementation tickets; the following dispositions apply to every row in each table.

| CSV | Rows | Tier / disposition / owner |
|---|---:|---|
| `01_MAP_PACKAGES.csv` | 520 | H; 520 build-qualified hashes retained in source; map compatibility UNSEALED; archaeology/asset owner |
| `02_MISSIONS.csv` | 12 | H; all 12 mission IDs individually disposed above; N00 |
| `03_DIALOG_EVENTSET_SCRIPT.csv` | 0 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `04_KISMET_EVENTSET_ACTORS.csv` | 8 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `05_RUNTIME_OBJECT_WORLD_BINDINGS.csv` | 256 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `06_CINEMATIC_RUNTIME_PATHS.csv` | 240 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `07_CINEMATIC_WORLD_BINDINGS.csv` | 751 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `08_DIALOG_VO_SAMPLE_CANDIDATES.csv` | 105 | H candidate relation; VO/audio/UI existence or naming is not mission delivery; asset owner |
| `09_WORLD_AUDIO_CANDIDATES.csv` | 84 | H candidate relation; VO/audio/UI existence or naming is not mission delivery; asset owner |
| `10_UI_MINIMAP_CANDIDATES.csv` | 0 | H candidate relation; VO/audio/UI existence or naming is not mission delivery; asset owner |
| `11_RUNTIME_ASSET_USAGE.csv` | 252 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `12_PLAYER_STARTS.csv` | 0 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `13_TRAVEL_TRANSPORT.csv` | 224 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `14_TRIGGER_ACCESS.csv` | 207 | H indexed relation; no current behavior inferred, exact build/export join required before reuse; N01/asset or ring owner |
| `15_SPAWN_ENCOUNTER_ROOTS.csv` | 7 | H; seven Matinee roots R00042–R00048, EntitySpawnProven=False; asset owner, no NPC grant |
| `16_SPAWN_ENCOUNTER_BINDINGS.csv` | 1 | H/P; QA4046 R00048 variable 988 → CameraActor 89; asset owner, not server spawn |

The one independently corroborated camera binding is documented by
[asset ledger E08–E10](../cellblock-asset-audit/evidence-ledger.md):
QA `Castle_CellBlock-fffffffe.umap` SHA-256
`d00b8261b7d882fd7ff6cf8329c21a347db7f5969175d186989610ebbfa0c4e2`,
sequence export 953, InterpData 281 (10.0095968246s), variable 988 → camera 89.
This audit reuses that static finding; it does not claim a new Ghidra trace or render.
The root keys are `(55124,R00042)`, `(57050,R00043)`, `(58674,R00044)`,
`(60130,R00045)`, `(62429,R00046)`, `(63682,R00047)`, `(QA4046,R00048)`;
43485 has no row in this root table (not proof of no scene in that build).
No combat spawn follows from the seven roots' ENCOUNTER_COMBAT label: their
RuntimeInterpretation is explicitly SCRIPTED_ATTACK_MATINEE_NOT_COMBAT_AI_PROOF.

## Unsealed row closure

- **H-MISSION-BUILD:** all M rows lack build provenance. Obtain upstream extraction
  inputs and build-qualified catalog identifiers; do not borrow map build labels.
- **H-PATCH-LINEAGE:** later build joins need build-qualified patch/input checks above.
- **P-PLAYBACK:** first/repeat/relog/late-entry pod/door/barrier/Straegis and ring
  acceptance requires CB-N01's sibling handoff plus client/server evidence.
- **S-PLACEMENT/COPY:** every proposed added actor position, heading and dialogue
  remains UNSEALED until owner and domain seals plus physical UAT.
- **T-CURRENT:** historical session success does not close #1667 or prove today's
  route; use the bounded baseline rows in [route-baseline](route-baseline.md).
