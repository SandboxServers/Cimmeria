---
name: project-invisible-cellblock-guard
description: "Open bug: the Cellblock NID guard (template 15, spawn 20) is often never rendered on a fresh character although the client creates it; what SigNoz showed, what is ruled out, and the dropped requestEntityUpdate (#838)"
metadata:
  type: project
---

**Symptom (colo, 2026-09-26/27, build 83154f8b).** On a fresh-character Cellblock run, the guard `ArmYourself_NIDGuard` (template 15, spawn 20) aggroes through chain 1008 on Region8 entry and shoots the player, but the client never renders him. This happened in 4 of 5 fresh runs. Relogging heals it.

**Evidence from SigNoz:**

- The guard enters AoI during the first-login cinematic hold (`docs/architecture/first-login-cinematic-aoi-hold.md`). He is flushed in a three-NPC bundle with the two stasis-room corpses when `cancelMovie` arrives (`aoi.cinematic_hold` reason=`cancel_movie`). The server path is identical in the one visible control run, and `held_ms` does not separate visible from invisible runs.
- **The client does create him.** About 375 ms later it sends `requestEntityUpdate` (`0x07`) with his bare entity id, as it does for every newly created NPC. The base drops every such request: see #838, which also shows the message is the BigWorld cache-stamp handshake rather than a recovery request.
- `WitnessEntityMethod` is held only during the cinematic hold, not in the pre-`onClientReady` window. So `onAggressionOverrideUpdate` (mob method 27) can reach the client before the create. The visible control run got the same early method, so this is not the discriminator.

**Ruled out earlier** (June and 2026-09-19, for the related corpse that vanished until relog): a failed spawn, the class gate in AoI create, the witness-address gate, a missing mesh, and Mercury loss (the create and its cascade were ACKed on the first try).

**2026-09-29 update (server-side fix prepared, not merged).** Live lab (build b819aae4) showed the world-entry phase-2 cascade for 24 NPCs going out as ONE 18,367-byte bundle in 15 `FLAG_FRAGMENTED` packets; the client ACKed all 15 but processed only a prefix, then dropped the rest (~1 login in 4, random NPC subset). Ghidra (`Bundle::iterator::unpack` @ `0x01579830`, `Nub::processOrderedPacket` @ `0x0157c820`) shows the client demands each message HEADER (id + u16 length = 3 bytes) inside one packet and abandons the whole rest of the bundle when it is not. Our fragmenter cut the byte stream raw every 1300 bytes, so about one cut in ten landed in a header. Fix: `crate::packet::plan_fragments` moves such a cut back to the message start; guards are in `crates/mercury/src/packet/fragmenting_wire_tests.rs`, the loopback `fragment_header_guard.rs` and the 24-NPC flush test. This is a strong candidate for the "guard never rendered" symptom below (the guard is in the same flush), but it is NOT confirmed until the client telemetry (`client.mercury.bundle`) shows the abort live; the earlier hypotheses in this note about a create landing mid cinematic-exit predate it. The original observation "prefix of 12 NPCs, a skipped chunk, one stray message" is explained at the entity layer (messages for a not-yet-created entity are queued), not by a Mercury-level skip.

**Open (before the 2026-09-29 update).** Why the client leaves this one actor unrendered is client-side and unproven. The leading guess is a create landing mid cinematic-exit. There is no GM command to re-introduce an entity. Next steps: once #838 lands, retest; otherwise read the client entity list through the live research lab (`docs/guides/live-research-lab.md`).
