# harness-tuning: resume card

> **Type:** Reference · **Audience:** The orchestrator resuming this campaign · **Updated:** 2026-11-01
> **Companions:** [Ledger](README.md), [wave 1](work-packets.md), [wave 2](packets-wave2.md), [waves 3-5](packets-wave3.md)

**State (2026-11-01):** waves 1 and 2 DONE locally; HT-16 DONE; waves 1-4 DONE in `ht-w1`..`ht-w4`; campaign review, then HT-19 next. 24 packets in 5 waves (HT-16 split HT-17 into HT-17a and HT-17b). Ledger edits since the ledger PR live in the main checkout and ride along with HT-19.

**Goal:** a net-positive harness. Report-writer contract misses, coder stops, wrong-campaign resume cards, stale memory and owed guard proofs each get measured (wave 1), fixed (waves 2-4) and re-measured (HT-19) against README § Targets.

**Owner directive (2026-11-01): no PRs or merges to main until the campaign ends.** `ops` merges reviewed packet heads into `ht-w1`..`ht-w4` with `merge-noff.ps1`, and each later wave starts from the previous integration branch. "Merged" here means merged into the integration branch. All PRs open at close-out. HT-18 needs the user's OK for the lab; lab-autopilot has lab priority.

**Done so far:**

- Wave 1: `ht-w1` at f6b346758 (wave review pass 2 `ship`: HT-01, 02, 03, 04, 02a, 05). Delete the leftover branch `ht/HT-05-baseline` at close-out.
- Wave 2: `ht-w2` at b7641d961 (wave review pass 2 `ship`, 7 packets; HT-06 ruling in worknotes/HT-06.md). Re-run the skipped metrics-vs-hook LINE-equality test in `ht-w3` (the wave 1+2 join).
- Wave 3 DONE (2026-11-01): `ht-w3` at 04265381e (wave review pass 2 `ship`: HT-12a, 12b, 13, 15a, 15b, 15c). HT-15b and HT-15c needed a pass-3 architect ruling each (worknotes in their worktrees); HT-15c ships with about 50 memory files not traced to code (accepted by ruling). Next: wave 4 (`ht-w4` from the `ht/w3` head): HT-14, HT-17a, HT-17b at once, then HT-18 (live; ask the user first).
- Wave 4 DONE (2026-11-01): `ht-w4` at 3e4b3a350 (wave review pass 2 `ship` for HT-14, 17a, 17b; HT-18 `ship` after a ruling; HT-14b and HT-18b `ship` at packet level, user OK for both live runs). Live guard proofs: subagent-stop and hands-on-cargo proven; Codex hooks do not fire (C0 reproduced); orchestrator-edit never attempted by the model, so it stays owed (stop rule: no HT-14c). Next: campaign review once the re-measure window is full (at least 100 report-writer contract dispatches, 20 coder stops, 15 cards), then HT-19 close-out (PRs, ledger commit, board handoff, retire worktrees, delete `ht/HT-05-baseline`). Check the C9 prompts in packets-wave3.md lines 103-104 match the amendment before committing the ledger.
- HT-16: ruling in worknotes/HT-16.md. HT-17a is a 75k `coder_context_note:` hook in packet-coder's frontmatter plus Grep/re-read habits; HT-17b adds the commit and habit fields to `metrics coder-context`. The new Targets rows are in README. No owner call.

**Start here:**

1. Campaign-packet § 0: check that no peer runs this campaign. Then the campaign review once the re-measure window is full (at least 100 report-writer contract dispatches, 20 coder stops and 15 cards), then HT-19.

**Hazards:**

- **Haiku coders write no report and no worknote (owner, D-HT3).** Coder briefs name no report path. A coder return with no usable line counts as a context stop: continue from the branch with `base=<start SHA>`.
- The user-level Haiku cap stays as it is; never work around it. HT-17a only reads its value.
- The main checkout holds uncommitted files that aren't ours (`frontend/ui/index.html`, other ledgers). Never commit from the root.
- `work-packets.md` (424 lines) holds the dispatch rules, C1-C2 and wave 1. `packets-wave2.md` (394 lines) holds C3-C7 and wave 2. New packet text goes in `packets-wave3.md`.
- `.github/workflows/agent-skills.yml` is LF; HT-10 and HT-13 both edit it.
- `orchestrator.md`: HT-07, HT-08 and HT-09 edit it; check it with `git merge-tree`.
- No cargo anywhere in this campaign; every check is a foreground `python` command.
- HT-18 runs live Haiku sessions (at most 6), only from a worktree.
- **lab-autopilot runs in parallel.** HT-15c keeps `game-archaeology-specialist/ue3-reflection-trace-anchors.md` and leaves `lab-driver/` alone while AX-61 is open. HT-11a/11b/12a: run `git merge-tree` on `development-workflow.md` against AX wave 6. README § Overlap.

**Measure:** Guard proofs 2026-11-01: 3/4 (subagent-stop and hands-on-cargo proven; Codex hooks do not fire); orchestrator-edit is owed (no HT-14c).

**Open decisions:** none. D-HT3 and D-HT10 were decided by the owner on 2026-10-31.

**Next ledger update:** at the campaign review (HT-19).
