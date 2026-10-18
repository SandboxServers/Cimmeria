---
title: "UE3 editor co-op: opportunity analysis"
type: analysis
audience: owner, tools engineers, map authors
last_updated: 2026-10-16
---

# UE3 editor co-op: opportunity analysis

**Status:** research only. No code, no packets, no lab run. Written for the owner's question: *can we use the co-opted UE3 editor source to improve Cimmeria tooling, and can we extend the editor so that we, other team members and AI agents can use it more easily?*

**Short answer:** yes, but treat the tree and the editor as an **external oracle**, never as a build dependency of Cimmeria. Cimmeria can start on two pieces of work with no licensing exposure and no dependencies: a format reconciliation that reads the tree as a spec, and a structural package differ in `crates/upk`. Both pay off even if nobody ever runs the editor again. Headless use through commandlets comes next, and gives agents a text surface (T3D, OBJ, logs) without touching Epic code. A live agent bridge into the editor is the most valuable long-term piece, but it depends on the collaborator owning a small additive module inside the tree and on several owner decisions (listed at the end).

## 1. What was inspected

| Item | Where | What it is |
|---|---|---|
| Engine tree | `..\SGW\Src` (sibling of this checkout on the owner's workstation), 4,229 files, 59.5 MB | The `Development\Src` of an **Infinity Blade UE3 tree (engine 7892)**, not SGW source. No `.git`. Original files carry 2011 dates and the read-only attribute. |
| SGW overlay | Same tree. 382 files have 2020+ dates (252 Engine, 51 SGWGame, 46 UnrealEd, 11 Core, 5 SGWEditor, the rest small). `@SGW BEGIN/END` markers appear in 36 files. | A collaborator's port that lets the IB editor load and save SGW packages (pkg 486, licensee 8, engine 3004, cooked 65). `README_SGW_PORT.md` (dated 10 Oct) still says "UNBUILT, UNTESTED" and lists 168 files, so the tree has moved on since that README was written. |
| Build evidence | `build_log.txt`, `BuildSGWEditor.bat` | A 15 Oct UBT run on the collaborator's machine (`E:\...\SGW UE3`) linked `Binaries\Win64\SGWGame.exe` (VS2008, DirectX SDK June 2010). |
| Compiled editor | Not on this workstation. A search of `..\SGW`, Downloads, Desktop, Documents and the projects folder found no `SGWGame.exe` | Only the collaborator has it. The owner and collaborator report that it opens every map, UPK, script and effect, and saves maps and UPKs. Not verified here. |
| Prior Cimmeria work | [Atrea editor archaeology](../../reverse-engineering/findings/atrea-editor.md), [editor bridge ADR](../../architecture/atrea-editor-bridge.md) (Proposed), [editor feasibility](../debug-area/custom-map-editor-feasibility.md), [custom-debug-map CM-00](../custom-debug-map/automation-probe.md), [lightmap tail](../custom-debug-map/lightmap-tail.md), [navmesh pipeline](../../engine/navmesh-build-pipeline.md) | Everything so far has targeted the **32-bit in-client Atrea editor** inside `SGW.exe`. Its automation (CM-00) never reached a usable scripted session. |

### Ground rules this analysis assumes

- **Licensing:** the tree is Epic/IB code marked confidential, and is very likely not redistributable. Nothing from it is copied into this repo: no code, no headers, no verbatim comments. Reading it as a format spec is the safe use. Findings are restated in our own words, with the class, field and layout facts only.
- **Evidence class:** the tree is evidence about **package format and editor behavior** only. It says nothing about SGW client runtime behavior or the wire protocol. The overlay is also a reconstruction that marks its own guesses `[U]`. Under the repo rule, a draft is a claim, not evidence: every format fact taken from it must be checked against client bytes or Ghidra before it changes a constant in `crates/upk` or `crates/upk-objects`.
- **Repo rules:** Cimmeria-side tools use PowerShell and the build lane. Prefer outputs that need no client patch. Map changes that do reach players ship as `cimmeria-patchset` bsdiff deltas, so no CME bytes are shipped ([crates/README.md](../../../crates/README.md)).

## 2. Improving existing tooling

### 2.1 The tree as a format oracle

These rows compare what the overlay serializes with what Cimmeria parses today. "Gives" means a hypothesis to test against client bytes, not a fact.

| Area | Tree source (read as spec) | Cimmeria counterpart | What it gives |
|---|---|---|---|
| Package summary and export table | `Core\Src\UnLinker.cpp`: SGW constants (486 / lic 8 / engine 3004 / cooked 65). At version 486 or lower, exports have no per-export `PackageFlags` field. | `crates/upk/src/header.rs`, `exports.rs`, `tools/upk_parser.py` | An independent check of header and export decoding across licensees 2/6/7/8. It also supports the overdue doc fix: some docs still say engine 3717 and pkg 530 (for example [source-reconstruction-feasibility.md](../../technical/source-reconstruction-feasibility.md); the 2026-10-12 64-bit client re-scope found 3004 / 486). |
| StaticMesh kDOP | `Engine\Src\UnStaticMesh.cpp`: a separate legacy kDOP tree type for **licensee < 8**, converted in PostLoad; saves require licensee 8 or higher. Also rebuilds smoothing groups for SGW meshes. | `crates/upk-objects/src/static_mesh/parse/mod.rs` (fixed 32-byte node / 8-byte triangle, no licensee branch found) | A concrete test target: decode one licensee-7 StaticMesh and confirm or deny the layout split. Navmesh collision depends on this. |
| BSP nodes | `Engine\Src\UnModel.cpp`: the field order of `FBspNode` at version 486 or lower (plane first, then component element index, ...) | `crates/upk-objects/src/model/` | A cross-check of the BSP decode used by navmesh and occluder extraction. |
| Lightmaps on components | `Engine\Src\UnLightMap.cpp/.h`: a legacy SGW lightmap holder, read and written back unchanged. The README says it parses to the exact byte on SGC_W1 for 5,387 StaticMeshComponents, 184 ModelComponents and 900 TerrainComponents. | [lightmap-tail.md](../custom-debug-map/lightmap-tail.md) (StaticMeshComponent, `FLightMap2D` only), `crates/upk/src/patcher/object_clone.rs` | Layouts of the **ModelComponent and TerrainComponent tails**, plus the "vertex" lightmap variant. `upk_patch` needs these before it can clone more than static-mesh actors. |
| Prefabs and archetypes | Overlay prefab serializer (listed in the README; `Engine\Src\SGWPrebuild.cpp`, `UnrealEd\Src\SGWBrowserTypes.cpp`) | `navmesh-extractor` coverage reasons `skip_archetype_stub_component`, `skip_archetype_unrooted`, `skip_archetype_no_mesh` and others | A spec for the prefab-archetype gap that `archetype_census` measures. |
| Classes Cimmeria does not decode | Overlay serializers for SkeletalMesh, animation encoding, Material / MaterialInstance (uniform expressions), decals | None | A starting spec if NPC bounds, animation timing or material swaps are ever needed. Low priority for a server emulator. |
| SGW script classes | `Engine\Classes\SGWRegionComponent.uc` (`Points[]` with Location / Rotation / PointId, `Height`, `Radius`, `bClosed`), `SGWSpecCoverNode.uc` / `SGWCoverNodeComponent.uc`, `SGWSkyDomeActor.uc`, `SGWAudioComponent.uc` | `cover_extract`; the `actor.rs` class list names `SGWSkyDomeActor`; **no** decoder for `SGWRegionComponent` | Property names and types for map-authored regions, which can seed server regions (ring or spawn areas) without hand-placing them. The client's own script source ships in the `.u` files, so this is a convenience copy, not a unique source. |
| BigWorld navgen export | `UnrealEd\Src\SGWBWExport.cpp` (a collaborator-authored new file): writes `.chunk` / `.cdata` per 100 m chunk, the way the original SGW editor did. Chunk id `%04x%04xo`; axis map BW = (UE.Y, UE.Z, UE.X) / 100; BinSection magic `0x42a14e65`; 103-vertex terrain grid. | `navmesh-extractor/src/chunk_id.rs` (**same axis map, verified by reading both**). Cimmeria builds NavBuilder input from extracted geometry, not from `.cdata`. | A second, independent source of navmesh input for differential checks. It also confirms the chunk-id convention Cimmeria arrived at on its own. |

### 2.2 Round-trip verification

Two directions, and both need a tool Cimmeria lacks: a **structural package differ**. `upk_info` dumps names, imports, exports, properties and tails (`--tail`); nothing compares two packages.

1. **The editor checks Cimmeria's patcher.** The collaborator opens an `upk_patch` output (for example the [Worldforge probe](../custom-debug-map/worldforge-shell-test.md)) in the editor, runs its map check, and resaves it. If the file loads without missing-resource errors, our append-only clone produced a well-formed package. This is a stronger check than a stock-client load, because the editor resolves every export.
2. **Cimmeria checks the editor's saves.** The overlay README admits that "many `Ar.Ver() >= X` checks will still write new-format fields when saving at 486". A structural diff of an editor-resaved stock tile against the original shows exactly where those leaks are, per export and per property. Today the collaborator can only find them by loading the result in the client. Cimmeria can give them a precise list (and a lab load, with the owner's OK).

