---
name: stargate-dial-travel-timing-na35
description: NA35 (2026-09-25) — deprecated Python is not gate-travel evidence; DHD dial is single-shot server-invisible; onStargatePassage unsent
metadata:
  type: project
---

**Deprecated Python is not a behavioural reference for gate travel.** The owner confirmed 2026-09-25 that the legacy server never had working gate travel end to end, so "the 2009 server emitted/never emitted event X" (D-CA10's framing) is not evidence about the 2009 *client's* expectations — only about one abandoned server. Ground truth for Stargate/DHD timing is `SGW.exe` alone. D-CA20 in `docs/analysis/castle-rebuild/README.md` records the correction; full evidence in `docs/reverse-engineering/findings/stargate-dial-and-travel-sequences.md`.

**Why:** general pattern risk — other systems ported from `deprecated/python/` may carry the same false authority if they trace back to gate travel or anything else the legacy server never finished end to end. Worth checking before citing `deprecated/python/cell/*.py` as a timing/behavioural source anywhere, not just gate travel.

**Key binary facts (all HIGH confidence, `SGW.exe` Ghidra):**
- The DHD dial UI (`FUN_005682d0` @ `0x005682d0`, case `'d'` "dialStargateAddress") collects all 7 glyphs client-side and reports the finished address to the server exactly once — there is no wire-level signal for in-progress chevron selection. This makes server-driven chevron broadcast (6106-6112) **structurally impossible without a client patch**, not merely unattested.
- `runStargateEvent` (same function, case `'r'`) computes `eventId = atoi(param) + 6100`, confirming the `ESequenceEventType` Stargate base and that the DHD's own Flash UI can trigger any of the 14 Stargate sequences locally.
- `Stargate_CrossGate` (6113) is matched per-gate-instance via `SourceAddressId`/`TargetAddressId` against a `USeqEvent_Stargate` Kismet node (`FUN_00e2c810`/`FUN_00d2de90`) — a real world-space trigger, not UI chrome.
- `onStargatePassage` (client method 68, `ON_STARGATE_PASSAGE` constant in `crates/services/src/cell/client_methods/gate_travel.rs`) has a confirmed real `VGateTravel` subscriber (`0x00e30010`, corroborating `stargate-dhd-state-machine.md`) — had zero production send call sites before this session; now sent.
- `GATE_DIAL_DURATION` (was 4s, `crates/services/src/cell/space_manager/gate_dial_state.rs`) had zero client-binary support; its only source was the disavowed Python.

**Implemented (2026-09-25, once the owner lifted the session's usage restriction):** `GATE_DIAL_DURATION` retimed to 100ms (tick-drain minimum, no confirmed replacement duration); `onStargatePassage` now sent to the crossing player right after `Stargate_CrossGate`; a `PendingCrossing` hold (`crossing_hold_state.rs`, a sibling split from `gate_dial_state.rs` to respect the file-size soft cap) defers `RESET_ENTITIES` behind a movement-locked, timeout-bounded, disconnect-safe wait (`CROSSING_CINEMATIC_HOLD = 1.5s`, still provisional — no Matinee/Kismet duration extracted). Commit `907c187a` on `npcai/na35-gate-travel-cinematic`. `crates/upk-objects` still has no Matinee/`SeqAct_Interp` reader — extracting a real cinematic duration remains a follow-up for whoever picks up the open questions in the finding doc.
