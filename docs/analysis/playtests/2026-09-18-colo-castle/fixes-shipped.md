# Colo playtest 2026-09-18 — fixes shipped

Part of the [2026-09-18 colo playtest report](README.md). Section numbers continue from that document so existing references (code comments, PR descriptions) still resolve.

## 10. Fixes shipped from this playtest

First behaviour-fix pass (PR `fix/playtest-npc-facing-follow`). Everything here is covered by a regression guard;
none of it has been seen in a client yet, so each row names what to look for in UAT.

| Finding | Fix | UAT check |
|---|---|---|
| H1 backwards facing | `pack_angle` wraps into `[0, TAU)` and rounds to the nearest step instead of saturating negative yaws to 0 | Aggro a guard from its -X side: it should run at you face-first. `.bug` it: `wire_facing_vs_caller_deg` near 0, `yaw_byte` not 0 |
| P49 player facing (same wire field) | Client facing bytes are stored as **radians**, in `[pitch, yaw, roll]` slots from wire order `(yaw, pitch, roll)`; they round-trip to the identical byte. Done in the same change because the `pack_angle` fix alone would have turned the old saturated player yaw into noise | Needs two clients: the other player should face the way they are actually facing. **Least certain row** — the wire order comes from RE of the client packer (`0x00de1720`), not from a capture |
| H4b attackers cannot turn | The stop-and-attack branch now writes yaw toward the target every AI tick (before the ability check, so a mob on cooldown still tracks). No new wire traffic | Strafe around a guard that is shooting you: it should keep turning to face you (2 s AI tick, so in steps) |
| Zero-length hop snapped NPC to north | Coincident waypoints keep the current yaw | — |
| Follower levitates (section 8.2) | The unrouted follow fallback keeps the follower's **own** Y instead of lerping toward the leader's | Jump repeatedly while Zuritska follows in Castle: he should stay on his floor. He will still walk through walls until Castle has a navmesh (H3) |
| Romney unreachable (section 8.5) | Spawn 240 moved from the sealed mirror wing to the dead end of the accessible cell corridor, `(244.0, 66.79, 1036.0)`, a line the playtest telemetry proved walkable; heading +PI/2 faces the approaching player | Walk the Interrogation Block without `.gotoxyz` / `ghost`: Romney is at the far end of the corridor past Zuritska's cell |
| Cell NPC faces the back wall (section 8.5) | `Castle_Zuritska_Cell` heading 0 -> PI | Zuritska faces his doorway |

Still open, in suggested order: take-cover edge race (H9); respawn drops region hints (H8 — reproduce first); leash
predicate + snap (H6 — fix both together or the snap desync becomes visible); escort robustness and Coppleman's
missing chain (H7); `castle.nav` (H3); OnGround `0x18` experiment (H2); `aggression` seed + assist aggro (H5); DHD
`INT_DHD` arm (H10); duplicate Zuritska + marker; objective 4647; dialog 5859 timing; item 2135 icon/description.