The editor has its own `DiffPackagesCommandlet`, but it runs inside Epic code and needs the editor installed. A Cimmeria-owned differ is shareable and also works as a regression check for `upk_patch` with no editor at all.

### 2.3 Headless and commandlet use

The tree implements about 90 commandlets (verified by grepping `IMPLEMENT_CLASS`) and the standard exporters. The ones that matter to Cimmeria:

| Commandlet or exporter | Use for Cimmeria | Existing Cimmeria tool to check against |
|---|---|---|
| `BatchExport` + `SequenceExporterT3D` | Dump every Kismet sequence of a map as T3D text | `extract_kismet`, `tools/kismet_extractor.py` |
| `BatchExport` / `MAP EXPORT` + `LevelExporterT3D` | Dump every actor and its properties as text | `extract_actors`, `tools/extract_actors.py`, SceneEditor import |
| `LevelExporterOBJ`, `StaticMeshExporterOBJ`, `TerrainHeightMapExporter*` | Whole-level geometry as OBJ, including prefabs and BSP that the editor resolves | `extract_map` OBJ output and its `skip_*` coverage reasons. A differential check shows which geometry our extractor still drops. |
| `PkgInfo`, `ShowTaggedProps`, `ExamineOuters`, `ListPackagesReferencing` | A second opinion on package structure | `upk_info`, `query_index` |
| `AnalyzeKismet`, `FindDuplicateKismetObjects`, `PerformMapCheck` | Map lint before a patch ships | None |
| `ResavePackages`, `ReplaceActor`, `MergePackages`, `FixupRedirects` | Batch edits authored by the editor | `upk_patch`, `tools/map-lab/*` |
| `Make` | Compile UnrealScript packages, including our own script-only commandlets (see section 3) | None |

