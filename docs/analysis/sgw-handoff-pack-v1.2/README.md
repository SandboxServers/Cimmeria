# SGW Claude Server Handoff Pack v1.2

An implementation handoff for the class, skill-tree, combat, and world-content layer of the Stargate Worlds reconstruction, produced by an external chat-based research effort and delivered on 2026-09-18. It was written to be given to Claude working on this server, and its first task is a compatibility audit, not code.

## Contents of this folder

| Path | What it is |
|---|---|
| [phase0-gap-report.md](phase0-gap-report.md) | **Start here.** The Phase 0 compatibility and gap report the pack asks for, plus the proposed Phase 1 patch plan and the decisions the owner must make. |
| [audits/](audits/) | The five unabridged domain audits (schema, trainer runtime, combat, items, worlds) the report is synthesised from. Evidence for every claim. |
| [pack/docs/](pack/docs/) | The pack's own documents: start prompt, source policy, known unknowns, implementation checklist, combat spec, QA tests, schema proposal, world index. |
| [pack/data/](pack/data/) | The pack's machine-readable data: trainer export, class and skill trees, recovered ability and effect master, weapon variants, combat config, starter loadouts, world routing. |
| [pack/references/class_and_combat/](pack/references/class_and_combat/) | Nine reference workbooks rendered to markdown (one file per workbook, one section per sheet), plus the missing-formulas note. |
| [pack/references/world_content/](pack/references/world_content/) | Seventeen per-world dev-master workbooks rendered to markdown. |
| [pack/references/source_extracts/](pack/references/source_extracts/) | The pack's "legacy SQL" and text extracts. These are extracts of this repo's own seed; see the report's provenance section. |
| [pack/manifest_v1.2.json](pack/manifest_v1.2.json) | SHA-256 manifest of the original delivery. |

## What was not committed

The original `.xlsx` workbooks (27 files, about 3.5 MB of binary). Every sheet is preserved verbatim in the markdown renderings, cell values only, no formulas or formatting. The two stale manifests (`manifest.json`, `manifest_v1.1.json`) were dropped.

## Authority

Per the pack's `SOURCE_POLICY.md`, every unlock level, branch-point gate, skill-point cost, prerequisite edge, and combat constant in the pack is **RECONSTRUCTION / INFERENCE** unless a row says otherwise. Branch membership and the recovered ability, effect, item, world, and stargate records are source-backed. Do not relabel a reconstruction as original because it appears in a "Final v1" file.

The report finds that the pack's data files were derived from this repository's own `db/resources/` seed. A match between the two confirms provenance, not retail correctness.
