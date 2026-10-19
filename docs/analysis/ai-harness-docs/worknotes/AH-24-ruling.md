# AH-24 ruling (campaign review, after pass 2 verdict=decision)

Inputs: AH-24-review-1.md, AH-24-review-2.md (this folder). Base: `ah-campaign-review` at beec20088 on origin/main 102d15444. The architect writes no fix. This note says what changes and who changes it.

## Ruling 1: N1, option (b), the minimal RESUME.md rewrite now

Option (b). The owner relaunch comes next, and AH-15 is an owner wait of unknown length. A fresh orchestrator reading the current card sees "nothing of W4 dispatched" and "Next action 1. AH-W40", so it would re-dispatch waves that already merged. Fixing that costs one edit; a wrong re-dispatch costs a wave.

Scope:
- Only `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\ai-harness-docs\RESUME.md`, in the main checkout, left uncommitted like the rest of the local ledger edits. AH-16 commits it with the final rewrite.
- `README.md` ledger statuses stay as they are; AH-16 flips them. The card says so in plain words, so nobody plans from the table.
- **Who edits:** the orchestrator. It owns RESUME.md at wave boundaries, and a worker writes only inside its own worktree (D-LO16). Use **Edit**, not Write, so the file keeps its CRLF line endings.

Edit A: insert one line after the `Ledger:` paragraph (line 3), followed by a blank line:

```
**The README packet table is stale until AH-16 (AH-W40, AH-11g/h show Ready; W4 rows show BlockedDependency). Plan from this card, not the table.**
```

Edit B: replace everything from `## State (2026-10-18)` through the end of the `## Next action` list (old lines 5-17) with:

```
## State (2026-10-18)

- Merged: W1-W3d (#1805-#1863) incl. W3c (AH-11g #1860, AH-11h #1863); W4a #1865 (+#1866); S6 #1868; S5a #1870; S5b #1872; S4 #1873; W4b #1874; S5c #1877; S3 #1878; W4c #1879 (main 102d15444). AH-11i done (#1797 closed). S1 vetoed (D-AH57); S2 folded into AH-23a.
- Owner decisions D-AH50..D-AH58 are recorded in the ledger. W4c was the last persona change: the owner relaunches Claude Code before AH-15.
- AH-24 campaign review runs in worktree `ah-campaign-review`: pass 1 fix beec20088, pass 2 `decision`, ruled in AH-24-ruling.md. Its reports land in `worknotes/AH-24-*.md` with the AH-24 PR.
- **Gate (rule 0, D-AH55):** nothing merges while `main` is red; recheck before each merge.
- **Review loop (C12):** never dispatch `packet-review-fixer`. A read-only `packet-reviewer` reviews; a fresh `packet-coder` applies each fix pass; at most 3 passes.

## Next action

1. AH-24: one Sonnet implementer applies the ruling edits in `ah-campaign-review`, then a fresh packet-reviewer runs pass 3. On `ship`, `ops` ships the docs PR and retires the worktree.
2. Owner relaunches Claude Code (personas changed in W4c).
3. AH-15 (owner): fresh-clone smoke test, Windows W1-W10 and Linux/WSL L1-L7 (D-AH33).
4. AH-16 close-out: the list in worknotes/AH-24-review-2.md § Open items (regen, README table flip, this card, retire ah-11p, ah-17..21 and ah-25..27, S1/S2/S4 rows, the Not-fixed list, F39 check, board handoff, empty `.claude/state/active-campaign`).
- Owner, if not done yet: tell persistent-player-data's coordinator that C12 replaced "one review-fixer each" (F39).
```

Keep `## Owner questions open` (D-AH33 line) and `## Loop rules (C12)` unchanged. The result is about 34 lines, under the 40-line cap.

Verify (the pass-3 reviewer, read-only): the main-checkout RESUME.md is at most 40 lines and has no LF-only line, `nothing of W4` and `AH-W40` appear only in the stale-table warning, and `python -m unittest discover -s tools/agent-skills -p "test_ledgers.py"` passes from the main checkout.

## Ruling 2: fix the bash-first residuals now, in this loop

Fix them now, with no deferral to AH-16. AH-15 is a fresh-clone smoke test of `main`, and the relaunched orchestrator and `ops` read these docs. They are wording edits, the same class pass 1 already fixed, in the same three files, so one small implementer pass covers them. The list below is closed: the implementer does not sweep for more. The Linux section (development-workflow.md:266 onward) and descriptive mentions of a `.sh` script are correct as written.

**Who:** one Sonnet implementer in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ah-campaign-review`, as one commit on top of beec20088. It runs every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ah-campaign-review;` first. PowerShell only. It never runs cargo (there is no compile step here). Files under `docs/` are CRLF, so edit with Edit and never rewrite a whole file with Write.

Line numbers refer to beec20088.

