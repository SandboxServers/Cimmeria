---
name: kismet-designer-events-and-door-state
description: sequences.event_id 6000+N is the map's SeqEvent_Designer N; a door's start pose and what a play_sequence does to it can be read from the cooked map's Matinee keys; the SGC_W1 legacy script closes Carter's lab doors and never opens them
metadata:
  type: project
---

Found reading the cooked SGC_W1 map for Class Start v6 CS-05 (2026-10-05).

**`resources.sequences.event_id` 6000+N fires the Kismet `SeqEvent_Designer`
with index N** in the named sequence. The node's `ObjComment` says what it
does ("Designer 0: Close Doors", "Designer 1: Open Doors"), and
`extract_kismet --graph` prints it. Cross-checked on four SGC_W1 sequences
(Hammond's office door, Code 9 start and stop, the north bulkhead doors):
each comment is consistent with where the legacy script plays the sequence.

**A ported legacy `play_sequence` on a door is not always an open.** SGC_W1
chain 3028 plays 10009, which is `CartersLabDoors` Designer 0 = close. The
legacy script never played 10010 (open), so the lab was sealed for anyone
sent there. Before building content behind a door, read the comment.

**Start pose and travel come from the Matinee keys.** Each `SeqAct_Interp`
has an `InterpData` whose `InterpTrackMove.PosTrack` holds the keys as
tagged properties (`InVal` float, `OutVal` vector, UE units, relative to the
initial pose). Two keys `(0,0,0) -> (0,0,-400)` over 2.0 s is a door dropping
4 m; the actor's placed `Location` is the pose before any clip plays.
`SeqVar_Object.ObjValue` names the actors a clip moves. The repo's Python
reader (`tools/upk_parser.py`) needs `lzallright`; a venv outside the repo
works, and tagged properties start at byte 32 for actors, 8 for components,
and vary for Kismet nodes (try offsets 0..40 in steps of 4).

**Why:** a mission route was built to a room the script itself had locked.
**How to apply:** for any chain that sends a player through an `InterpActor`
door, list the map's `SeqEvent_Designer` nodes for that door, check which
events the seeded chains play, and check `nav_inspect` separately: the
navmesh does not model doors.

Related: [[map-data-placement-toolkit]], [[ue3-and-navmesh-index]].
