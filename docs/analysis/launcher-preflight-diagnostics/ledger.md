# Ledger: launcher preflight and actionable failure reporting

Live packet state. [RESUME.md](RESUME.md) is the short card; this file holds the detail.

## Execution mode (2026-10-24)

- The owner ordered fully autonomous execution: use best guesses and don't wait for the owner.
- packet-coder, packet-reviewer, ops and architect refuse darwin. The orchestrator dispatches `general-purpose` stand-ins briefed with each role's `.claude/agents/<role>.md`, plus the bash lane (`tools/build-lane/lane.sh`) and the git/ship twins (`tools/build-lane/*.sh`). It never dispatches `hands-on` for packet work.

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
| PD-22 | .claude/worktrees/pd-22 | review-1 ship (5cf2ba66e); PR #2004; merge gated on UAT-PD7 (live Mac) |
| PD-01 | .claude/worktrees/pd-01 | review-1 ship (26f0b079a); PR #2006, auto-merge on (MG only) |
| PD-02 | .claude/worktrees/pd-02 | fix-2 done (a4d0a0cf5); review-3 (final pass) running. Spec follow-ups (architect stand-in, before PD-05 dispatch): correct the PD-02 Tests bullet in work-packets.md; PD-05 must handle that this failure path writes no launch result record (see PD-02-review-1/2). |
| PD-23 | .claude/worktrees/pd-23 | coder blocked (`display` in Preferences breaks the golden plan bytes, R10); ruling-1 written; implementer (sonnet) applying it |
| PD-38 | (none, RE) | review-2 ship; verdict cursor=none; outputs shipped in the campaign-docs PR; PD-39 re-specced |
| campaign-docs | .claude/worktrees/pd-docs | PR #2005 merged (43ad7f1ef), but its RESUME was 53 lines, which broke the card test on main. Fix PR (branch `resume-cap-fix`, worktree pd-docs-fix) is being shipped. After that: sync the main checkout and keep the ledger on main. |
