# Harset coordinator resume (session 1, written 2026-09-18 before the session-limit rollover)

Read this first in a fresh session; it replaces the conversation context. Ledger: [work-packets.md](../work-packets.md) (status lines are current). Decisions: README D-H19, D-H20.

## Branches, worktrees, PR

| Thing | Where |
|---|---|
| Wave 1 PR | **#662** `content/harset-rebuild` (CI fully green at 2b05e02f; Copilot review 5 findings + code-review round 10 findings, all dispatched to fix workers below; CodeRabbit skipped: >100 files, no credits) |
| Wave 2 base | `content/harset-wave2` @ 372b49ae = wave 1 + `origin/main` (784e0425 + C05 43bfbca5) + tag registry `worknotes/harset-tags.md` + empty faction seed files + ledger |
| Coordinator worktree | `.claude/worktrees/harset-integration` (currently on `content/harset-wave2`; never use the root checkout) |
| Fix worktrees (PR #662) | `.claude/worktrees/harset-review662` branch `harset/review-662` (Copilot 1-4 + round-2 items 5-8: ring readiness, back-pointer abort, ring arrival validate-only, shared respawner helper) and `harset-review662b` branch `harset/review-662-b` (kill_credit health seam, entities.rs player gate, template mapper dedupe, wrapper removal, drop `respawn_secs` from spawn_entity, split conditions tests) |
| Wave-2 worktrees | `harset-H06` (known_stargates + region world/containment guard), `harset-H08` (Submit cleanup), `harset-H09` (multi-ability sets), `harset-jaffa` (H20 1324, H22 1326), `harset-opcore` (H30 1360/567, H31 1361), `harset-goauld` (H40 1200, H41 742; D-H20 offer chains), `harset-H50` (objective persistence, repo-wide) — branches `harset/<name>` |
| Live test DB | `sgw_harset` on :5433 (reload from `db/database.sql` after merging seed changes; do not touch `sgw`) |
| Build lane | `/c/Users/Steve/AppData/Local/Temp/cimmeria-castle/lane.sh cargo ...` (2-slot semaphore, per-worktree target dir, sccache) |

## Next actions, in order

1. For each worker branch above: `git -C .claude/worktrees/<wt> log --oneline content/harset-wave2..HEAD` (or `..content/harset-rebuild` for the two fix branches) and `git status --short`; anything uncommitted is finishable by a fresh `rust-gameserver-dev` agent told to work in that worktree (prompts are reconstructible from the ledger packet sections; worknotes drafts may exist under `worknotes/`).
2. Merge `harset/review-662` then `harset/review-662-b` into `content/harset-rebuild`, run `lane.sh cargo fmt --all -- --check`, clippy, targeted tests, reload `sgw_harset`, push, reply on PR #662 to each Copilot/review comment with the resolution, request re-review, merge when green.
3. Merge `content/harset-rebuild` forward into `content/harset-wave2`; integrate wave-2 branches one at a time (seed lanes last; `chain_replay_tests/mod.rs` `mod` lines conflict trivially; keep alphabetical), reload `sgw_harset`, validate, open PR "wave 2".
4. Castle CA10 = PR #663 (splits `gate_travel.rs` into a dir, owns stargate region routing). **Resolved 2026-09-18:** `origin/main` was merged into `content/harset-rebuild` and H01's single `validate_gate_arrival` call now lives at the top of `cell::gate_travel::perform_gate_travel` — the one function both the volume-entry crossing and the no-gate-volume immediate fallback funnel through. Do not add a second call.
5. Still M0-gated (needs the user in-client): H12, H14, H15, every spawn-position mission packet; respawner rows 20/22/23; gate-3 arrival; chain 6007 enable.