**Unverified:** whether commandlets take the overlay's SGW load path (the per-package `IsSGW` flag), and whether `ResavePackages` writes 486 or the tree's native 788 format. The SGW save path in `SavePackage.cpp` is triggered by the package's SGW flag and by the editor's "Save packages for SGW" toggle, so a commandlet might silently write 788. This has to be the first thing tested, on copies.

Spawn data note: server spawns come from `db/resources/` seeds, not from map files. Exports help only where the map carries markers (cover nodes, regions, Kismet-driven events), and their output becomes seed edits through the [seed-change](../../../.claude/skills/seed-change/SKILL.md) flow.

### 2.4 Editor-authored maps instead of byte patching

| | `upk_patch` / `map-lab` (today) | 64-bit IB editor (collaborator) | 32-bit Atrea editor (in `SGW.exe`) |
|---|---|---|---|
| Who can run it | Anyone with a client and the repo | Only holders of the compiled editor and IB content | Anyone with a client and Atrea's byte patches |
| Output fidelity | Exact for what it touches; append-only; deterministic | Native SGW format only as far as the downgrade audit reaches (README: partial); lighting written empty | Native, because it is the original editor |
| Scriptability | Full CLI, testable in `cargo test` | Commandlets plus an exec console; a bridge is possible because source exists | CM-00 failed to get a scripted session; the bridge ADR needs injection at fixed addresses |
| Best for | Small, reviewable, reproducible deltas; patchset recipes | New geometry, new levels, mass edits, inspection | The reference output for format questions |

