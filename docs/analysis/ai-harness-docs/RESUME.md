# ai-harness-docs: resume card

Ledger: docs/analysis/ai-harness-docs/README.md. Packets: work-packets.md (rules, contract C1-C9), work-packets-w3b.md (W3b: AH-11 rules, C7b, C10, AH-11a..f) and work-packets-w3.md (W3c, W4).

## State (2026-10-18)

- W1, W2 and W3 (AH-12..14) are merged. The old AH-11 is replaced by the #1797 takeover (D-AH26..34).
- Ledger update: worktree `.claude/worktrees/ah-11-design` (branch ai-harness-docs/ah-11-design), committed locally, not pushed.
- Nothing in flight. No BlockedDecision blocks W3b or W3c.

## Next action

1. `ops`: ship ah-11-design as a docs PR and merge it, so packet worktrees contain w3b.
2. `ops`: worktrees ah-11a..ah-11f from origin/main (branches in each packet), one dispatch each, one message.
3. Dispatch AH-11a..AH-11f (packet-coder) in the background. AH-11a must reach Aura's commit f6f180ef (it fetches `docs/wsl-host-reality` if missing).
4. W3b done: one packet-review-fixer over all six. AH-11e's CI job is proven by its own PR run.
5. Ship order: **AH-11e first**, then AH-11b, c, d (their PRs then run the Linux job), AH-11a, then AH-11f.
   AH-11a uses `--title` and `--body-file` from its packet; the body says "Supersedes #1797", not "Closes".

## Later

- W3c: AH-11g (ops bash path; runs sync.py) and AH-11h (docs) in parallel, after all W3b merges.
- AH-11i: owner posts pr-1797-close.md with the PR numbers filled in, then closes #1797.
- W4: AH-15 (owner; ask D-AH33 first: Linux leg required or best effort), then AH-16.

## Owner questions open

- D-AH33: AH-15 Linux/WSL leg required (recommended) or best effort.
- D-AH29 (default no, non-blocking): a bash path for packet workers and the lane.
- Owner actions: AH-11i reply and close; the AH-15 run.

## Loop rules

- Coder `blocked` with reason `context`: fresh packet-coder, "continuation N of 3", same worknote; then one Sonnet implementer.
- Review-fixer `recheck`: fresh review-fixer for that packet, up to 3 passes, then an architect ruling.
