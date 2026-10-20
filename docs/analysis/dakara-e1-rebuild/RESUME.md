# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-19 (documentation-writer, DK-99e). Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no agent lab use until the campaign ends; the owner runs every UAT. One read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder`. Do not touch `.claude/state/active-campaign`.
- **State:** everything buildable before the owner's UAT is merged locally on `dakara/dk-final` (worktree `dk-final`): waves 1 to 6d, including DK-14c, DK-15b, DK-30b-r, DK-33-r, DK-32c and DK-SGC-01/02, final-reviewed. It awaits the single end-of-zone PR; nothing is pushed. No step has been run in a client.
- **Dormant by design (D-DK23):** 1648 to 1654 stay unreachable in play until 1647 (wave 7) ships. Until then the owner sets 1647 up with the SQL in the [UAT guide](../../guides/unified-uat.md#dakara_e1) (no GM command completes it).

## Next action, in order

1. Open the end-of-zone PR from `dakara/dk-final` (`ship pr -C <wt>`), rebased onto `main`; CI green, then merge via the queue. Retire `dk-final` and the old `dk-*` worktrees the day it merges.
2. The owner runs the UAT from [the unified guide](../../guides/unified-uat.md#dakara_e1): test order 15a (M1-M2, DK-U1 to DK-U17, DK-P1 to DK-P9) then 15b (M3-M5, DK-U18 to DK-U37).
3. Wave 7 (GDK1, DK-20, DK-21, DK-32b) waits on the M2 playtest (D-DK5 A, answered). Then DK-99b to DK-99d (D-DK38), post-M2.
4. Through `ops`, post the [§ Notice](sgc-return-plan.md#notice-to-the-union-sgc-coordinator) if it has not gone out; it revises the SGC campaign's D-US9.

## Owner checklist

- [x] D-DK40, D-DK44, D-DK2 reading: option C (owner, 2026-10-19), built by DK-SGC-01/02.
- [x] D-DK21 to D-DK23, D-DK32 to D-DK39, D-DK41, D-DK42, D-DK45 accepted (owner, 2026-10-19).
- [ ] **D-DK48** (non-blocking): should the SGC story departures become hand dials? Recommended no (keep Harriman).
- [ ] **D-DK5 follow-through:** answered A (wave 7 after the M2 playtest). Confirm when the M2 playtest has run so wave 7 and DK-99b to DK-99d can start.

## UAT owed (owner)

- M1 DK-U1 to DK-U10 and M2 DK-U11 to DK-U17: runnable on deploy. M3 DK-U18 to DK-U25, DK-U32 and M4/M5 DK-U26 to DK-U31, DK-U33 to DK-U37: need the 1647 setup. DK-U34 checks the Vocuum pad (template 457, PL-DK-A-14); DK-U35 to DK-U37 check the SGC end. Placement DK-P1 to DK-P9.
