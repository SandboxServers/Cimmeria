# lab-autopilot: resume card

> **Type:** Reference · **Audience:** The orchestrator, or the owner, picking up the leftovers · **Updated:** 2026-11-01
> **Companions:** [Ledger and close-out](README.md#close-out), [AX-64 UAT ledger](uat-ax64.md), [work packets](work-packets.md), [contract](contract.md)

**State (2026-11-01): closed.** Waves 0 to 6 shipped and the campaign review passed at pass 3. AX-01, AX-03 and AX-04 are on `main` (#2052, #2050, #2051); everything else ships as one stacked PR from `lab-autopilot/wave6-int` (PR to be opened; link added by the orchestrator). Nothing is left to dispatch. AX-02 (live spike) never ran; AX-64 ran once, thinly (1 PASS, 2 FAIL, 10 NOT RUN, no lies found).

**What is owed (all need the owner's OK, D-AX4):**

1. O1: `tools/lab/install.ps1`, then `tools/lab/daemon.ps1 restart`, from the merged `main`.
2. AX-02, then an AX-64 re-run: cases and setup in [unified-uat.md](../../guides/unified-uat.md#lab-autopilot-ax-64-cases-still-owed). Fix first: the lab account file's character name and the `SGWEditor.ini` DST modal. U7 and U11 need a coordinator session with `client_input_key`; U9 needs three back-to-back identical failing calls.
3. O2: drop the orphan `sgw_*` databases after showing the dry-run list.

**Open decisions:** D-AX2, D-AX3, D-AX10, D-AX4 (per run); D-AX6 is provisional (z -100000).

**Known debts** (full list: [README § Close-out](README.md#close-out)): AX-24 queued `watch_start` after a daemon disconnect; AX-27 takeover/new-acquire race; AX-43 `client_cursor_move` cannot show `denied`; AX-46 `contains` check; AX-56 `movement.rs` size; the F1 and F3 fixes are unverified live.

**Hazards:** only trace flags `0x2086` and `0x20bf` are proven (`0x3fff` AVs; the trace latches off after a fault). Key fail-safe gaps remain for the fallback input mode and posted mouse buttons on a daemon crash. Tests never reach the lab (R7). Workers: `Set-Location <wt>;` on every call, lane in the foreground, never `--target-dir`, never `git worktree prune`.
