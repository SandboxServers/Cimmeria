# CellBlock → Castle Ring Transport: Audit and Client-Patch Feasibility

**Date:** 2026-09-19
**Status:** Feasibility only. No patcher, seed change or client file has been produced.
**Trigger:** external handoff `01_RING_TRANSPORT_CLAUDE_INSTRUCTIONS.zip` (baseline `f661a97d`), which asks for mission 688's instant `cross_world_teleport` to be replaced with a full Ring Transporter ceremony.

## Summary

The handoff assumes the ceremony can be restored from the server. It cannot. The ring drop, flash and sound are a client-side Matinee inside a Kismet sequence baked into the cooked map, and neither pad has one. The server half is already there: the ring FSM supports cross-world trips, and mission 640 shows the `trigger_transporter` + `teleport_in` chain shape.

What the maps do contain changes the framing. At both pads the original designers placed a complete physical ring station (base platform, console and the top ring) using the exact meshes of the wired rigs. They never instanced the prefab's Kismet. A client patch would finish work the designers started, not invent a ring where none was planned.

A client patch is feasible but it is a tooling project. The repo has no UE3 package writer, and the riskiest steps are rewriting object references inside opaque property blobs and splicing the level's actor list. The recommendation is a gated four-phase plan whose first phase is a cheap go/no-go experiment.

Evidence labels used below: `CONFIRMED` (parsed from the cooked client or read from the binary/docs), `INFERENCE`, `UNRESOLVED`, `PROJECT FINAL` (a deliberate reconstruction, not recovered original content).

## Sources

