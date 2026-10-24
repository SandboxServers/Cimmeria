# Ledger: launcher preflight and actionable failure reporting

Live packet state. [RESUME.md](RESUME.md) is the short card; this file holds the detail.

## Execution mode (2026-10-24)

- The owner ordered fully autonomous execution: use best guesses and don't wait for the owner.
- packet-coder, packet-reviewer, ops and architect refuse darwin. The orchestrator dispatches `general-purpose` stand-ins briefed with each role's `.claude/agents/<role>.md`, plus the bash lane (`tools/build-lane/lane.sh`) and the git/ship twins (`tools/build-lane/*.sh`). It never dispatches `hands-on` for packet work.

- Owner order (2026-10-24): every campaign PR requests GitHub reviewer `Cadacious` once it's ready for review. Every ship brief must include `gh pr edit <n> --add-reviewer Cadacious`.

## Which host runs what (D-PD19)

- Everything builds on this Mac. These need a CI job before `done`, because the Mac can't compile their Windows code:
  - WG (`native-engine (windows-latest)`): PD-07, PD-10, PD-20, PD-24, PD-29, PD-31, PD-36, PD-41.
  - RG (`runtime-probe`) and WR (release dry run, Windows): PD-25, PD-33.
- MG (the `native-engine (macos-latest)` CI job plus local `lane.sh`): every packet whose README Verify column names it.
- These need a live Mac check: PD-30, plus UAT-PD1-4, UAT-PD7, UAT-PD8 (PD-27), UAT-PD10 (PD-40), UAT-PD11 and UAT-PD12 (PD-38/39).
- These need a Windows PC: UAT-PD5, UAT-PD9, UAT-PD11 (Windows half), UAT-PD13, and the PD-41 install check. UAT-PD6 runs on both.

## Owner questions: all resolved (D5 in OWNER-DECISIONS.md)

- D-PD5 `os_error` in local events: yes. D-PD4: map onto the existing 18 summary values. D-PD6: deferred.
- D-PD22: PD-38 returned `none`, so the PD-39 client-patch hook is approved. D-PD23: RetinaMode off by default, opt-in after UAT-PD10.
- These stand: Windows also defaults to windowed 1024x768 (D-PD14), `resolution` is left unwritten (D-PD21), and the frame cap is Mac only (D-PD20).

## Packets

| Packet | Worktree | State |
|---|---|---|
| PD-22 | .claude/worktrees/pd-22 | review-1 ship (5cf2ba66e); PR #2004; merge gated on UAT-PD7 (live Mac); Cadacious requested |
| PD-01 | .claude/worktrees/pd-01 | MERGED (PR #2006); worktree being retired |
| PD-02 | .claude/worktrees/pd-02 | review-3 ship (a4d0a0cf5); PR #2008, auto-merge armed (MG only); Cadacious requested. Spec follow-ups (PD-02 Tests bullet; PD-05 must handle the no-launch-result-record failure path) are being written by an architect stand-in; PD-05 dispatch waits on them. |
| PD-03 | .claude/worktrees/pd-03 | coder running (branched from origin/main cf27c056c, which contains PD-01) |
| PD-04 | .claude/worktrees/pd-04 | coder running (same base). PD-05 starts from the PD-04 tip once PD-04 has `ship` and #2008 (PD-02) has merged; rebase onto origin/main first. Spec follow-up done (spec-followup-pd02-pd05.md). |
| PD-23 | .claude/worktrees/pd-23 | coder blocked (`display` in Preferences breaks the golden plan bytes, R10); ruling-check ship (ecc58065d); PR being opened (auto-merge, Cadacious) |
| PD-24 | .claude/worktrees/pd-24 | coder running (stacked on PD-23 ecc58065d; WG gate) |
| PD-26 | .claude/worktrees/pd-26 | coder running (stacked on PD-23 ecc58065d) |
| PD-38 | (none, RE) | review-2 ship; verdict cursor=none; outputs shipped in the campaign-docs PR; PD-39 re-specced |
| campaign-docs | .claude/worktrees/pd-docs, pd-docs-fix | #2005 merged (43ad7f1ef) with a 53-line RESUME. Fix PR #2007 MERGED. Both worktrees are being retired. Next: once spec-followup returns, ship the main checkout's ledger and work-packets edits in one docs PR, then sync the main checkout. |

## Parked side-quests (not this campaign)

- `agent-skills / skills mirror + hook tests` isn't a required check and doesn't run on `merge_group`, which is how #2005 merged while red. Candidate fix: add it to `ci gate` or to the ruleset.
- On darwin, `mk-worktree.sh` creates `external` as a symlink, which the `external/` ignore pattern misses, so every worktree retire needs `--force`. Candidate fix: also ignore `/external` (no trailing slash).
- The stale local branch `launcher-preflight-diagnostics/campaign-docs` (80831a7c7) has content superseded by #2007. It needs `git branch -D`, which only the owner or hands-on can run.
- `markdownlint-cli2 --fix <file>` without `--no-globs` rewrites the whole repo, because of the config's `globs`.
