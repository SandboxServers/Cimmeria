# lab-autopilot: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-31
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [contract](contract.md)

**State (2026-10-31):** design revised against the live pass (L1-L8) and static pass 2; nothing dispatched. Base `main` `2019922fe`. 7 waves, 46 packets. Campaign docs are uncommitted in the main checkout; commit them in the first docs PR through `ops`. No worktrees, no board subcategory yet.

**Goal:** lab tools that cannot lie (evidence verdicts, gate, post-reads, server corroboration) plus puppet-grade control (holds, watch, batch, trace, coast-aware walker, detectors), back-ported from ramenvr/autopilot concepts to the UE3 client. Lab-only client code, never in the manifest or the launcher (C13).

**Start here:**

1. Campaign-packet § 0: check no peer runs it. Have `ops` create the board subcategory.
2. Ops chores ([work-packets § Ops chores](work-packets.md)): O1 (rebuild and re-verify the lab binaries) before any live run or wave 2 DLL test on the lab; O2 (`sgw_*` orphan-DB cleanup). Both need the user's OK; ask, never let an agent ask.
3. Wave 0 (one PR per packet): AX-03, AX-04 (packet-coder) and AX-01 (game-archaeology-specialist, static only) at once, from main. Merge order AX-04, AX-03, AX-01, then AX-02 last. AX-02 is live: ask the user first (D-AX4).
4. Wave 1 (one PR from `lax-w1`): AX-10 and AX-15 at once, then AX-11..AX-14, then AX-16. Merge order 10, 15, 11, 12, 13, 14, 16.
5. Wave 2 (one PR from `lax-w2`): the DLL packets AX-20 (then AX-21, AX-24, AX-25 at its head) may start from main beside wave 1; then the chain AX-22, AX-23, AX-26, each at the `lax-w2` head after the previous merged in. Merge order 20, 21, 24, 25, 22, 23, 26.
6. Wave 3 (one PR from `lax-w3`): AX-30 first, then AX-27, then AX-31..AX-36. Waves 3, 4, 5 each open when the previous PR merged; dispatch order is at the top of each `packets-waveN.md`.
7. AX-55 is Ready once the wave 2 PR merged (no AX-01 wait, no wave5b); AX-56 after AX-54 and AX-55.
8. Wave 6: split ship per `packets-wave6.md`. Ask the owner D-AX3 and D-AX10 before AX-61 step 2. AX-64 is live: O1 first, then the user's OK. Then the campaign review loop, then AX-65.

**Open decisions:** D-AX2 (capture stays PrintWindow; default holds), D-AX3 (describer model; default haiku), D-AX10 (lab-driver tool list; AX-61 step 2 only), D-AX4 (user OK per live run: AX-02, AX-64). O1 and O2 need the user's OK.

**Hazards:**

- The main checkout has unrelated uncommitted edits (`frontend/ui/index.html` and others); never commit from the root.
- O1 restarts the lab daemon, which drops any held lease; run it when no lab session is live.
- `server/mod.rs` (732) and `world/click.rs` (733) are over the hard cap until AX-15 and AX-30 split them; R4 lists every near-cap file.
- Trace flags: only `0x2086` and `0x20bf` are proven (L2); `0x3fff` AVs. AX-55 refuses everything else (C14) and latches on an AV.
- Key fail-safe gaps left after AX-26/AX-27: fallback input mode, and posted mouse buttons on a daemon crash (README § Safety).
- Workers: `Set-Location <wt>;` on every call, lane in the foreground, never `--target-dir`, never `git worktree prune`.
- Tests never reach the lab (R7). The only live packets are AX-02 and AX-64.
- Every new routed tool goes in one `lease/policy.rs` list and in `JUDGED` or `NO_CLAIM` (R9); OPEN tools never in `NO_CLAIM`.

**Next ledger update:** when wave 0 returns or wave 1's PR opens.
