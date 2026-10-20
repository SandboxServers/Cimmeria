---
name: greedy-chain-local-minimum
description: Block on sight: "restart find_path from the last Partial waypoint" is greedy and stalls at a Euclidean local minimum; a stall is not proof of an island. Use authored goal-directed hops that each return Ok.
metadata:
  type: project
---

UB-00b-3 (2026-10-19): Beta RuinNote (122.2, 27.17, 356.8) looked unreachable because the greedy chain stalled at (137.61, 7.77, 370.28), 19 m below. The route exists via hops (394.3,-84.63,388.7) -> (169.6,-0.571,324.4) -> (131.2,27.57,320.3) -> pin, each Ok.

**Why:** `find_path` Partial ends at the visited poly nearest the goal in straight-line distance; the true route detours away first. Also the 256-poly cap.

**How to apply:** to prove reachability, chain toward intermediate targets and assert each leg Ok within 2.0 m; to prove an island, use a negative control that really is one (nav 890 Science interior behind the 0.90 m gap fails honestly). Reverse legs are not symmetric. Probe by throwaway example in `crates/entity/examples`, deleted after. Detail: placement-seal.md Addendum UB-00b-3 in the uzp-e1 worktree. See [[tiled-navmesh-seams]].
