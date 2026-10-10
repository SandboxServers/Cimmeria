# CellBlock asset audit evidence ledger

> Date: 2026-10-06. Type: reference. Baseline: `3bccbd9789480c56873754a4c00dfebf3c60a3b2`.
> Companions: [campaign](README.md), [packets](work-packets.md).

2026-10-07 follow-up: [knowledge gaps](knowledge-gaps.md) records the proved
Director camera binding, SetViewTarget slot confirmation, activation gates and
per-export recovery of 609 actor records from previously discarded tiles.
Five actor exports and CharacterRimLighting export136 remain parsing gaps.
These findings do not establish a runtime pass or an original rift/blood binding.

## Investigation boundary

This is a static source/package investigation. No client rendering, editor comparison,
live database query, wire capture or in-game UAT was performed. Current seeds establish
Cimmeria behavior, not the final retail implementation. Attached memos and their
`PROVEN` labels are claims until reproduced against their named package exports.

The owner selected restoration through in-game UAT and identified the sibling QA
client and Downloads archives as research inputs. The owner selects original recovery followed by project design for unresolved pieces,
superseding the earlier camera-only compromise as the restoration target. The existing
QA client is the agreed inspection and UAT baseline.

## Corpus and coverage

- `02_CASTLE_CELLBLOCK_MAPS_01.zip`: SHA-256
  `50f9ee55543acadda6d5f579b7755b52e4c24e434acaec927138a13c52712291`.
  Parsed all 66 entries: **65 UMAPs total (64 tiles plus persistent map)** and one
  MapData UPK. The handoff's “65 streamed maps plus persistent” is off by one.
  [Name census](map-name-census.json) records file names and targeted matches.
- QA `CookedPC/Packages`: 361 UPKs successfully parsed; 26 have search-term matches.
  [Package search](package-search.json) records terms, matching export classes/indices,
  package hashes and relevant archive inventories. This is a name/export search,
  not a material-color, texture-pixel or all-reference search.
- `tools/extract_actors.py`: 5,083 actor records returned. Four tiles fail property
  extraction: `00000000`, `fffdfffe`, `fffffffd`, `ffffffff`; each full filename and
  error is recorded by the survey. These tiles are **unresolved coverage**, not empty.
- `tools/kismet_extractor.py`: 65 maps processed, 1,079 nodes, 125 sequences,
  148 extracted chains. Persistent-map export 135 fails with a 16-byte buffer
  bounds error. [Selected graphs](kismet-selected.json) preserve this failure.
- Direct `PackageReader.read_export_properties` on 57 exports beneath the four
  named gameplay sequences succeeded. [Export evidence](sequence-export-evidence.json)
  retains parsed properties and opaque array bytes; opaque bytes are not decoded
  track semantics.
- `SGW_FINAL_DEVELOPER_HANDOFF.zip` contains a 17-file CellBlock dossier: map,
  mission, Kismet, actor-binding, cinematic and asset-use CSVs. Selected rows from
  five tables are preserved in [handoff cross-check](handoff-crosscheck.json).
  Its “520 map packages” spans a broader/repeated corpus and is not the QA map count.
  No file named `cellblock_actor_inventory.csv` was found in the inspected relevant
  archive inventories or repository; the regenerated actor list is a separate source.

Tools: repository `tools/upk_parser.py`, `tools/extract_actors.py`,
`tools/kismet_extractor.py`, and [research wrapper](research_corpus.py).
LZO decompression used `lzallright==0.2.6`. Run the wrapper with the QA CookedPC
directory, supplied archive directory and an output directory. Client/art files stay
outside Git; these outputs contain metadata only. The package scan covers the QA
Packages tree, not all historical archives or uncooked Content packages.

## Findings and implications

