# ai-harness-docs: resume card

Ledger: README.md (F1-F56, D-AH1-D-AH58). Packets: work-packets-w4.md (dispatch rule 0, R1-R11, C12-C16, AH-W40, W4a, ship procedure), work-packets-w4b.md (order, AH-23a/b, W5, S2-S6), work-packets-s4.md (C19, S4a-d), work-packets-w4c.md (C17, AH-25..27), work-packets-w3.md (W3c AH-11g/h/i, AH-15, AH-16). Drafts: w4-drafts.md, w4b-drafts.md. Owner answers 2026-10-18: w4-owner-answers-report.md, w4-s1s4-report.md.

**The README packet table is stale until AH-16 (AH-W40, AH-11g/h show Ready; W4 rows show BlockedDependency). Plan from this card, not the table.**

## State (2026-10-18)

- Merged: W1-W3d (#1805-#1863) incl. W3c (AH-11g #1860, AH-11h #1863); W4a #1865 (+#1866); S6 #1868; S5a #1870; S5b #1872; S4 #1873; W4b #1874; S5c #1877; S3 #1878; W4c #1879 (main 102d15444). AH-11i done (#1797 closed). S1 vetoed (D-AH57); S2 folded into AH-23a.
- Owner decisions D-AH50..D-AH58 are recorded in the ledger. W4c was the last persona change: the owner relaunches Claude Code before AH-15.
- AH-24 campaign review is done and merged with the campaign-review docs PR (pass 1 fix, pass 2 `decision`, architect ruling applied and checked). Its reports are in `worknotes/AH-24-*.md`.
- **Gate (rule 0, D-AH55):** nothing merges while `main` is red; recheck before each merge.
- **Review loop (C12):** never dispatch `packet-review-fixer`. A read-only `packet-reviewer` reviews; a fresh `packet-coder` applies each fix pass; at most 3 passes.

## Next action

1. Done: AH-24 (worktree retired after merge).
2. Owner relaunches Claude Code (personas changed in W4c).
3. AH-15 (owner): fresh-clone smoke test, Windows W1-W10 and Linux/WSL L1-L7 (D-AH33).
4. AH-16 close-out: the list in worknotes/AH-24-review-2.md § Open items (regen, README table flip, this card, retire ah-11p, ah-17..21 and ah-25..27, S1/S2/S4 rows, the Not-fixed list, F39 check, board handoff, empty `.claude/state/active-campaign`).

- Owner, if not done yet: tell persistent-player-data's coordinator that C12 replaced "one review-fixer each" (F39).

## Owner questions open

- D-AH33: AH-15 Linux/WSL leg required (recommended) or best effort. (S1 vetoed; S4 approved.)

## Loop rules (C12)

- Coder `blocked` with reason `context`: fresh packet-coder, "continuation N of 3"; then one Sonnet implementer.
- Reviewer `verdict=fix` at pass 1 or 2: fresh packet-coder, "review fix, pass N of 3"; then a fresh reviewer.
- `fix` at pass 3, or `decision`: architect ruling note only (writes no code or fix), one Sonnet implementer, one fresh reviewer.
- S4 briefs: never write the fix-pass phrase with a digit except in a real fix pass (C19 item 6).
