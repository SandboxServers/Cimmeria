# Union Beta E1 continuation: resume card

> **Last updated:** 2026-10-18 (UB-00c)
> **Ledger:** [README.md](README.md) · **Contract:** [receiver-contract.md](receiver-contract.md) · **Amendment:** [id-amendment-request.md](id-amendment-request.md) · **Index:** [beta-work-packets.md](../union-zone-planning/beta-work-packets.md)

## Current wave

**B0** (seals and pilot). UB-00c is committed on `uzp-ub00c` and not merged. The U-D03 / FZ-99 start gate is waived by the user for Beta (W-UB1). The Cellblock, Castle, Harset and Tollana ledgers were not independently verified.

## In flight

- UB-00c: this commit (docs only; no seed, code or `id-allocation.md` change).
- UB-00a1, UB-00a2 and UB-00b are Ready under the waiver. This ledger does not know whether they are running.

## Next action

1. Coordinator merges the UB-00c commit and the D-UB9 rows from `id-amendment-request.md` into `docs/analysis/four-zone-planning/id-allocation.md`, re-running the ID check that day.
2. Dispatch UB-01a once that row is merged (the ID re-check found no conflict).
3. Orchestrator asks the owner D-UB4, D-UB5, D-UB7, D-UB8 and H-LBL before the dependent waves.
4. Rewrite this card at every wave boundary.

## Open decisions

| Decision | Blocks | Default in the packet |
|---|---|---|
| D-UB4 west-valley route | UB-02d, UB-21, UB-22, UB-40 camp rows | none (UB-00b may find a walk route) |
| D-UB5 offworld tails | UB-22, UB-50, UB-51 | A: stop at the last Beta-local step |
| D-UB7 FV twins | UB-52 | leave out |
| D-UB8 rewards, H-LBL | UB-50 | xp 0 / naq 0 |
| HELD-B16 Asgard producer (UO-16, U-D08/U-D11) | UB-30 reachability | replay-test with archetype 5 |

## Findings to carry

- ID re-check: every range `clear` on `origin/main` @ `102d15444`, all 1047 remote refs, 185 local trees and 188 worktrees.
- The planning index defines M1-M9 only partly: M3 and M5 have no text (receiver-contract.md). UB-45b is missing from the index packet table.
- Receiver fixtures: UO-13 owns the departure side; UB-10, UB-20 and UB-30 ship `beta_e1::m1423`, `m1500` and `m481` `receiver_*` and never import Omega's `handoff`.
