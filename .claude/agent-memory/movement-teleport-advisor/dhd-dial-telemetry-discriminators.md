---
name: dhd-dial-telemetry-discriminators
description: How the stock DHD window really dials (first click eaten, a selection auto-dials), the telemetry that shows each step, and the 2026-10-05 Debug Area hub report that was not a server or grant bug
metadata:
  type: project
---

**How the stock DHD window dials (lab-verified 2026-10-05, colo, v2026-10-05.3):**

- The first left-click after the window opens is eaten (Flash focus). A second click on a row selects it.
- Selecting a row is the dial. The Flash `removeMovieClip` burst and `client.net.out onDialGate` (target `-1`, a reset) fire in the same millisecond. The glyphs fill and light (`activateGlyph`/`fadeOver` errors), and about 2 s later the client sends `onDialGate(target, source)` on its own. Pressing the orb (`ORB OVER`) only re-dials.
- The window reopens in its last state (the glyph ring, not the address book).
- The Reset button sends nothing.
- `Unsupported opcode 8F` shows up in every session: not diagnostic.
- Client rows' `timestamp` is upload time; use `attributes_number['ts_ms']`.

**The 2026-10-05 report (tester GM, Debug Area hub, 13 world-entry grants):** in one session (DHD at 20:54 UTC) the client logged a `removeMovieClip` burst but never emitted `onDialGate`, not even the `-1`. Two hypotheses were tested and **both are refuted**:

1. Addresses that arrive only through `updateStargateAddress` with an empty `setupStargateInfo` book are not dialable.
2. Gate 29 does not resolve from a cache resynced in an earlier session.

Evidence: the same tester relogged at 21:08 straight into DebugArea (`already_known=0`, all 13 again via `updateStargateAddress`) and dialled hub-granted Lucia (id 10, source 29) about 7 times. A lab GM on a persisted cache (no category-13 resync that session) dialled hub-granted Harset (3, source 29) after SGC_W1→DebugArea, after Castle_CellBlock→DebugArea with the Cellblock intro dialog, and in `/gmsetghost` mode. The one failed session stays unexplained. An untested candidate is the tester right-clicking the DHD again while the window was open, which reloaded the movie (`ActionScript Memory leaks in movie 'DHD.DHD'` 160 ms after the second `interact`).

**Why:** a DHD complaint can look like a server or grant bug when it is the window's input behaviour or a one-off client state.
**How to apply:** check `client.net.out onDialGate` first. No `-1` at all means the native emitter never ran. For a lab check, click a row twice and wait about 2 s; don't expect a single click or the orb to dial. Related: [[cross-world-teleport-arrival-path]], [[harset-travel-ground-truth]].
