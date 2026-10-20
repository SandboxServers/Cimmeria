# Union SGC continuation: resume card

> **Last updated:** 2026-10-19 (US-99)
> **Ledger:** [README.md](README.md) (release readiness) · **UAT:** [U1](uat-U1.md), [U2](uat-U2.md), [U3](uat-U3.md) · **Index:** [sgc-work-packets.md](../union-zone-planning/sgc-work-packets.md)

## State

The campaign is closed on the code side. Packets through US-32 are on `main`. US-33, US-34, the S4 fix pass and this close-out ship in the single Union SGC PR. Nothing has been played in a client.

## Next action

1. Owner lifts the no-PR directive; ops opens one PR for `union-sgc/sgc-final`. Before it merges, decide whether `dakara/dk-sgc` (gate 27 pin) lands first.
2. Owner runs the three UAT files and records PASS, FAIL or NOT RUN in each; the escort walk and the placement lab checks are in U1.
3. A FAIL becomes a fix packet; do not fix inside the UAT session.

## Owner still owes

| Item | Note |
|---|---|
| D-US2, D-US3 label policy | US-40 stays HELD until it is set |
| D-US4 to D-US8, D-US12 | Shipped on their defaults |
| UAT U1, U2, U3 | Not run |
| Heading on crossings | `Action::CrossWorldTeleport` has no heading field; decide whether to add one after U3 |

## Carry

- Engine: an all-optional step completes the mission on the first completion; chains of equal priority run in chain-id order (all SGC chains keep priority 0).
- Retire the `sgc-final` worktree and its test database the day its PR merges.