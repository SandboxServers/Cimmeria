---
name: CellBlock asset audit baseline
description: Source and package findings and the restoration campaign packet/UAT pointers.
type: reference
---

2026-10-07: Campaign now has 23 child execution contracts and 21 planned UAT
scenarios. `knowledge-gaps.md` records confidence limits. Targeted QA map decode
proves Director cut time0/transition0 targets Camera, Interp892 -> SeqVar988 ->
CameraActor89. Ghidra PlayerController execSetViewTarget0x0052ee10 confirms virtual
+0x3a4 used by Director; controller cast0x0055fcc0 checks PlayerController class
constructed0x0077a870. Runtime cause remains unresolved; no blind view0->3 fix.

# CellBlock asset audit static baseline — 2026-10-06

Source: `docs/analysis/cellblock-asset-audit/evidence-ledger.md`, baseline
`3bccbd9789480c56873754a4c00dfebf3c60a3b2`; generated package/export evidence beside it.

- Supplied map archive contains 65 UMAPs **including persistent**, plus MapData:
  the handoff's 65 streamed + persistent count is incorrect.
- Existing Cimmeria source already re-tags spawn 79 as ArmoryRingSwitch, gates
  its region on mission 688 and invokes Straegis sequence 1751 from chain 1161.
  Do not implement these again from a stale handoff. Terminal chain 1107 still
  has no door action; Preparation Livewire chain 1061 has no pod action.
- QA package exports prove ShelfBox13 and Syringe00 meshes and GroundVoid/Hold
  ParticleSystems exist. They do not establish intended mission bindings.
- StraegisAttack direct export read gives 10.0095968 s Camera/Director Matinee;
  SeqVar 988 points to camera 89. Its package hash matches the supplied developer
  handoff row. No rift/blood track was established by this investigation.
- Python actor extraction fails on four tiles; Kismet extraction has one persistent
  map export failure. Name census succeeds, but that does not repair actor coverage.
- Owner selects full restoration through in-game UAT, QA baseline, original recovery
  followed by explicitly labelled project design for unresolved presentation.
  Implementation and UAT are not performed by the planning investigation.

The campaign's `work-packets.md` contains the dispatch contracts; `uat-guide.md`
defines CA-U01–CA-U21 presentation, recovery and two-client acceptance scenarios.
These are planned checks, not results. Keep current source verification distinct
from original-asset evidence and in-client playback.
