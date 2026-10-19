# wave4a ruling (C12 step 3)

Architect ruling after wave4a review pass 3 returned `fix`. Range a8910a384..19f8c6a08 in the worktree (`.claude\worktrees\ah-w4a`) (branch `ai-harness-docs/w4a`). Inputs: `.claude\reports\ai-harness-docs\wave4a-review-1.md`, `-2.md`, `-3.md`, `wave4a-fix-1.md`, `wave4a-fix-2.md`.

## Ruling

**fix (re-spec of the one open note).** Pass 3 N1 [AH-19] is verified: `.claude/agents/orchestrator.md:32` (and its generated twin `.codex/agents/orchestrator.toml:23`) stops at "then one fresh reviewer to check it" and never says what a failed ruling check means. The skill (`.claude/skills/campaign-packet/SKILL.md:203-204`), C12 step 3 in `work-packets-w4.md` and `docs/agents/ai-harness.md:133` all say it is BlockedDecision. The orchestrator acts from its persona line, so this is a real stop missing, not wording. The fix is one clause on one line. No re-slice and no owner question.

## Implementer

One Sonnet `documentation-writer` in the worktree above. Brief: this note's path, report path `.claude\reports\ai-harness-docs\wave4a-ruling-1.md`, return line `wave4a <done|blocked|failed> <commit-sha|-> <report-path>`.

## Edit 1 (the only hand edit)

- File: `.claude\agents\orchestrator.md` (the worktree copy, never the root checkout), line 32 (the `- Review loops (campaign-packet skill § 4; ...` bullet).
- Find (exact, once in the file):
  `then one fresh reviewer to check it. The architect decides only what to fix and how`
- Replace with:
  ``then one fresh reviewer to check it (a ruling check that is not `ship` is BlockedDecision). The architect decides only what to fix and how``
- Keep it on the same line. The file must stay 72 lines. Keep the file's existing line endings (do not convert). Touch nothing else in the line or file.

## Edit 2 (generated, do not hand edit)

- Run from the worktree: `Set-Location <worktree>; python tools/agent-skills/sync.py`
- It rewrites `.codex/agents/orchestrator.toml` (line 23 gets the same clause). Do not edit the .toml by hand.

## Checks (all must pass, in the worktree)

1. `Select-String -Path .claude\agents\orchestrator.md,.codex\agents\orchestrator.toml -Pattern 'ruling check that is not' -SimpleMatch` returns exactly 1 line per file.
2. `(Get-Content .claude\agents\orchestrator.md).Count` is 72.
3. `python tools/agent-skills/sync.py --check` exits 0.
4. `python -m unittest discover -s tools/agent-skills -p "test_*.py"` passes (79 tests, 1 skipped at pass 3).
5. `git status --short` lists only `.claude/agents/orchestrator.md`, `.codex/agents/orchestrator.toml`, this worknote and the ruling report before commit.

No cargo and no lane call: no Rust is touched.

## Commit

One commit, explicit paths: `.claude/agents/orchestrator.md`, `.codex/agents/orchestrator.toml`, `docs/analysis/ai-harness-docs/worknotes/wave4a.md`. Message: `AH-W4a wave ruling: failed ruling check is BlockedDecision in the orchestrator persona`. Do not push. The `.claude/reports/` file is not committed.

## Ruling check

Then one fresh `packet-reviewer`, scope wave4a, range `19f8c6a08..<ruling commit>`, no pass number, names this note. Report: `.claude\reports\ai-harness-docs\wave4a-ruling-check.md`. It checks only that Edit 1 and 2 match this note and checks 1-4 pass. A verdict other than `ship` is BlockedDecision (C12 step 3); do not dispatch another ruling.

## Pass 3 "Not fixed" items: accepted, with follow-ups

These do not block W4a. The orchestrator records them in the ledger.

- `<N>` in `ruling-<N>` undefined (SKILL.md:142, orchestrator.md:43, AGENTS.md:33): accepted as is. C12 allows one implementer per ruling, so N is 1.
- README.md:63, :70 and AGENTS.md:34 summaries without the fix-pass or failed-check clause: accepted. They point at the skill and ai-harness.md.
- Sibling campaigns that still dispatch `packet-review-fixer` (lean-orchestration, campaign-metrics, lab-cli-rust, any-install-adoption, lab-harness, dev-workbench; plus dakara-e1-rebuild, persistent-player-data, player-capture-bots outside this tree): orchestrator action per `work-packets-w4.md` dispatch rule 2, before W4a merges. Not a packet.
- `tools/token-profile` costing fix-pass `packet-coder` sessions apart from first implementation (D-AH42 kept the old name on purpose): follow-up packet, to be sliced into a later wave.
- No test guards verdict words, return-line formats or `3 per loop` (test_agent_refs.py pins names only): follow-up packet, to be sliced into a later wave alongside the token-profile one.