Recommendation: **don't replace `upk_patch`.** Use the 64-bit editor for authoring new levels and geometry (custom-debug-map CM work), use `upk_patch` for scripted deltas, and use the structural differ to check both against stock files. Editor saves are probably not byte-deterministic (GUIDs, export order; assumed, not checked). So editor-authored changes should be reviewed through the differ's output, not through binary diffs.

## 3. Making the editor LLM-friendly

What agents need is a request/response surface that returns text, runs one session at a time and leaves deterministic logs, the same shape as the `cimmeria-lab` MCP (a lease, compact results, screenshots saved as file paths).

### 3.1 Out of process: no Epic code changes (preferred)

| # | Capability | How | Lives in |
|---|---|---|---|
| O1 | **Commandlet runner** | Runs `SGWGame.exe <commandlet> ... -unattended -nopause` against a disposable copy of the client tree. Captures the editor log, normalizes it into JSON (exit status, warnings, missing-resource lines) and holds a file lease so only one editor runs per workspace. | New Cimmeria tool (PowerShell or Rust bin); never ships the editor |
| O2 | **T3D ↔ JSON** | A Cimmeria parser turns T3D (actors, Kismet, materials) into stable, sorted JSON that agents can diff and edit. The reverse direction writes T3D snippets that the editor pastes in or imports (`MAP IMPORT`, `LevelFactory`). This is how a **text diff becomes an editor change**. | Cimmeria Rust crate; format knowledge only |
| O3 | **Startup command files** | UE3 normally accepts a command file at startup (`-EXEC=`) and an `EXEC` console command. Build an editor session as a script: load map, run commands, save under `SGWSAVEREDIRECT` (verified in the overlay: it redirects SGW saves to a side folder so originals are never overwritten), then quit. | Cimmeria scripts; the flag needs a test on this build |
| O4 | **Editor MCP server** | Wraps O1 to O3 (and later I1) as MCP tools: `editor_lease_*`, `editor_commandlet`, `editor_export_t3d`, `editor_import_t3d`, `editor_map_check`, `editor_screenshot`. Modelled on `lab-mcp`. | Cimmeria crate; useless without a local editor, but harmless in the repo |
| O5 | **Screenshots** | Window capture of the editor viewport, reusing the lab's capture approach. A proper in-editor shot command is I3. | Cimmeria |

A fallback that we do not recommend: DLL injection into the compiled editor, as the Atrea bridge ADR planned for `SGW.exe`. With the collaborator's PDB this is easier than with Ghidra addresses, but it breaks on every rebuild and duplicates what I1 does cleanly.

### 3.2 Additive changes inside the tree (owned by the collaborator)

UE3 of this era has no plugin DLLs: everything links into `SGWGame.exe`. A live bridge therefore means compiling code into the editor. The least invasive place is the collaborator's own `SGWEditor` module (5 new files today, loaded by `Launch`), with **new files only and no edits to Epic files**:

| # | Change | Why |
|---|---|---|
| I1 | **Loopback exec bridge**: a tickable object that polls a token-gated, loopback-only socket, runs each request through the editor's `Exec` on the main thread and returns the captured output. Uses the security model of the [bridge ADR](../../architecture/atrea-editor-bridge.md#security-model-loopback-socket). | Turns the editor console into an RPC that O4 can call: select, move, set properties, rebuild, save. |
| I2 | **Exec verbs for menu-only SGW features**: `SGWBigWorldExport` is reachable only from the File menu (verified; it has no console command). | Makes the BigWorld export and the "Save for SGW" toggle scriptable. |
| I3 | **Viewport capture verb** (a high-resolution shot to a given path) | Visual feedback for agents without screen-scraping. |
| I4 | **Structured log prefix** for SGW save, load and downgrade warnings | Deterministic logs that O1 can parse. |