| ID | Classification | Reproduced evidence | Campaign implication |
|---|---|---|---|
| E01 | CONFIRMED current source | `db/resources/Worlds/Seed/spawnlist.sql`: spawn 79 is `Cellblock_ArmoryRingSwitch`. `ring_transport_regions.sql`: region 33 uses this tag and mission 688. | Handoff's decoy verdict is stale for Cimmeria. Preserve the exit switch. |
| E02 | CONFIRMED current source | `db/resources/Content/Seed/castle_cellblock_chains.sql`: chain 1107 advances 2356 to 80688 and swaps highlights; no door action. Chain 1109 owns mission completion and ring transport. | Door presentation restoration must retain today's two-step mission/exit contract. |
| E03 | CONFIRMED current source | Chain 1061 displays 3998, completes 641, accepts 680 and highlights rings; no pod action. Original `deprecated/python/cell/missions/Castle_CellBlock/Preparation.py` similarly completes after Livewire without an explicit pod event. | Terminal progression exists; visible pod opening still needs a recovered binding. |
| E04 | CONFIRMED package | `CA-Props.upk` exports 632/633/634 are `Ca-PuddleLarge/Med/Small`; export 38 is `Ca-PuddleWater_FX-Mat`. | Water-puddle substitution lacks blood evidence. |
| E05 | CONFIRMED package | `Em-Props.upk` export 1710 is StaticMesh `EM-ShelfBox13`. `Character/WP-Special.upk` export 5 is StaticMesh `WP-Syringe00`, with material instance export 2. `PFX-abilities.upk` export 67 is BodyComponent `PFX-Syringe_BC`. | Both comparison candidates exist. Syringe also has an ability-related component lead; visual suitability and intended pickup use remain unproved. |
| E06 | CONFIRMED current source | `db/resources/Items/Seed/items.sql`: item 19 is Ambernol Vial with Spray_Injector icon; item 21 is SGHC 6 SMG with `WP-Human.WP_SMG_1A`. Preparation.py grants item 21. | Retain SMG pending contrary primary evidence. “Grant P90” in existing documentation is not proof of the current item visual. |
| E07 | CONFIRMED package, UNRESOLVED linkage | `Character/Straegis-VFX.upk` exports 35142/35143 are ParticleSystem `Str-GroundVoid` and `Str-GroundVoidHold`. | Asset existence is proved; CellBlock scene linkage is not. |
| E08 | CONFIRMED package | `Castle_CellBlock-fffffffe.umap`: export 953 is StraegisAttack sequence; export 281 InterpData duration is 10.0095968246 s; groups 286/287 are Camera/Director, with Move/Director tracks 293/288. SeqVar_Object 988 points to CameraActor export 89. | Existing recovered scene supports a camera move; it does not prove rift/blood choreography. |
| E09 | CONFIRMED cross-check | QA scene package SHA-256 `d00b8261b7d882fd7ff6cf8329c21a347db7f5969175d186989610ebbfa0c4e2` matches the handoff's actor-binding row. SeqVar_Object 988 → camera 89 reproduces its reference. | This particular handoff binding is independently validated. Other rows need individual reproduction. |
| E10 | CONFIRMED source | `db/resources/Events/Seed/sequences.sql`: 1751 maps event 6000 to StraegisAttack; event_sets_sequences.sql links 747 to 1751. Chain 1161 invokes 1751, destroys Marsh immediately and delays 2516 by 10,100 ms. | Do not duplicate the existing cinematic trigger. Investigate view routing and ordering before changing it. |
| E11 | CONFIRMED graph | `fffefffc` StasisBlockDoubleDoors designer event → Interp “Open Stasis Block Double Doors”; LevelLoaded → Interp “Close Stasis Block Double Doors”. | These are block doors, not proof that individual pods open. Preserve the distinction. |
| E12 | CONFIRMED graph | `fffefffd` Prisoner329CellDoor designer event → SetBool → PlaySound → Interp. TakeCoverIndicator designer reveal → ToggleHidden. | Original local state machinery exists; determine event dispatch, object targets and restore semantics. |
| E13 | CONFIRMED names, UNRESOLVED target | Map name tables include Castle fence material and ForceFieldCollisionPlane references in multiple tiles. | Reference presence cannot select the mission barrier or prove visible/collision activation. |
| E14 | BOUNDED NEGATIVE SEARCH | Search terms blood/gore/splatter have no name matches across 361 successfully parsed QA package files. | Blood remains unresolved. This does not exclude red materials, unnamed decals, generic death effects, other builds or server-created effects. |
| E15 | PROJECT DESIGN compatibility | `docs/analysis/class-start-v6/README.md` OD-CS02 supersedes the old two-branch Aftermath reward decision. | This campaign owns crate presentation, not rewriting rewards or ammunition policy. |

## Remaining investigations before implementation

1. Repair or cross-check the four actor extraction failures and persistent Kismet
   failure using existing Rust readers or a verified alternate extraction. Confirm
   coordinate conversion with ring control actors; inspect mesh components, transforms,
   material overrides and collision rather than CSV positions alone.
2. Render ShelfBox and Syringe side by side in the selected editor/client; inspect
   scale, pivot, materials, usage and the actual medical counter. Semantics alone
   cannot identify the original final mesh.
3. Decode pod, fence and door object-variable/Matinee links, and resolve their event
   IDs against the sequence catalog. Establish whether state is per-player, shared
   instance or persistent world state before restoring it.
4. Follow generic decal/material/particle/death-effect references for blood; inspect
   red textures and non-obvious names. Compare historical versions separately with
   hashes and revision provenance. No declaration that art is globally absent yet.
5. Trace `onSequence` view routing only if protocol/docs plus live observation leave
   uncertainty: SGWBeing method 1 is documented in
   `docs/protocol/client-method-dispatch-table.md`; no new method is proposed.
   Ghidra question: how Witness versus EventInvoker selects the local Director camera
   and resolves instance/source/target for sequence 1751. Obtain address-cited findings.
6. Run isolated and two-client UAT for first press, repeat press, mission phase,
   relog, late AoI entry and map stream-out/in. A static graph does not pass these gates.

No runtime changes, client patches, builds, deployment or UAT are claimed by this ledger.

## Subsequent binary pass

The owner opened QA SGW.exe in Ghidra. The [address-cited routing investigation](ghidra-findings.md)
confirms source-entity rejection, distance culling, designer-event dispatch and the
0/3 activation-filter equivalence. This narrows investigation 5; it does not establish
successful cinematic playback or justify a blind view-type change.