E1. `docs/agents/development-workflow.md:117-122` (the ship code block). Change the fence to `powershell` and put the PowerShell form first: `$env:SHIP_TRAILERS = "Co-Authored-By: ...`nClaude-Session: ..."`, `$env:SHIP_PR_FOOTER = '...'`, then `pwsh tools/build-lane/ship.ps1 pr -C <its worktree> -m "<message>" [--body-file <file>]`. Then a comment line `# On Linux and WSL, with bash:` and the two existing `SHIP_TRAILERS=$'...' ... bash tools/build-lane/ship.sh pr ...` lines. Delete the old `# From PowerShell: ...` comment.

E2. `docs/agents/development-workflow.md:125` (stacked PRs). Change it to `pwsh tools/build-lane/ship.ps1 pr -C <worktree> -m <msg> --base <the other PR's branch>` (`ship.sh` on Linux and WSL). Change `ship.sh merge refuses it` to `` `ship.ps1 merge` (or `ship.sh merge`) refuses it ``. Replace the parenthetical `rebase-pr.sh --upstream <the base's merged head> --push` with `rebase_pr.py --upstream <the base's merged head> --push`; that is what `ship.py` calls (ship.py:69, :429). Leave the rest of the bullet alone.

E3. `docs/agents/development-workflow.md:129`. Change `Ship with `ship.sh` (above)` to `Ship with `ship.ps1` (`ship.sh` on Linux and WSL; above)`.

E4. `docs/agents/development-workflow.md:262`. Change it to start `` `pwsh tools/build-lane/rm-worktree.ps1 --merged` (`rm-worktree.sh --merged` on Linux and WSL) retires every merged, idle worktree... ``. Leave the rest of the line alone.

E5. `docs/agents/development-workflow.md:264`. Change `Run it under `lane.sh --exclusive`` to `Run it under `pwsh tools/build-lane/lane.ps1 --exclusive``. `measure-build.ps1` is a PowerShell script anyway.

E6. `docs/agents/rules-and-gotchas.md:96`. Change `ship with `tools/build-lane/ship.sh pr -C <own worktree>`` to `ship with `pwsh tools/build-lane/ship.ps1 pr -C <own worktree>` (`ship.sh` on Linux and WSL)`. The two `rm-worktree.sh` mentions earlier on that line describe the 2026-10-03 prune incident, so they stay.

E7. `docs/agents/pre-pr-checks.md:162` (the `unpaired_id` re-bless). Review 2 did not list this one, but it is the same class and a real trap: the only form given is `NT_BASELINE_BLESS=1 bash tools/build-lane/lane.sh ...`. Lead with `` `$env:NT_BASELINE_BLESS = '1'; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server unpaired_id` (then `Remove-Item Env:NT_BASELINE_BLESS`) ``, then `or `NT_BASELINE_BLESS=1 bash tools/build-lane/lane.sh ...` on Linux and WSL`. Leave the rest of the bullet alone.

E8. Keep the reports. The worktree's reports are deleted when it retires, and AH-16 needs the pass-2 Open items list. Copy `AH-24-review-1.md`, `AH-24-review-2.md` and `AH-24-ruling.md` from `.claude\reports\ai-harness-docs\` to `docs\analysis\ai-harness-docs\worknotes\` under the same names, converted to CRLF (`[IO.File]::WriteAllText` with an absolute path and `` `r`n `` joins). Add them to the same commit.

Out of scope, leave as is: pre-pr-checks.md:50 and development-workflow.md:125's restack wording outside E2 (they describe `ship.py`'s behaviour, not commands to run), :188/:255/:256 (the `.ps1` form already comes first), :213 and :247 (they name the script, not a command line), and the CI-mirror block at pre-pr-checks.md:105-120 (it already names the PowerShell twins).

Commit message: `docs(agents): Windows forms lead in the remaining ship/lane/retire lines; AH-24 reports to worknotes (AH-24 review fix 2)`, with the session trailers from the brief. The implementer commits locally and does not push; `ops` ships after pass 3.

Implementer verify:
- `python tools/agent-skills/sync.py --check` exits 0.
- `python -m unittest discover -s tools/agent-skills` passes.
- Each edited `.md` file and the three worknotes have 0 LF-only lines.
- Grep the three docs for `bash tools/build-lane`. Every hit must sit inside the Linux section (line 266 onward) or next to a `pwsh` form, or carry an "on Linux and WSL" label.
- `git -C <wt> diff --stat HEAD~1` shows only the three docs and the three worknotes.

## Then

Pass 3 is a fresh read-only packet-reviewer over beec20088 plus the new commit, and it also checks the main-checkout RESUME.md (Ruling 1 verify). If pass 3 says `fix` or `decision`, the loop is spent: anything left goes to AH-16 as a listed item, with no new loop. If it says `ship`, `ops` ships `ah-campaign-review` as a docs PR (`ship.ps1 merge <PR> --retire ah-campaign-review`). Then the owner relaunches, and then AH-15.

BlockedDecision: none new. D-AH33 is still open for the owner.
