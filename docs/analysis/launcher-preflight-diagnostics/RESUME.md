# Resume: launcher preflight and actionable failure reporting

Design: [README.md](README.md). Packets: [work-packets.md](work-packets.md), [work-packets-display.md](work-packets-display.md), [work-packets-mac-extras.md](work-packets-mac-extras.md), [work-packets-wave3.md](work-packets-wave3.md). Owner rulings: [OWNER-DECISIONS.md](OWNER-DECISIONS.md) (D1-D5). Live state: [ledger.md](ledger.md).

## State (2026-10-24)

- Autonomous mode (owner order). Every campaign PR gets `Cadacious` as reviewer. darwin host: role agents run as `general-purpose` stand-ins (see ledger.md).
- Merged: wave 1, wave D, wave M (#2022), PD-41 (#2024).
- **Owner order (2026-10-24): no more per-wave PRs.** Everything from here lands as one PR at the end. Branch `launcher-preflight-diagnostics/wave2` (`d6de6d10c`, pushed) is the integration branch. Waves 3-5, close-out and the docs/ledger changes merge into it. The final PR to main tags Cadacious. Wave 2 PR #2028 was closed unmerged.
- Wave 2 (PD-07..PD-12): every packet reviewed to ship, wave review-2 ship.
- Main checkout is stale at `2019922fe`; read specs with `git show origin/main:<path>`. The ledger lives in worktree `pd-ledger` (uncommitted).

## Paused (2026-10-24, owner left)

- Integration tip: wave2 `a42fc9ef8` (main merged in; pushed).
- PD-14: coder done (7274dde74). **Review-1 returned ship**, so it's ready to merge into wave2. A note for the wave 3 review or a fix packet: the shell's `RuntimeError` mapping in `shell/src/host/preflight.rs:178-188` (`Io` and `Cancelled` give `wine_runtime_unavailable`) disagrees with the new engine `RuntimeError::failure()` (`Io` gives `local_io`).
- PD-17: coder done (739bfd9f7). Review-1 not started (held because of the wrap-up): dispatch it first on resume.
- PD-15: coder done (b7ccf5c6e). Review-1 not started (held because of the wrap-up).
- PD-13: coder done (8f4fb6aa7). Review-1 not started (held because of the wrap-up).
- All four packet branches and this ledger branch are pushed as backup (no PR).
- **On resume:** check each worktree for a coder report and a commit. If there's a commit with no report, or a dirty tree, dispatch a fresh coder as a continuation. If PD-14-review-1.md is missing, rerun the review.
- Nothing is merging, and no PR is open.

## Waves

1 -> 2 -> 3; D beside 1-2; M after D; 4 (PD-27, PD-29, PD-30, PD-36, PD-37); 5 gated (PD-19, PD-20, PD-39, PD-40); close-out PD-21.

## Next actions (orchestrator)

1. Wave 3 (PD-13..PD-18): base its packet worktrees on the integration branch, then wave 4, wave 5, close-out PD-21. Keep the per-packet and per-wave review loops.
2. Merge main into the integration branch at each wave boundary, so the final PR stays clean.
3. Fold the docs changes into the integration branch, not a separate PR: ledger, this file, the PD-18 and PD-33 (A1-A3) amendments, the PD-41 and PD-10 ruling amendments, UAT-PD14, and the C-PD5 webview2 wording.
4. At the end: a campaign review, then one PR to main (Cadacious), CI with WG/RG/WR, merge, then retire every campaign worktree.
4. Batch the live UATs into one owner session: UAT-PD3, UAT-PD7, UAT-PD11 (Mac, plus Windows A1), UAT-PD14 (Windows).
5. Parked: the `launch.md` split (over 700 lines), the tooling gaps (mk-worktree base, `ci gate` lacks WG/RG/WR, `uat:launch` on darwin), the PD-07 edges.
