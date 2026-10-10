# Knowledge gaps and confidence

> Updated: 2026-10-07. Static findings do not imply a live UAT pass.
> Companions: [evidence ledger](evidence-ledger.md), [packets](work-packets.md),
> [binary findings](ghidra-findings.md), [UAT](uat-guide.md).

## High-confidence follow-up findings

The QA `Castle_CellBlock-fffffffe.umap` contains a Director cut at time 0 with
transition time 0 targeting group `Camera`. Interp export892 links its Data to
281 and Camera to SeqVar_Object988, whose ObjValue is CameraActor89. InterpData281
lists camera group286 and Director group287. This is a proved static camera
binding, not successful runtime playback. See [structured extraction](straegis-binding-followup.json)
and [read-only reproducer](research_straegis.py). The nested CutTrack decoder
reports 92 of 100 bytes consumed; its known fields support the binding, but the
remaining bytes must not be silently treated as fully decoded native data.

Ghidra confirms the Director's virtual +0x3a4 call is SetViewTarget: the named
PlayerController script wrapper calls the same slot. Its controller cast checks
PlayerController ancestry. The event path also has parent/level, instigator,
count and retrigger gates. Addresses and prototype limitations are recorded in
the binary findings.

## Answers not yet supported at high confidence

The [per-export coverage diagnosis](parser-coverage-followup.json), reproduced
with [the existing readers](research_coverage.py), recovers 609 successful actor
property records from the four tiles previously discarded by whole-tile actor
extraction. Five individual exports still fail: Terrain29 in 00000000, Brush6
in fffdfffe, Brush6/12 in fffffffd and Brush8 in ffffffff. These are parsing
failures, not evidence that the map data is corrupt. The remaining persistent
Kismet failure is `CharacterRimLighting`, one-based export136; the original
extractor's error log reports zero-based index135. Successful property reads
remain partial evidence because opaque native data is not fully decoded.

| Question | Current limit | Next action / packet |
|---|---|---|
| Why does the Straegis camera fail on the current route? | Static binding exists; live event, controller, source and loading state have not been captured. No proof ViewType3 fixes it. | CA-04A: observe activation and group/controller resolution, then isolated and real-route CA-U10. |
| Which original rift assets bind to this scene, and when? | GroundVoid/Hold are package candidates; a scene attachment/timeline has not been proved. | CA-04B: follow dependencies and binary callbacks, compare with Marsh removal and runtime playback. |
| Is there an original blood aftermath? | Name-negative search is limited to the inspected QA corpus; puddles/materials need dependency and rendered inspection. | CA-04C: exhaust decal/material/particle references, then use authorized project design if unresolved. |
| What does Ambernol actually look like on the medical counter? | Syringe and ShelfBox leads are proved exports; icon identity does not establish rendered mesh or transform. | CA-02A: inspect mesh/material dependencies and QA rendering. |
| Do the stasis pods open, or only the block doors? | The recovered block-door graph does not establish pod behavior. | CA-03B: separate actor/animation targets and verify each. |
| What exact barrier and Armory-door state should be restored? | Current chain actions omit the suspected visual transitions; exact event targets, collision and phase ownership remain to be established. | CA-03C/D: trace actor/event dependencies before implementation. |
| Is actor/Kismet coverage exhaustive? | Four actor tiles and persistent Kismet export135 have reported parse failures. | CA-00B: reproduce and cross-check with Rust extraction; do not infer absence in affected coverage. |
| Will restored presentation recover after relog, streaming and differing player phases? | No restoration implementation or recovery UAT has run. | CA-05A state contracts, CA-06D regression guards, CA-U17..19 live observations. |

These are open investigative gates, not findings that originals are unavailable.
Only a bounded, documented search can justify a project-designed replacement.
The campaign remains open through restoration and full-route UAT.
