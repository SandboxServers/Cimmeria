# harness-tuning: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-10-31
> **Companions:** [Ledger](README.md), [waves 1-2](work-packets.md), [waves 3-5](packets-wave3.md)

**State (2026-10-31):** designed, re-sliced for the owner rulings D-HT3 and D-HT10, nothing started. 23 packets in 5 waves. The ledger's four files were committed by the ledger PR (2026-10-31); later ledger edits are made in the main checkout and ride along with the next wave PR or HT-19.

**Goal:** a net-positive harness. Report-writer contract misses, coder stops, wrong-campaign resume cards, stale memory and owed guard proofs each get measured (wave 1), fixed (waves 2-4) and re-measured (HT-19) against README § Targets.

**Start here:**

1. Campaign-packet § 0: check that no peer runs this campaign. Have `ops` create the board subcategory.
2. Wave 1 (`ht-w1`, one PR): HT-01, then HT-02/03/04 at once from HT-01's reviewed head, then HT-05 from `ht-w1`. Merge order 01, 02, 03, 04, 05.
3. Wave 2 (`ht-w2`, one PR) **alongside wave 1**: HT-06, 07, 08, 10 at once from `main`; then HT-09 and HT-11a at once from `ht-w2`; then HT-11b. Merge order 10, 08, 06, 07, 11a, 09, 11b. HT-06/07/08 also get a testing-validation-engineer pass.
4. Once wave 1 merges: HT-16 (architect ruling) fills the HT-17 section.
5. Once wave 2 merges: wave 3 (`ht-w3`): HT-13 and HT-12a, then 12b, then 15a/15b/15c (Sonnet, multi-session, from worknotes).
6. Once wave 3 merges: wave 4 (`ht-w4`): HT-14, HT-17, HT-18 (HT-18 at HT-14's head; D-HT10 decided).
7. Campaign review once the re-measure window is full (at least 100 report-writer contract dispatches, 20 coder stops and 15 cards), then HT-19.

**Hazards:**

- **Don't write `.claude/state/active-campaign` until HT-08 merges.** Another campaign's card reads it. After a compaction, read this file by hand.
- **Haiku coders write no report and no worknote (owner, D-HT3).** Coder briefs name no report path. Until HT-07 merges, add to every coder brief: "No report file and no worknote. At `Context cap reached`, call no tool and return the one line with `reason=context`." A coder return with no usable line counts as a context stop: continue from the branch with `base=<start SHA>`.
- The user-level Haiku cap stays as it is; never work around it.
- The main checkout holds uncommitted files that aren't ours (`frontend/ui/index.html`, other ledgers). Never commit from the root.
- `.github/workflows/agent-skills.yml` is LF; HT-10 and HT-13 both edit it, in different waves.
- `test_hooks.py`: HT-06 (`WiringTest`) and HT-08 (`ResumeCardTest`). `orchestrator.md`: HT-07 and HT-08 (different paragraphs), then HT-09. Check both pairs with `git merge-tree`.
- No cargo anywhere in this campaign; every check is a foreground `python` command.
- HT-18 runs live Haiku sessions (at most 6): only from a worktree.
- **lab-autopilot (#2048) runs in parallel.** HT-15c keeps `game-archaeology-specialist/ue3-reflection-trace-anchors.md` (AX-64 may update it) and must not touch `lab-driver/` while AX-61 is open. HT-11a/11b/12a: `git merge-tree` `development-workflow.md` against AX wave 6 (frame-describer roster row). README § Overlap.

**Open decisions:** none. D-HT3 and D-HT10 were decided by the owner on 2026-10-31.

**Next ledger update:** when the wave 1 or wave 2 PR opens.