- Cooked client maps: `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC\Maps\Castle` (146 chunks) and `Maps\Castle_CellBlock` (70 files). The handoff's `02_`/`03_` map zips were hash-compared against this tree: 210 of 210 `.umap` files are byte-identical. `CONFIRMED`
- Tools: `extract_actors`, `extract_kismet`, `upk_info`, `inspect-export` from `crates/upk` and `crates/upk-objects`. This investigation added `upk_info --imports` / `--names` and fixed `inspect-export --props` for component exports (see [Tool changes](#tool-changes)).
- Coordinate transform: game `(x, y, z)` = UE `(Y, Z, X) / 100`. Derived from the wired region 1 rig: UE `(-12142.3, -21534.6, 6554.1)` against seeded region 1 `(-215.455, 65.918, -121.4)`. `CONFIRMED`
- Client behaviour: [cinematic-system.md](../../gameplay/cinematic-system.md), [ring-transport-system.md](../../gameplay/ring-transport-system.md), [cooked-data-pipeline.md](../../reverse-engineering/findings/cooked-data-pipeline.md), [black-market-client-window-patch.md](../../reverse-engineering/findings/black-market-client-window-patch.md), [ue3-package-format.md](../../engine/ue3-package-format.md), plus Ghidra queries against `SGW.exe`.

## Audit findings

### Current server state

| Item | State | Label |
|---|---|---|
| Chain 1109 | `interact_tag Cellblock_ArmoryRingSwitch` + `step_status 688/80688 active` → `complete_mission 688`, clear highlight, `cross_world_teleport Castle (466.365, 70.397, 991.466)`. Bypasses the ring FSM. | `CONFIRMED` |
| Region 33 | World 12, centred on spawn 79 (the console) at `(-54.880, 26.080, -163.840)`, event set 10000, destination `{34}`, `required_mission_id=688`. | `CONFIRMED` |
| Region 34 | World 8, `(466.365, 70.397, 991.466)`, event set 10000, inbound-only. | `CONFIRMED` |
| Event set 10000 | Sequences 10015/10016 → `Castle_Cellblock-fffefffd…GLB-RingTransporterBase_TC00_Pf0_Seq_0`. That is region 1's rig, about 160 m from the Armory. Firing it for region 33 or 34 would animate an unseen ring. | `CONFIRMED` |
| Chain 1109's comment ("nothing to animate on this route") | Correct as to Kismet. Wrong where it calls the platforms "set-decorated geometry": they are real ring stations, see below. | `CONFIRMED` |

### What the maps contain

| Site | Chunk | Ring | Base platform | Console | Kismet |
|---|---|---|---|---|---|
| Region 1 (wired) | `Castle_CellBlock-fffefffd` | InterpActors 334–338 → `GLB-Global.GLB-RingTransporter00` (five rings) | StaticMeshActor 1463 → `GLB-RingTransporterBase_TC00`, UE z 6538.1 | StaticMeshActor 1381 → `TC-Props.TC-Ring_Trans_Console00` | Sequence 1169, prefab-instanced |
| Region 3 (wired) | `Castle_CellBlock-fffeffff` | InterpActors incl. 220/227/228 | StaticMeshActor 1192 | StaticMeshActor 1194 | Sequence 772, **de-prefabbed** |
| **Armory pad (un-wired)** | `Castle_CellBlock-fffeffff` | InterpActor 226 → `GLB-RingTransporter00` (one ring) | StaticMeshActor 1174 → `GLB-RingTransporterBase_TC00`, UE z 2442 | StaticMeshActor 1178 → `TC-Ring_Trans_Console00` | **none** |
| **Castle pad (un-wired)** | `Castle-00090004` | InterpActor 77 → `GLB-RingTransporter00` (one ring) | StaticMeshActor 433 → `GLB-RingTransporterBase_TC00`, UE z 6986 | StaticMeshActor 434 → `TC-Ring_Trans_Console00` | **none** |

All rows `CONFIRMED`.

- CellBlock holds exactly three ring sequences (chunks `fffefffd`, `fffefffe`, `fffeffff`), matching regions 1–3 and event sets 10000 / 874 / 875.
- Castle holds no `SeqEvent_RegionTeleport` and no ring sequence in any of its 146 chunks (zero parse errors). The handoff's raw string scan was unreliable because chunks are LZO-compressed; this result comes from parsed export tables.
- At both un-wired pads only the top ring of the stack is placed, at `base_z + 20` UU, the same offset as the top ring of the wired rig. The remaining four rings, the emitter and the sequence would have come from instancing the prefab.
- The source prefab survives cooking: `CookedPC\Packages\GLB-Global.upk` (uncompressed) contains `Prefab` export 3243 `GLB-RingTransporterBase_TC00_Pf0` with its archetype actors and template `Sequence`.

### Region placement

The wired region 1 sits 0.537 m above its base platform origin. Applying the same offset to the un-wired pads:

| Region | Seeded | Pad-derived | Delta |
|---|---|---|---|
| 33 | `(-54.880, 26.080, -163.840)` | `(-53.602, 24.957, -166.596)` | about 3.2 m; the seed is on the console, not the pad |
| 34 | `(466.365, 70.397, 991.466)` | `(466.451, 70.397, 991.552)` | 0.12 m; the in-game HUD pin was accurate |

Pad-derived values are `INFERENCE` from `CONFIRMED` actor positions. Region 33 should move regardless of which option below is chosen.

### How the client plays a ring sequence

- The server sends only an integer sequence id (`onSequence`). The client maps it to a Kismet script name through its cached copy of the `sequences` table (cooked-data category 1), which the server supplies. New sequence rows are therefore a server-side change. `CONFIRMED`
- The name is then resolved to a live object by the engine. The consuming function was not traced; `UObject::StaticLoadObject` is present at `0x004a8e10`, `[Engine.StartupPackages]` lists no packages, and no `KIS-` literal exists in the binary, so resolution is most likely stock UE3 find-or-load by object path. `INFERENCE`
- An unresolvable name is probably a silent no-op, as in neighbouring lookup paths. Not verified for this call site. `UNRESOLVED`
- `USeqEvent_RegionTeleport` (`0x0069fc40`) carries no region or player data. Its filter at `0x006a09e0` fires when `sequenceEventType == teleportDirection + 8000`. All region logic is server-side, so a cloned rig needs no per-region client data. `CONFIRMED`
- The engine checks only package version on load (`"Package '%s' version mismatch"`). No content hash, signature or server GUID comparison was found. File integrity lives in the launcher's SHA-256 + Ed25519 manifest, which is also the existing channel for shipping client changes. `CONFIRMED`

## Options

| | Option | Delivers the ring animation | Client change | Verdict |
|---|---|---|---|---|
| A | Clone a rig into the two cooked chunks | Yes, identical to regions 1–3 | Two modified `.umap` files via launcher overlay | **Recommended**, gated |
| B | New standalone Kismet package (`KIS-*` style) | No. A standalone sequence cannot bind the pad's level actors, so at best particles and sound on the player | One added `.upk` | Rejected for this goal |
| C | Runtime patch (Lua or native hook spawns and moves ring actors) | In principle | Per-launch memory patch | Rejected: no documented surface spawns actors or plays a Matinee; all new RE |
| D | Server-only: route 1109 through the ring FSM with empty map-local event sets | No visuals. Gains movement lock, hide/show, party passengers, timeout release, and mission 688 completing on arrival instead of before the trip | None | Worth doing regardless; it is also phase 3 of option A |

## Option A in detail

### What has to be cloned

Use region 3's rig in `fffeffff` (sequence 772) as the source, not region 1's. It is de-prefabbed: its InterpActors carry explicit `StaticMesh` properties, and it needs no `Prefab`, `PrefabInstance` or archetype imports. That removes the whole prefab-chain import problem (16 imports in the `fffefffd` rig). `CONFIRMED`

For scale, the prefab-instanced `fffefffd` rig closes over 48 exports / 59,752 serial bytes / about 45 imports. The de-prefabbed shape is about 34 exports. Audio is an FMOD event string (`prp_gen/rings/transport`), not an import. `CONFIRMED`

Import cost per target:

| Target | Imports already present | To add |
|---|---|---|
| `Castle_CellBlock-fffeffff` (Armory pad) | All of them. Source and target are the same package. | 0 |
| `Castle-00090004` (Castle pad) | 14, including all three meshes | about 31: the four SGW Kismet classes (they live in `Engine`), `SeqAct_Toggle`, `InterpGroupDirector`, `InterpTrackEvent`, `Emitter`, `ParticleSystemComponent`, `GLB-VFX.Par-ring05` and outers |

Both targets already have a `Main_Sequence.Prefabs` sub-sequence to attach to (`fffeffff` 763/764, `Castle-00090004` 272/273). `CONFIRMED`

One item is easier than expected: `InterpTrackMove.MoveFrame = IMF_RelativeToInitial` with a zero Euler track, so the 3,419-byte position tracks copy verbatim and only the actors' `Location` vectors need rewriting. `CONFIRMED`

### Work items

| # | Item | Risk |
|---|---|---|
| 1 | Package writer: serialize header, name, import, export and depends tables plus export data. `crates/upk` is read-only today; `crates/navmesh-extractor/src/test_support/package_bytes.rs` is a test-only skeleton. The `tools/ue3_*.py` round-trip scripts that [ue3-package-format.md](../../engine/ue3-package-format.md) mentions are no longer in the repo. | Medium |
| 2 | Name-table merge (no-op for the Armory pad). | Low |
| 3 | Import-table merge with outer chains (no-op for the Armory pad). | Medium |
| 4 | Export append with object-reference remap. References inside `ArrayProperty` / `StructProperty` blobs (`SequenceObjects`, `InterpGroups`, Kismet `Links`, `LinkedVariables`, `Targets`) are returned as opaque bytes by the current parser. A missed reference is silent corruption. | **High** |
| 5 | Actor `Location` / `Rotation` rewrite. Reuse the already-placed top ring (exports 226 / 77) as one of the five. | Low |
| 6 | Level actor-list splice. The list is a post-property binary `TArray` inside the `Level` export; locating it needs a real `ULevel::Serialize` walk, which does not exist yet. | **High** |
| 7 | `Prefabs.SequenceObjects` splice and `ParentSequence` fix-up. | Low |
| 8 | Header recompute: every downstream `serial_offset`, depends table, `generations[]`, `total_header_size` (ends at the depends table, a documented trap). | Medium |
| 9 | Emit uncompressed first; LZO repack only if the client rejects it. All shipped `.upk` packages are uncompressed, but every shipped `.umap` is LZO and an uncompressed map has never been tested. | Medium |
| 10 | Decode how `SeqEvent_RegionTeleport` and `SeqEvent_Console` bind to the level (originator / `Targets`). Must be understood before costing item 4. | `UNRESOLVED` |

### Phased plan

| Phase | Work | Exit gate |
|---|---|---|
| 0 | Writer round-trip. Read `Castle-00090004`, write it back uncompressed with no content change, load it in the client. Also decode item 10 and trace the `onSequence` consumer from `register_NetIn_onSequence @ 0x00d76f40`. | Client loads the rewritten chunk and the pad looks unchanged. **If this fails, stop and ship option D.** |
| 1 | Same-package clone: region 3's rig onto the Armory pad inside `fffeffff`. Items 2 and 3 are no-ops, isolating the two high-risk items. | A GM-triggered sequence animates the Armory ring in-game. |
| 2 | Cross-package clone into `Castle-00090004`. | The Castle ring animates in-game. |
| 3 | Server seed (option D plus sequences): four new `sequences` rows (8000/8001 per pad), two new map-local event sets, regions 33/34 repointed and region 33 re-centred, chain 1109 → `trigger_transporter {"regionId": 33}`, new `teleport_in '34'` chain completing 688 (the mission-640 / chain-1044 shape). Tests per the handoff's list, including "failed trip does not complete 688". | Full flow UAT, plus regions 1→2, 2→3, Omega 14↔17 and Harset regression. |
| 4 | Distribution: two patched chunks as a launcher overlay zip under the signed manifest. | Unpatched client verified to degrade to option D behaviour, not crash. |

Phase 3 does not depend on phases 0–2 except for the sequence rows, so option D can ship first and the visuals can follow.

Everything produced by phases 1–2 is `PROJECT FINAL`: a reconstruction using original assets and an original sequence, completing a station the designers placed but did not wire. It is not recovered original Kismet.

## Open questions

| # | Question | How to close it |
|---|---|---|
| 1 | Does the client load an uncompressed `.umap`? | Phase 0 experiment. |
| 2 | Which function consumes the resolved script name, and what happens when it fails to resolve? | Decompile forward from the `onSequence` subscriber (data xref `0x019c7f44`). Decides how unpatched clients behave. |
| 3 | What do `SeqEvent_RegionTeleport` / `SeqEvent_Console` bind to in the level? | Decode `Targets` / originator in sequence 772. |
| 4 | Exact end of the `ULevel` property block and start of the actor array. | Implement `ULevel::Serialize` for Epic 486. |
| 5 | Licensee version: [ue3-package-format.md](../../engine/ue3-package-format.md) says 6, `upk_info` reports 8 on these chunks. | Check the header parse against a hex dump; fix whichever is wrong. |
| 6 | The rigs' designer comment "event switched due to code bug". | Low priority. A clone inherits the shipped workaround unchanged. |

## Tool changes

Made during this investigation, uncommitted in the worktree that holds this document. `cargo fmt` and `cargo clippy` clean.

- `crates/upk/src/package.rs`: `Package::import_full_path()` and `Package::resolve_object_path()`.
- `crates/upk/src/bin/upk_info.rs`: `--imports`, `--names`; `--exports` now prints ref value, archetype and outer.
- `crates/upk-objects/src/bin/inspect_export.rs`: `--props` probed offsets `[0, 4, 32]` and kept the first non-empty parse, so component exports (8-byte prefix) yielded one garbage property. It now probes `[0, 4, 8, 12, 32]`, keeps the parse with the most properties, prints resolved object paths, and accepts `--prop-offset N`.