A script-only option sits between the two tiers: UnrealScript commandlets (`class X extends Commandlet; event int Main(...)`) in a separate script package, compiled by `Make` and registered through ini `EditPackages`. This needs no C++ changes, but script commandlets cannot open sockets or write arbitrary files (assumed from UE3 norms). It suits batch exports that report through the log, not a bridge.

Changes to Epic files themselves are not needed for anything on the shortlist below.

## 4. Benefit to other team members

| Thing | Shareable? | How |
|---|---|---|
| Format findings written in our own words (field orders, layouts, class/property names) | Yes | `docs/` and `crates/upk*` tests, after checking against client bytes |
| Structural package differ, T3D↔JSON, commandlet runner, editor MCP | Yes | Cimmeria code. Useful to anyone; the editor-facing parts only matter to someone who has an editor. |
| The tree, the compiled editor, IB `Engine\Content` | **No** (owner legal call) | Collaborator-to-individual at most. Never in this repo, CI or the launcher. |
| Collaborator's in-tree bridge (I1 to I4) | Their decision | Lives with the tree. Cimmeria documents the socket protocol so the MCP side stays decoupled. |
| Editor-authored changes to stock maps | Yes, as deltas | `cimmeria-patchset` bsdiff recipes against the player's own files, the existing no-CME-bytes path |
| Wholly new editor-authored maps | Owner decision | They contain only SGW asset references and our geometry, but the publishing rule is unset |
| T3D/OBJ dumps of stock maps | Follow the existing rule for derived client data | Commit derived results (seeds, `.nav`); keep bulk dumps local (assumed policy, confirm) |

Who gains what: **the collaborator** gets a precise validation loop (structural diffs plus a lab load) in place of "send me the errors". **Map authors**, including whoever has the editor, get agents that can prepare T3D edits, run map checks and turn markers into seeds. **Contributors without the editor** still benefit through better `upk` / `upk-objects` coverage, a better navmesh and better seeds. **Agents** get one more MCP that behaves like the lab.

## 5. Risks

| Risk | Likelihood / impact | Mitigation |
|---|---|---|
| Licensing: Epic/IB confidential code and content | Certain / high if mishandled | Read as spec only. No code, headers or verbatim comments in the repo. Editor binaries never in CI or releases. Owner legal call before any wider sharing. |
| The overlay is a reconstruction (README "UNBUILT/UNTESTED" is stale, guesses marked `[U]`, incomplete downgrade audit) | High / medium | Treat every format fact as a claim and verify it against client bytes. Gate every editor-saved map on the differ plus a stock-client lab load. |
| No version control on the tree; the copy here differs from the README and from the collaborator's build machine | High / medium | Ask the collaborator to keep the tree in a private repo and quote a revision in every finding. Record the tree's file date when citing it. |
| Commandlets write 788-format packages or bypass the SGW save path | Unknown / high (corrupt files) | Work only on copies; use `SGWSAVEREDIRECT`; the differ checks the version field first. |
| Tool drift: Cimmeria tools assume an editor build that changes underneath them | Medium / medium | Keep the bridge protocol tiny and versioned; O1 records the editor exe hash in every run log. |
| Only one person can run the editor | Certain / medium | Do the editor-free work first (format note, differ). Treat editor runs as scheduled collaborator sessions, like lab time. |
| Agents editing the stock client tree | Medium / high | Enforced by the runner and lease: disposable copies only, same rule as CM-00. |

## 6. Candidate first packets (ranked, not sliced)

