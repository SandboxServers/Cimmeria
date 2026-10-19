# Union Beta E1 continuation: resume card

> **Last updated:** 2026-10-18 (ARCH-B1) · **Ledger:** [README.md](README.md) · **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md) · **Packets:** [beta-packets/](../union-zone-planning/beta-packets/)

## Current wave: B1a (Ready)

B0 is integrated on `uzp-e1` except UB-00a1, which is in review pass 2 (finding C6), owned by the concurrent editor of `mission-seal.md`; B1a does not wait on it. ARCH-B1 ruled D-UB4 A (ring), D-UB5 A, D-UB6 confirmed, D-UB7 out, D-UB8 xp/naq 0, and D-UB9 (approved by the coordinator; screens re-carved to 64124-64193).

## B1a dispatch list (all in parallel, each off `uzp-e1`)

| Packet | Worktree = branch | Packet file | Role |
|---|---|---|---|
| UB-01a | `uzp-ub01a` | `beta-packets/UB-01a.md` | packet-coder |
| UB-02a | `uzp-ub02a` | `beta-packets/UB-02a.md` | packet-coder |
| UB-DP | `uzp-ubdp` | `beta-packets/UB-DP.md` | packet-coder |

Report path: `<worktree>\.claude\reports\union-beta-e1\<packet>-coder.md`. One packet-review-fixer for the wave, then merge each into `uzp-e1`.

## Wave order (sequential; lanes inside a wave run in parallel)

- **B1b**: UB-01b (`uzp-ub01b`) and UB-02b (`uzp-ub02b`) in parallel; then UB-02c (`uzp-ub02c`) and UB-02d (`uzp-ub02d`), both stacked on `uzp-ub02b`. Needs UB-01a and UB-02a merged. UB-DP may merge late, but before B3.
- **B2**: the UB-10 to UB-13 stack, UB-20, UB-30 (three lanes).
- **B3**: the UB-14 to UB-16 stack, UB-21, UB-31, UB-40 (four lanes). Needs UB-DP merged.
- **B4**: the UB-32 to UB-36 stack.
- **B5**: UB-45a, then UB-45b stacked.
- **B6**: UB-90 (lab: ask the user first), then UB-99 close-out.

## Open owner decisions (BlockedDecision; the orchestrator asks)

| ID | Question | Holds |
|---|---|---|
| H-LBL | Which label does player-facing CIMMERIA_AUTHORING copy carry on Beta? | UB-50 |
| U-D07 | What xp and naq should each of the 17 Beta missions award? | UB-50 |
| HELD-B16 | Which Omega packet delivers an Asgard player at 481 S1760 with item 3399 (UO-16 held)? | Asgard end-to-end reachability only; UB-30 to UB-36 dispatch (D-UB13) |

## Carry

- Hold/closed: UB-22 and UB-51 held (D-UB5 A), UB-52 closed (D-UB7). Stock screens sit in the Dakara and Omega authored screen ranges (README § Screens re-carve); flag it to those campaigns.
