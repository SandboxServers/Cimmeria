# AH-24 campaign review, pass 2 of 3

Scope: fix commit beec20088 on origin/main 102d15444 (5 files: architect.md, its .codex twin, development-workflow.md, pre-pr-checks.md, rules-and-gotchas.md). Reran: `python tools/agent-skills/sync.py --check` exit 0 ("mirror is current"); `python -m unittest discover -s tools/agent-skills` Ran 84, OK (skipped=1); LF-only lines in the three changed docs: 0 each (CRLF kept). Working tree clean. No new breakage found in the diff: the changes are wording only, and no test reads these lines.

## Verdict

decision

## Pass-1 findings

N1 (RESUME.md and README ledger statuses stale): NOT fixed, declined by the fixer because the brief said status docs are AH-16's. Confirmed still stale: RESUME.md still says "Wave 4 ledger written in the main checkout, uncommitted; nothing of W4 dispatched" and "Next action 1. AH-W40", and README.md still shows AH-W40, AH-11g, AH-11h as Ready. The decline is reasonable as a process rule but it leaves the hazard open: the owner relaunches after W4c (personas changed) and AH-15 is an owner wait of unknown length, so a fresh orchestrator reading the card re-dispatches finished waves. This is a decision: (a) accept the stale card and run AH-16's RESUME rewrite before any relaunch, or (b) make the minimal RESUME.md State/Next action rewrite from pass 1 N1 now in a tiny docs-only commit. Recommend (b): cheaper than a wrong re-dispatch; AH-16 still does the full README status flip.

N2 (bash-first forms on win32): fixed for every site pass 1 named. Verified in the diff: development-workflow.md `ops` merge line leads `ship.ps1 merge` with `ship.sh` named for Linux and WSL; rebase line gives `rebase-pr.ps1`; rm-worktree block is PowerShell with a Linux/WSL comment line and includes `pwsh ...rm-worktree.ps1 --merged-into <PR> <name>...`; the lane block leads `lane.ps1`; the after-merge checklist names `.ps1` first; rules-and-gotchas.md lane, retire and `Set-Location ... (cd <worktree> && on Linux)` lines fixed; pre-pr-checks.md:77 leads `pwsh`. Residuals of the same class that pass 1 did not list and the fix did not touch (low, none blocks; fold into AH-16 or a nit commit):
- docs/agents/development-workflow.md:117-119: the ship code block (fenced `bash`) still leads with `bash tools/build-lane/ship.sh pr ...` and gives `ship.ps1` second, though the line above now says `ship.ps1` first. Swap the order and fence as `powershell`.
- docs/agents/development-workflow.md:127: stacked-PR bullet gives only `ship.sh pr -C ... --base` and `rebase-pr.sh --upstream`; no `.ps1` form named.
- docs/agents/development-workflow.md:262: `tools/build-lane/rm-worktree.sh --merged` with no `.ps1` form (`rm-worktree.ps1` exists).
- docs/agents/rules-and-gotchas.md:96: "ship with `tools/build-lane/ship.sh pr -C <own worktree>`" with no `.ps1` form.
- pre-pr-checks.md:50 and development-workflow.md:125 mention `rebase-pr.sh --upstream` as `ship.py` behaviour; not invocations, fine.
Failure scenario: a win32 ops reading development-workflow.md:119 or :262 copies a `bash ...` line; `shell_guard` blocks it, so the cost is one wasted request, not a wrong action.

N3 (architect return shapes): fixed. architect.md:22 scopes the bare `architect done - <ledger path>` line to "no report path in the brief" and defers to `## Reporting` otherwise; the line 19 `architect blocked - ...` forms are pre-brief platform/tool errors and stay, a defensible reading. `.codex/agents/architect.toml` refreshed; sync --check passes. Soft spot: a blocked architect with a report path still returns the line 19 form (no sha field); acceptable because it only covers platform/tool errors.

## Not-fixed list from pass 1

All still correctly classed as AH-16 or owner items: sibling-campaign cards dispatching `packet-review-fixer` (zero hits remain in .claude/agents, .claude/skills, .codex, docs/agents, AGENTS.md, README.md, CLAUDE.md; hits remain only in sibling docs/analysis); agent-memory Git Bash notes; `rm-worktree.sh` missing from the `shellcheck -S warning` list; S4 follow-ups and no test guarding verdict words, return-line formats or "3 per loop"; stale generated count blocks; unified-uat.md at 1080 lines (pre-existing). No new items.

## Open items (current)

AH-15 (owner, fresh-clone smoke test). Relaunch first (personas changed; W4c is the last persona change). Windows W1-W10: W8 README read on GitHub with the Mermaid diagram rendering; W9 `/agents` lists `packet-reviewer` and `packet-coder` and no `packet-review-fixer`; W10 ask "How does review work here?" and compare to the README. Linux/WSL leg L1-L7, including C17 through a `Host: linux` packet. Open owner decision D-AH33: Linux leg required (recommended) or best effort. Until it runs, docs must keep saying "no full session has run on Linux/WSL". New: settle the N1 decision before the relaunch.

AH-16 (close-out): run regen; project-status AI harness line (work-packets-w4b.md:144); unified-uat W1-W10 entry (decide on splitting unified-uat.md); rewrite README ledger statuses to `Integrated (#NNNN)` and RESUME (unless N1 is done earlier); retire `ah-11p`, then `rm-worktree.ps1 --merged`, then `--merged-into <W4a PR> ah-17 ah-18 ah-19 ah-20 ah-21` and `--merged-into <W4c PR> ah-25 ah-26 ah-27`; mark S1 "Not done: owner veto D-AH57", S2 "Done, folded into AH-23a", S4 Done (#1873); list the Not-fixed items and S4 follow-ups; sweep or accept the four bash-first residuals above; confirm persistent-player-data's coordinator was told (F39); board handoff; empty `.claude/state/active-campaign`.