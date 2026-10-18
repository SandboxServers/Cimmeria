# Castle continuation resume card

**Castle is a second concurrent campaign.** Cellblock owns `.claude/state/active-campaign`; Castle never writes it, and reads this file itself after compaction (D-CA3). This overlay is never committed to `main` (D-CA2). The canonical ledger is `content/castle-campaign` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle`. Read `README.md`, then `r2-packets.md`, `r3-packets.md` and `r3-packets-2.md`, then the branch ledger.

## State, 2026-10-17 (R2a integrated; R2b starting)

- **`FEATURE_SHA=e78a44272` (CS-INT2a)**: R2a seals (CS-00R2, CS-00Be, CS-02S, CS-03S, CS-02L) reviewed (2 rounds) and cherry-picked with CS-01b. D-CA-SYM = Option B (no implant). The Castle server is stopped. R2b: CS-MGF1 and CS-02c1 first, CS-03c1 after CS-02c1.
- Previous: `FEATURE_SHA=da2b82c70` (CS-INT1). R1a is integrated: chains Ra 1371-1373, Rb 1374-1377. None of 1367-1377 is seeded yet.
- **No lab until the end** (D-CA11). The user runs `uat/end-uat.md` in CS-07c. The CS-01c rows are UATPending.
- **The Castle server is still running**: PID 36540 (`castle-cs-01b`), DB `sgw_castle_cs_01b` (it holds Cellblock's `Cellutb`). Stop it through CS-01d (D-CA13). Never reload or drop that DB.
- CS-01b `009ab0536` is waiting for CS-INT2a. The R1a worktrees await the owner's retire (D-CA14).
- R3 is fully pre-written: R3a seals, R3b/R3c builds, the R3d template, and the final phase. Every packet's gate is checked at dispatch.

## Next actions

1. R2, per `r2-packets.md` § Dispatch briefs: CS-01d, then R2a (five seals), CS-INT2a, R2b, R2c, CS-INT2. Record each new `FEATURE_SHA` here.
2. After CS-INT2a: dispatch R3a (`r3-packets-2.md` § Dispatch briefs): CS-06S, CS-05S, CS-04aD, CS-04bD and CS-04cS now; CS-08bM only if CS-00R2 is `SEALED`; CS-03dS only if D-CA-SYM = A. These can run while R2b/R2c build.
3. R3a review, then CS-INT3a (needs CS-INT2 before R3b starts). Then R3b, R3c and CS-INT3, dispatching only packets whose gate is met.
4. Ask ops at each dispatch for `git log content/harset-campaign`: HA-01b gates CS-06a (D-CA18); HA-1360a3c/b gate CS-08a2S.
5. Relay D-CA15 to the Harset coordinator: HA-1360b should target `crates/services/src/frosts_letter_round_trip_tests.rs` (created by CS-08a2M).
6. R3d after D-CA16; then the final phase: CS-07R, CS-08a2 (if Castle merges second), CS-07a, CS-07b, CS-07c (user), then the PR.

## Open decisions (owner)

- **D-CA-SYM:** the 1521 implant, Option A or B. It gates CS-03dS and CS-03d only.
- **D-CA16:** build the authored quests after `medical_q1` before a live catalog proof? Recommended: no (hold), and accept Cellblock's CB-N05 proof as valid for Castle.
- D-CA12 is sealed by CS-02L, not the owner. D-CA9 is moot until the end UAT.
- **D-CA12 reading (note, awaiting the owner's confirmation):** b2-lite is "player-bound and server-counted, not skill-proof" (residual R-LW1: a scripted client can win Livewire instantly). Both CS-02L verdicts are conditional on it; a "skill-proof" reading makes CS-MGF1/2 BlockedDecision.

## End-of-campaign cleanup

- For CS-07c, restart the Castle server with the CS-01b recipe; `Stop-Process` it afterwards.
- The owner retires every integrated `castle-*` worktree with `rm-worktree.ps1 <name> --force`; `castle-cs-01b` goes last. Drop `sgw_castle_*` only after Cellblock no longer needs `Cellutb`.
- Copy worthwhile reports into the branch `worknotes/` before any retire. Never run `git worktree prune`. Never touch Cellblock's worktrees, DBs, server or overlay.

## Blockers

G-567 (CS-08bM/08b, waiting on CS-00R2); HA-01b (CS-06a, D-CA18); HA-1360a/b (CS-08a2S/08a2); D-CA-SYM; D-CA16 and the catalog proof (R3d); the full G-REVIEW (CS-07b).
