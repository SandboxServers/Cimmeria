# CellBlock presentation UAT

> Type: how-to. Audience: packet workers and campaign testers.
> Updated: 2026-10-06. Companions: [packets](work-packets.md),
> [campaign decisions](README.md), [evidence ledger](evidence-ledger.md),
> [existing mission-route UAT](../castle-cellblock-rebuild/uat-guide.md).

These are acceptance scenarios, **not completed test results**. The existing mission
guide supplies route setup; this checklist adds asset, state and presentation checks.
Use the agreed QA client. Record the server commit, client/package hashes, overlay
version, mission phase, world/instance, scenario ID, expected and observed result,
and capture/log references. Keep credentials and personal character/account names out
of committed records. Use tester A/B labels in shared reports.

## Execution and recording

1. Confirm package baseline and patch manifests before starting. Keep original backup
   packages outside the client's package-scanned directories.
2. Use a clean tutorial character for route tests. A GM-triggered sequence is an
   isolated diagnostic and does not pass the mission-route case.
3. Run the normal interaction first; capture first-press response and the resulting
   mission/object/collision state. Inspect server state where a visual cannot prove it.
4. Repeat at the same mission boundary after relog and streaming reload. Run relevant
   observer cases with tester B, including a different mission phase.
5. Mark `PASS`, `FAIL`, `BLOCKED`, or `NOT RUN`. A missing capture, unavailable second
   client or unimplemented presentation remains unverified. Record the responsible
   packet and exact next action for every failure or blocker.

For each implementation packet, save a report under `worknotes/<packet-id>.md` with
scenario IDs and evidence. No build or replay pass substitutes for visual inspection.

## Scenarios

| ID | Setup and action | Required observation |
|---|---|---|
| CA-U01 | Start the tutorial and inspect the wake-up/corpse pickup area; take the existing first weapon. | Correct local props and streamed interactables appear once, are reachable and receive first-press feedback. Existing item grant behavior remains intact. |
| CA-U02 | Reach Prisoner 329's door-control step; fail/cancel then win its existing minigame. | Failure does not open the door. Success opens the intended cell door with correct sound/animation/collision and no unrelated door movement. |
| CA-U03 | Inspect Ambernol on the medical counter from pickup range and nearby viewing distances; take it. | Selected mesh/material/scale/pivot is believable and visible on the counter. Highlight and pickup refer to the same object. No clipping, duplicate actor or unreachable interaction. Confirm existing item semantics without altering cure design. |
| CA-U04 | Enter the intended desk cover during its active objective. | Cover marker aligns with the actual desk, appears in the intended phase and hides when satisfied. An unrelated cover location cannot pass the intended desk contract. |
| CA-U05 | Hack/use each existing internal ring control through its mission steps. | Correct rig/pair fires once; camera and movement recover. Existing minigame, mission and transporter ownership is preserved. |
| CA-U06 | Inspect the open Preparation locker and take its weapon. | Current SMG presentation occupies the locker correctly. No duplicate pickup or unintended P90 substitution. Class-start reward/ammunition expectations remain the sibling campaign's contract. |
| CA-U07 | Use Preparation_Terminal through failure and success. | Success visibly opens exactly the recovered pod set using the selected state mechanism. Stasis block double doors are checked separately from pod lids; mission progression and ring highlight still work. |
| CA-U08 | Approach the escape barrier before, during and after its defined mission phase. | Correct original/design-labelled visual and collision enable/disable together. No invisible wall or visible pass-through. Alternate route remains traversable. |
| CA-U09 | Inspect and use the Mess Hall long table in the existing tutorial. | Table geometry and relevant cover positions agree. Presentation changes do not change kill/flank gating or NPC AI. |
| CA-U10 | Trigger the existing Straegis scene through mission completion; repeat isolated routing diagnostics separately. | Source resolves, required tile is loaded, camera acquires the recovered Director track and restores control. Timeline and interruption/disconnect behavior match the packet contract; no duplicate sequence trigger. |
| CA-U11 | Observe the same Straegis scene from protagonist and approved observer positions. | Selected rift effect has correct position, scale, lifetime and timing relative to Marsh removal. Recovered and project-designed elements are identified distinctly in evidence. |
| CA-U12 | Inspect the aftermath after the scene, then leave/re-enter its streamed tile. | Blood is visibly distinct from water and has correct location/material/persistence. It does not replay the scene or accumulate repeated decals/effects. |
| CA-U13 | Open aftermath crate and return to it after loot collection/relog. | Correct crate mesh and reachable interaction; established loot/reward behavior is preserved. No additional grant from presentation restoration. |
| CA-U14 | Use Cellblock_TerminalX at mission 688 step 2356. | Exact Armory doors animate/open and permit passage. Step 80688 and ArmoryRingSwitch highlight remain intact; terminal interaction does not prematurely complete 688. |
| CA-U15 | Use the active Armory exit switch after the terminal. | Mission completion and World 12→8 transfer occur once, following the existing ring campaign contract. No downstream Castle content activates before transfer. |
| CA-U16 | Repeat interact/trigger requests in active and inactive phases; cancel/fail minigames where relevant. | Server rejects unauthorized phase/range/results, repeated inputs do not duplicate state/rewards/VFX, and valid first presses receive feedback. |
| CA-U17 | Disconnect/relog before and after pod, barrier, cinematic/aftermath and Armory transitions. | Mission and visual/collision state reconstruct consistently; one-shot cinematics do not replay unless the recorded contract requires it. |
| CA-U18 | Stream target tiles out/in; have an observer enter AoI after each completed transition. | Correct state is reconstructed for late viewers without duplicate actors, stale collision or replayed one-shot events. |
| CA-U19 | Tester A and B share the applicable instance with different mission phases. | Defined per-player/shared-state contract holds. One player's transition cannot bypass another's progression or leave contradictory visual/collision state. |
| CA-U20 | Run the complete tutorial through World 12→8 for every currently supported CellBlock branch selected in the coverage matrix. | All approved presentation changes work through real mission progression. Record each branch separately; forced skips do not pass a full route. |
| CA-U21 | Only if an editor/frontend feature changes: exercise affected logic in JS REPL and inspect it visually. | Create/update/delete, serialization, dirty/persistence and validation transitions affected by the change pass; report what the REPL did not cover. If no frontend changes, record `NOT APPLICABLE`. |

## Close-out gate

Each approved presentation row must map to a passing scenario and its packet's
automated regression evidence. Isolated checks, full-route coverage and two-client
coverage are reported separately. An accepted limitation needs its reason, impact and
next action; an unrun scenario cannot be reported as passed.

When restoration lands, mirror these scenario IDs into the matching section of
`docs/guides/unified-uat.md` in the same behavior-changing PR, per the doc-update map.
This planning checklist does not claim new behavior is available to testers today.
