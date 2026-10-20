# CS-00Ra worknote: G-MED archaeology

> **Packet:** CS-00Ra. **Agent:** game-archaeology-specialist. **Date:** 2026-10-17.
> **Base:** `c481008904ba39c80168424f67d96b9b271b9ac9`. The base check passed: HEAD equals the merge-base and `FEATURE_SHA`.
> **Output:** [evidence/g-med.md](../evidence/g-med.md).

## Sources read

- **Branch ledger:** `docs/analysis/castle-continuation/work-packets.md`, covering § CS-00Ra, the dispatch, repo and base-check rules, the 2026-10-14 stock prop supplement, the CS-02/03 graph candidates and the CS-01 boundary.
- **Prior evidence:** `evidence/cooked-interaction-maps.md`, `evidence/reconciliation.md`, `evidence/seed-excerpts.md` and `evidence/dialog-excerpts.md`.
- **Seeds:**
  - `db/resources/Texts/Seed/texts.sql:57107–57109` (monikers 8896/8897);
  - `db/resources/Items/Seed/items.sql:2509–2511`;
  - `db/resources/Entities/Seed/entity_templates.sql` (templates 147 and 173);
  - `db/resources/Dialogs/Seed/dialogs.sql:8329–8335`;
  - the Castle `start_minigame` Livewire precedents in chains 1024, 1233, 1292 and 1356.
- **Docs:**
  - `docs/gameplay/minigame-system.md:71–76, 159–160`;
  - `docs/content/interaction-flags.md`;
  - `docs/content/dialog-ui-client-contract.md:30, 38, 102–118`;
  - `entities/defs/enumerations.xml:754–759`.
- **Client (QA install, read-only):**
  - `CookedPC\Maps\Castle`: all 146 packages, decompressed with `tools/upk_parser.py`;
  - `SourceCache.en-us\CookedDataDialogs.pak` (`_4982`–`_4985`);
  - `SourceCache.en-us\CookedInteractionSet.pak` (set 1571);
  - `Content\UI\Core\Dialog\Dialog.lua:86` and `PushMission.lua:3`, for the radio queue.
- **Tools:**
  - `tools/extract_actors.py --format json`. It gives 14,265 Castle actors, but no mesh names are resolved, so no placement join was possible.
  - Scratch scripts (not committed): `scan_castle2.py`, `scan_names.py` and `idcheck.py`, run against the four refs listed in g-med.md Q5. The `idcheck.py` output is not reproducible from the repo and is no longer quoted; the `git grep` commands in g-med.md Q5 are the record.

## Dead ends

- A raw byte scan of the Castle packages finds nothing, because the package bodies are LZO-compressed. Decompress the 146 packages (145 `.umap`, 1 `.upk`) with `PackageReader` first.
- Name scans of the `SeqEvent`/`SetEvent` paks for 8896/8897 were already negative in `cooked-interaction-maps.md`. They were not rerun.
- `extract_actors` JSON leaves `PrefabInstance` names unresolved and carries no `StaticMesh` references, so it cannot place the E10 medical props.

## Open questions for CS-02b2's split

1. **Placement (G-MED placement).** Is the dispenser coordinate taken from a decoded `StaticMeshActor` join (E10 meshes inside the Infirmary bounds) or from a lab placement check? CS-02b2 must not ship spawn 13801 until one of these is signed.
2. **Mesh.** Should template 412 use `EM-MedicalBox00` or `EM-LockerMed00`? A lab render check is needed: either the object is visible, or the template falls back to an invisible `WorldObject_Small` with the glow only.
3. **Interaction mask.** Should the spawned entity carry bit 30 (`INT_MissionWorldObject`, as shipped in map 5828) together with bit 8 (Livewire 256), or only 256 while step 4453 is active? Map 5828 argues for keeping bit 30.
4. **Identity gate.** The victory callback (chain 1372) must bind to the CS-02b2 per-player identity gate, so that a `PlaceholderGame` or forged victory cannot advance 4453.
5. **Radio timing (G-MED radio).** Should 4985 follow the 4984 close directly, or a server delay? It must be immediate type 2 either way, and never a lure (E5).
6. **Dialog 4982.** Is it a 704-completion cue or the 1520 offer text? This stays outside CS-02b2 unless the coordinator folds it in.
7. **ID claims.** Template 412 and spawn 13801 are free on all four refs. Chains 1371–1373 are free as chain rows on all four refs but are template IDs in debug-area files (g-med.md Q5). Ra keeps chains 1371–1373 and CS-00Rb takes 1374–1377. The coordinator records them in `id-allocation.md` and rechecks them at integration.

## Budget note

The packet passed its ~50k-token budget during research. The rows are sealed as proven or `UNRESOLVED (<gate>)`. The remaining gates (placement, game and radio) need new evidence, not more desk work.