| Rank | Candidate | Effort | Value | Depends on |
|---|---|---|---|---|
| 1 | **UE-A Format reconciliation note.** Read the `@SGW` serializers against `crates/upk`, `upk-objects`, lightmap-tail and cover extraction. Write findings in our own words into [ue3-package-format.md](../../engine/ue3-package-format.md) or a sibling page. List test targets: kDOP on licensee 7, ModelComponent and TerrainComponent lightmap tails, `SGWRegionComponent` properties, the prefab archetype layout. Fold in the engine 3004/3717 doc fix. | S | High; no licensing exposure | Nothing (tree is local, read-only) |
| 2 | **UE-B `upk_diff` structural differ** in `crates/upk`: names, imports, exports, per-property and tail diff; GUIDs and offsets ignored. Useful straight away as an `upk_patch` regression check. | M (2-3 small packets) | High | Nothing |
| 3 | **UE-C Round-trip gate**: the collaborator opens one `upk_patch` output and resaves one stock tile; Cimmeria diffs both and lab-loads the resave in the stock client. | S (mostly coordination) | High; settles the editor-to-game boundary CM-00 never reached | UE-B; D-UE2; ask before lab use |
| 4 | **UE-D Commandlet runner and T3D→JSON** (O1, O2, O3): first targets are the Kismet T3D diff against `extract_kismet` and the level OBJ diff against `extract_map` coverage. | M | Medium-high | D-UE1 (an editor on a machine agents can use); UE-C confirms the commandlets' SGW load path |
| 5 | **UE-E Editor MCP and bridge** (O4 plus I1 to I4) | L | High long-term | UE-D; collaborator agreement and VCS (D-UE3, D-UE4) |

**Recommended order:** UE-A and UE-B in parallel now. Then UE-C at the collaborator's next session. Then UE-D. UE-E only after UE-D shows the editor surviving unattended runs. Cimmeria-side code for parsing `SGWRegionComponent` regions or new lightmap tails comes out of UE-A's test targets as ordinary `crates/upk*` packets.

## 7. Verified vs assumed

| Claim | Status |
|---|---|
| The tree is IB 7892 plus an SGW overlay; no `.git`; 382 files dated 2020+; `@SGW` markers in 36 files | Verified (read-only listing and grep) |
| SGW constants 486 / lic 8 / 3004 / 65; export `PackageFlags` absent at 486 or lower; licensee-split kDOP; `FBspNode` order; save redirect and compress console commands; IB-only components flagged transient on SGW save | Verified in source text. Their **correctness for SGW** is not verified. |
| `SGWBWExport.cpp` chunk id and axis map match `chunk_id.rs` | Verified by reading both |
| About 90 commandlets, plus T3D and OBJ exporters and the T3D level factory, exist in the tree | Verified (grep) |
| The BigWorld export is reachable only from a menu | Verified (only call site is `EditorFrame.cpp`) |
| A 15 Oct build linked `SGWGame.exe` on the collaborator's machine | Verified from `build_log.txt` only |
| The compiled editor opens all maps, UPKs, scripts and effects, and saves maps and UPKs that load in the stock client | **Claimed** by the owner and collaborator; not verified here; no binary on this workstation |
| Lightmap layout is byte-exact on SGC_W1 | **Claimed** in the overlay README (its verify scripts are not in the tree) |
| Commandlets load SGW packages through the SGW path; `-EXEC=` works; script commandlets cannot open sockets; editor saves are non-deterministic | **Assumed** from UE3 norms; each needs a test |
| `upk-objects` has no licensee branch for kDOP | Verified by reading the parser; whether licensee-7 meshes reach it is not checked |

## 8. Open owner decisions

| ID | Decision | Blocks |
|---|---|---|
| D-UE1 | May a copy of the compiled editor and the IB engine content live on the owner's workstation, where Cimmeria agents can run commandlets against disposable client copies? | UE-D, UE-E |
| D-UE2 | May Cimmeria agents ask the collaborator for editor sessions (resave or load specific files), and may their outputs be lab-loaded? | UE-C |
| D-UE3 | Do we ask the collaborator to put the tree under private version control, and to add the additive `SGWEditor` bridge (I1 to I4)? | UE-E |
| D-UE4 | Legal posture: confirm that "read as spec, restate in our words, never copy code" is acceptable, and that editor binaries and content are never shared through Cimmeria channels. | Everything past UE-A in practice; UE-A assumes yes |
| D-UE5 | Publishing rule for wholly new editor-authored maps (patchset delta, separate download, or not at all) | Shipping any CM map built with this editor |
| D-UE6 | Is the 64-bit editor now the primary authoring path, superseding the Atrea editor bridge ADR and CM-00's 32-bit route? | Whether the ADR is retired or kept as the fallback |
