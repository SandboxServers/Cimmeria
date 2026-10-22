# Launcher telemetry: resume card

Ledger: [README.md](README.md) · Packets: [work-packets.md](work-packets.md) (rules, contract, waves 0-2), [work-packets-coverage.md](work-packets-coverage.md) (waves 3, 7), [work-packets-install.md](work-packets-install.md), [work-packets-repair-launch.md](work-packets-repair-launch.md), [work-packets-adoption.md](work-packets-adoption.md) (waves 5, 6), [work-packets-wave4.md](work-packets-wave4.md), [work-packets-light-adoption.md](work-packets-light-adoption.md) (wave 8) · Design for wave 8: [light-adoption-proposal.md](light-adoption-proposal.md) (v2) · Older ledger text: [worknotes/resume-ledger-2026-10-21.md](worknotes/resume-ledger-2026-10-21.md)

**Goal.** Any launcher problem is debuggable from SigNoz alone. Wave 8 adds three things:
- Light adoption: in place, tolerant per patch op, and it never blocks.
- Incremental update without the 4.1 GB seed, for light-adopted installs only (D-LA14).
- A non-blocking comparison against the published file index, which LT-37 publishes.

**Follow-up campaign (2026-10-22).** The laptop UAT findings F1-F6 and G1-G4 moved to [launcher-uat-fixes](../launcher-uat-fixes/README.md) (closed; PRs #1984-#1988 plus its close-out). Light adoption's final check is now a per-file diff, Abandon works after `Promoted`, and "Upload debug logs" ingests into SigNoz. Owner UAT rows U1-U6 are in [unified-uat.md](../../guides/unified-uat.md#launcher-uat-fixes); the D2 wording question is still open.

**Wave 8 close (2026-10-22 late).** PR L #1981 and PR I2 #1982 merged. LA-01..LA-19 are done and `light-adoption` (tip `ebf8a46f3`, 47 commits over main, restacked) is open as **PR LA #1983**; the user queues the merge. LA-20 done: the signed manifest now carries `file_index` -> release `content-file-index-20261022` (worknotes/LA-20-publish.md, rollback recipe in it). Final cross-packet reviews A and B are running (worknotes/wave8e-final-review-A.md, -B.md); apply their fix notes on the branch, then push. Open: D2 (`inPlaceMaintenance.abandon` wording after a light abandon), D-LA12, Windows ARM64 scope, owner UAT (laptop light adoption, seedless Update, comparison panel, LT-27 U1-U9; ask first). Left from wave 7: LT-24 (PR F), LT-41 (PR H), LT-27 (PR X). Stale AU worktrees still to retire. Older notes below.

**Current state (2026-10-21 late).**
- PRs S #1976, I #1977 and A #1978 are merged; **PR L #1981 is open** (`launcher-telemetry/launcher`, worktree `lt-app-integration`, tip `e6f16395a`). The user queues its merge. Left from waves 0-7: LT-24 (PR F), LT-41 (PR H) and LT-27 (PR X, the owner's laptop UAT; ask first).
- LT-37 is superseded by LA-05 + LA-20 (worknotes/LT-37.md). The per-file policy idea is superseded by D-LA9 and D-LA11.

| Wave | Packets | Ships as | Status |
|---|---|---|---|
| 0-6 | LT-00..LT-36 | PRs S, I, A (merged), L (open) | see README |
| 7 | LT-24 (F), LT-41 (H), LT-27 (X) | own PRs | BlockedDependency |
| 8a | LA-01, LA-02, LA-06, LA-07, LA-08 (+ LA-05 for PR I2) | PR LA (`lt-light-integration`); LA-05 is PR I2 from `origin/main` | Ready |
| 8b | LA-03 → LA-04; LA-09 → LA-10 (Sonnet); LA-12; LA-11 (Sonnet) after LA-10 + LA-12 | PR LA | BlockedDependency |
| 8c | LA-13 (Sonnet); LA-14 → LA-15 → LA-16 → LA-18; LA-17 | PR LA | BlockedDependency |
| 8 close | LA-19 (documentation-writer), LA-20 (hands-on, after PR I2) | PR LA / release ops | BlockedDependency |

**Next step.**
1. `ops` follows work-packets-light-adoption § "Starting wave 8a while PR L #1981 is open": fetch, record `L_TIP` and `MAIN_TIP`, create `lt-light-integration` at `L_TIP`, create `la-01`, `la-02`, `la-06`, `la-07` and `la-08` at `L_TIP`, and `la-05` at `MAIN_TIP`. If PR L moves, merge its new tip into the integration branch; once it merges, rebase onto `origin/main` from the last `L_TIP`.
2. Dispatch wave 8a in parallel (six packet-coders, in the background; all six lanes are independent). Run each packet's review loop when it returns, then the wave review (five packets). Merge in this order: LA-07, LA-02, LA-01, LA-06, LA-08.
3. Ship PR I2 (LA-05) on its own, with green CI. Then dispatch LA-20 to hands-on. It is authorized: it signs, creates `content-file-index-<yyyymmdd>`, swaps, and has its rollback ready.
4. Waves 8b and 8c follow the merge orders in work-packets-light-adoption § Branches. PR LA restacks onto `main` after PR L merges.
5. Standing: retire each packet worktree, its `B:\targets\` folder and its test DB the day it merges. Never `git worktree prune`. No `--target-dir target/desktop`.

**Open owner decisions:** D-LT2 consent (default A applied); D-LT20 trust past loopback (only if OPS-LT2 says no); D-LT21 stream match by forwarded address (default no); D-LA12 delete the verified in-place path after the UAT (asked at close-out).

**Owner UAT owed (ask before any lab or laptop use):** light adoption of the laptop install; the first Update with no seed download; the comparison panel against the published index; LT-27 U1-U9.

**D-LA14 (owner, 2026-10-21).** The seedless Update is for light-adopted installs only (the adoption provenance has `check = light`). Downloaded installs update exactly as today (rule R-LA 10). Every-install is the later option D-LA14b.

**Gotchas.** `publication.rs` and `in_place.rs` are at the 700-line cap, so new logic goes in new files. R5: index samples carry only `Changed`/`Missing` paths from the index, never `Extra`. Docs are CRLF. No agent reads a key except LA-20 under the owner's authorization.
