# ai-harness-docs: resume card

Ledger: docs/analysis/ai-harness-docs/README.md. Packets: work-packets.md (rules, contract C1-C9, W1-W2) and work-packets-w3.md (W3-W4).

## State (2026-10-18)

- Design done. No packet dispatched yet. Ledger worktree: `.claude/worktrees/ai-harness-docs` (branch ai-harness-docs), committed locally, not pushed.
- No BlockedDecision blocks W1.

## Next action

1. `ops`: ship the ledger worktree as a docs PR and merge it (dispatch rule 1), so packet worktrees contain the packets.
2. `ops`: worktrees ah-01, ah-02, ah-05 from origin/main (one dispatch each, one message).
3. Dispatch AH-01, AH-02, AH-05 (packet-coder) in the background.
4. When AH-02 returns `done` with `Verdict: go` in hook-timing.md: an ah-03 worktree stacked on its branch, then AH-03.
   If AH-02 is `blocked` with no-go: ask the owner (D-AH8) before AH-03.
5. W1 done: one packet-review-fixer over AH-01, 02, 03, 05; then ship (AH-03 with `--base` the AH-02 branch, merge AH-02 first).

## Later waves

- W2: AH-06..AH-09 after W1 merges. AH-06..08 run sync.py; disjoint files.
- W3: AH-12, AH-13, then AH-14 stacked on AH-13. AH-11 waits on #1797 (merged → path A; closed → path B; still open after AH-14 merges → ask the owner).
- W4: AH-15 is the owner's smoke test (ask first); AH-16 close-out.

## Owner actions open

- Post the #1797 reply (pr-1797-reply.md), or OK ops to post it.
- Run, or approve a hands-on run of, AH-15.

## Loop rules (until AH-07/AH-08 land)

- Coder `blocked` with reason `context` → fresh packet-coder, "continuation N of 3", same worknote. Then one Sonnet implementer.
- Review-fixer `recheck` → fresh review-fixer for that packet, up to 3 passes, then architect ruling.
