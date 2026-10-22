# Launcher UAT fixes: resume card

Ledger: [README.md](README.md) · Packets: [work-packets.md](work-packets.md) (rules R1-R10, checks, C-LF1..3, waves 1-2), [work-packets-logs.md](work-packets-logs.md) (C-LF4..7, waves 3-5, close-out) · Source: [laptop UAT worknote](../launcher-telemetry/worknotes/laptop-uat-2026-10-22.md)

**Goal.** Unbrick light in-place adoption (F1), accept stock non-ASCII names (F2), one clear adoption UI (F3-F6), and make "Upload debug logs" ingest launcher and game log lines into SigNoz through a `logs`-purpose launcher session (G1-G5).

**State (2026-10-22, waves 2 and 3 at PR stage).** Worktrees live under `.claude/worktrees/`.

| Wave | Packets | PR | Status |
|---|---|---|---|
| 1 `lf-w1` | LF-01 43af4fafa, LF-04 c006bae75, LF-05 1c3b05ff0, LF-02 2acac60a4 merged (tip 9732aa635); LF-03 running (rust-gameserver-dev, STEP 0 ff to lf-w1 tip) | U1 | InProgress |
| 2 `lf-w2` | LF-06..09 shipped, wave review ship, tip c06433f85 | U2 #1984 | CI failure on `native-engine (macos-latest)`, log being read |
| 3 `lf-w3` | LF-10..12 + N4 fix, tip be931c9ca; ruling in worknotes/wave3.md (3,500 ms pacer for LF-14, halving test) | U3 #1985 | CI pending |
| 4 `lf-w4` | LF-13, LF-14 (security) → LF-15 → LF-16, LF-19 (security) → LF-17; stacked on lf-w3 | U4 | BlockedDependency |
| 5 | LF-18 after U4 merges | U5 | BlockedDependency |
| close | LF-20 | X | BlockedDependency |

**Next step.**
1. Resolve #1984 CI (real failure → fresh packet-coder in lf-w2 with the log, then re-review; infra → ops reruns failed jobs only).
2. LF-03 returns → packet review loop → merge into lf-w1 → wave 1 review → PR U1. U1 merges FIRST. Tell the owner D-LF1 replaced dry-run option (a) with a per-file check.
3. U2, U3 merge on green CI; retire worktrees on merge (`rm-worktree.ps1 --merged-into <PR>`).
4. After U3 merges: ASK the owner about the colo deploy before U4 merges. Ask before any launcher release, laptop or lab use.
5. Wave 4 from the lf-w3 tip (LF-14 follows the ruling).

**State update (2026-10-22, later).** U1 #1986, U2 #1984, U3 #1985 MERGED; wave 1-3 worktrees retired. Wave 4 in `lf-w4` (branch log-client-w4, tip cfb98490e = wave 3 + LF-13 + LF-14); LF-15 running in `lf-15`. D-LU5 (owner, 2026-10-22): merge ALL PRs first (no deploy-before-U4-merge gate), then cut both releases (colo server + launcher) together, server live before or with the launcher. Owner GO given 2026-10-22: do both releases (colo server + launcher) as soon as all code (U4, U5) is merged; no further ask. Check before cutting: the release path/skill (ops cannot release; may need `claude --agent hands-on`),. Owner confirmed 2026-10-22: nobody uses log upload today, so retiring the blob path (LF-18) breaks nothing in the field. Bring lf-w4 up to date with main before the U4 PR.

**Decisions:** D-LU1..4 decided (README). Wave-3 ruling check returned `fix` (spec-only halving test, applied by architect); shipped U3, disclosed to the owner.

**Owner risks relayed:** game logs unfiltered; 2,048-byte floor misses spaced names/secrets; install/address budgets are speed bumps; log rows forgeable by log-token holders; scrub gaps B2/B3/B4.

**Owner UAT owed (ask before any laptop use):** U1-U6 in the README.

**Gotchas.** `in_place.rs`, `inventory.rs`, `session.rs`, `log_upload/tests.rs`, `upload_gate.rs` near the cap: call lines only. Engine tests use `cargo test`. `Plan` is digested: never add a field. Never `git worktree prune`.
