# player-capture-bots: resume card

Ledger: `README.md`. Packets: `work-packets.md`, then `-w1`, `-w2`, `-w3-camera`, `-w3-join`, `-w3b-docs-gates`, `-w4-tools`. Ideas: `IDEAS.md`. Reports: `architect-revision-report.md`, `architect-amend-report.md`.

## State (2026-10-18 evening: waves 1 and 2 MERGED; wave 3a starting; design amended to the owner's rulings)

- Wave 1 merged: #1801 (PC-01+02), #1802 (PC-11), #1803 (PC-10), #1804 (PC-06, verdict `S-CAM: A`). Wave 2 merged: #1813 (PC-03), #1814 (PC-04), #1815 (PC-14a+b), #1816 (PC-12+13).
- Wave 3a coded and reviewed; PRs open, CI pending: #1821 (PC-15+16+16b, `pc-15`), #1822 (PC-26, `pc-26`), #1823 (PC-27+17, `pc-17`). Next: merge each when green (`ship merge <PR> --retire <wt>`, then retire `pc-27`), then wave 3b (PC-05, PC-18). Colo `/release` for PC-07 is pre-authorized (ops runs it now).
- Leftover worktrees to clear (NO_PR parents refuse retire): `pc-01`, `pc-12`, `pc-14a`, old `pc-06` (bad base), plus any `pc-02/03/04/11/13/14b` that another rm-worktree was retiring. Check with ops `worktree list`.
- Known gap (PC-12): a teleport is only seen at the next ring sample (>= 1 s); fold a distance check into capture_interaction_pose in a follow-up.
- Amendment: consent is closed (D-PC17 withdrawn, no disclosure edit); releases are pre-authorized (D-PC22); friction goes to SigNoz only (PC-11); the camera joins its pose row by Mercury sequence, then server time (D-PC18 to D-PC20). Wave 3 splits into 3a (code) and 3b (docs) (D-PC21). No BlockedDecision.

## Next action

1. Finish wave 1: PC-02 after PC-01, one packet-review-fixer, then four PRs (PC-01+02 from `pc-02`, PC-06, PC-10, PC-11). Keep PC-06's `S-CAM: A|B` line.
2. Wave 2 from `origin/main` after wave 1 merges: PC-03, PC-04, PC-12, PC-14a; then PC-13, PC-14b.
3. After wave 2 merges: ops `/release` (procedure below), then PC-07.
4. Wave 3a (six packets, one review): PC-15, PC-26, PC-27 at once; PC-16 then PC-16b after PC-15; PC-17 after PC-27. PRs: PC-15+16+16b from `pc-16b`, PC-26, PC-27+17 from `pc-17`. server-authority-enforcer reads PC-27 and PC-17.
5. Wave 3b after 3a merges: PC-05 and PC-18 (documentation-writer).
6. PC-19 (ask the user first), then ops `/release-launcher` and `/release`, then PC-07b.
7. Wave 4a (PC-20, PC-22, PC-23) after PC-07; 4b (PC-21, PC-21b, PC-21c as one PR; PC-24) after 4a merges and PC-07b. Then PC-25 (ask first) and PC-09.

## In flight

| Packet | Worktree | Status |
|---|---|---|
| PC-01, PC-06, PC-10, PC-11 | `pc-01`, `pc-06`, `pc-10`, `pc-11` | wave 1 dispatched |

## Gates and owner items

- **Releases are pre-authorized:** ops posts from PowerShell, `gh pr comment <PR> --body "/release"` (or `/release-launcher`), never Git Bash, then checks the comment and the `release-on-comment.yml` / `release-container.yml` / `launcher-release.yml` runs. If ops refuses `gh pr comment`, a general-purpose agent runs it.
- **Ask the user** before lab use (PC-19, PC-25 calibrate) and colo export (PC-25).
- PC-11: tell the user when it merges; the live "Playtest: friction" saved view may filter on WARN, and only the owner edits it.
- D-PC10 (content names on rows) and D-PC12 stand unless the owner vetoes them.

## Rules to remember

- No account names, character names, IPs or hosts on any row, test or doc. Never run `git worktree prune`. Docs are CRLF.
