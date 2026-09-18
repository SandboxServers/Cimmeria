# Castle Cellblock campaign — session resume handoff

> Written 2026-09-18 ahead of an expected session-limit rollover. If this
> session dies mid-work, `/clear` and relaunch from this file rather than
> replaying the full transcript. Update this file (don't append a second
> one) whenever the state below goes stale.

## Operating mode

Autonomous per-packet pipeline (user instruction: "work autonomously until
all work packets are complete; open PRs at regular intervals; run at least
one independent review round per PR"). Coordinator git operations happen
in the dedicated worktree `.claude/worktrees/ccb-coordinator` (never the
shared primary checkout) — checkout `pkg/<name>` off fresh `origin/main`
per packet, validate (fmt/clippy/check/live-DB), push, `gh pr create`,
run `Skill(code-review high --comment <PR-URL>)`, address findings, merge.

## PR / branch state (as of this writing)

| Packet | Branch | PR | State |
|---|---|---|---|
| C05 (take-cover objective) | `pkg/c05-cover-objective` | #653 | **Merged** (2026-09-18T12:09:38Z), incl. a review-fixup commit (self-completion guards on chains 1033/1132 + `docs/content/mission-chains.md` sync) |
| C08b (Straegis scene) | `pkg/c08b-straegis-scene` | #650 | **Merged**, incl. a review-fixup commit widening `destroy_entity`→`despawn_npc` to two more call sites |
| C08b doc sync | `docs/mark-c08b-done` | #654 | **Merged** |
| main fmt fix | (peer session `cimmeria-e5`'s PR) | #658 | **Merged** — main's `cargo fmt --check` was red from #650 briefly; fixed, both `pkg/c05-cover-objective` and `pkg/gc1-marsh-escort` were rebased onto the fix afterward |
| GC1 (Marsh escort) | `pkg/gc1-marsh-escort` | #655 | **Open, mid-fix.** Two real findings from an independent review round: (a) chain 1172's `delay_ms` needed bumping from 0 to 10600 so its dialog doesn't race C08b's chain 1161 Matinee/dialog-2516 beat on the same trigger — **fix applied** to both the SQL and a tightened `gc1_escort.rs` test asserting the exact delay; (b) chain 1175's comment had an inverted claim about execution order vs. C08b's chain 1161 — **fixed**. **UNRESOLVED right now:** after applying the SQL fix, a live-DB run of the new/tightened test (`chain_1172_mission_686_complete_shows_post_death_dialog_5859`) FAILED with "got delay=0" even though a direct `psql` query against the same reloaded `sgw_ccb_coordinator` DB immediately afterward showed `delay_ms=10600` correctly stored. Root cause not yet found — was mid-isolating via a single-test rerun (`cargo nextest run ... -E "test(chain_1172_mission_686_complete_shows_post_death_dialog_5859)"`, `DATABASE_URL=...sgw_ccb_coordinator`, no reload) when this handoff was written. **Next action: read that rerun's result; if it now passes, it was a transient/stale-read issue during the reload+test combo and is safe to just re-verify with a fresh `reload-db.sh` + full `gc1_escort`/`mission_686_straegis` run before pushing. If it still fails, the bug is real — trace `load_single_chain_for_test` → `build_chains_from_rows` → `resolve_event`'s `action_delays` handling for a single-action chain with nonzero delay; none of that code showed an obvious bug on inspection, so the next step is adding an inline `eprintln!`/`dbg!` in the test to print `resolved.action_delays` raw, or checking whether nextest reused a stale test binary (rare, but `cargo clean -p cimmeria-services` would rule it out).** Do not push to `pkg/gc1-marsh-escort` until this is green.

Issue #656 filed: `advance_step` never sends `ON_OBJECTIVE_UPDATE` for
objectives it implicitly completes, plus the same root cause has now
forced hand-split content chains twice (mission 688, mission 639) with no
general primitive proposed. Tracked as a follow-up, not fixed inline.

## Worktree map

| Worktree | Branch | Purpose / state |
|---|---|---|
| `.claude/worktrees/ccb-coordinator` | `pkg/gc1-marsh-escort` | Active coordinator worktree. Currently mid-debug on the delay_ms test failure above. Uncommitted changes: SQL fix for chains 1172/1175 comments, `gc1_escort.rs` test tightening — **not yet committed** (staged mentally, not via `git add`) as of this handoff. |
| `.claude/worktrees/ccb-c08b-fixup` | `pkg/c05-cover-objective` (stale, already merged) | No longer needed — safe to `git worktree remove` once confirmed nothing uncommitted remains (last known state: clean, matches merged PR #653 content). |

Six other stale agent worktrees (`agent-a1823739c7a3597b4`,
`agent-a20d59dfd37999cdc`, `agent-a7528cb59dea8570b`,
`agent-a86f727800dba0af5`, `agent-acadac2b412688f44`,
`agent-af06a84d79c558a5c`) were verified fully superseded by merged PRs
and removed during this session at peer `cimmeria-e5`'s disk-space
request (C: was at 98%; back to ~225GB free). One
(`agent-af06a84d79c558a5c`) is unregistered from `git worktree list` but
its directory wouldn't delete (Windows "device or resource busy") — retry
`rm -rf` on it later if it's still present.

## Ledger status (`docs/analysis/castle-cellblock-rebuild/work-packets.md`)

Done (merged): C00, C01, C02, C03, C04, C05, C07, C08a, C08b, GC1b-0.
GC1 (a/b1/b2) implemented and in PR #655, not yet marked Done in the
ledger (mark it Done + PR #655 once merged, matching the pattern used
for every other packet's docs commit).

Not yet started / still open:
- **C06** (flanking objectives 2725/2731) — was blocked on C05's pattern;
  C05 is merged now, so C06 is unblocked. Not yet dispatched.
- **GC1c** (lockdown VFX) — BlockedEvidence, explicitly out of scope per
  the ledger (no energy-field actor or Kismet event id recovered).
- **GC2** — chain range 1191-1199 reserved, not yet scoped/dispatched.
  Two idle research teammates from an earlier turn may already have
  findings: `gc2-item-research` (items-systems-advisor) and
  `gc2-re-itemids` (game-archaeology-specialist) — check
  `ListAgents`/message them before re-researching from scratch.
- **GC3** (mission-completion XP formula) — BlockedDesign, no user
  decision recorded yet (D-CB10 default: stays out of scope).
- **C10** (rolling doc sync) — ongoing, not a one-shot packet.

Two other idle research teammates exist from earlier work, already spent
(their findings are presumably folded into GC1/C07, now merged):
`gc1-escort-research` (npc-ai-spawn-advisor), `c07-precheck-research`
(game-archaeology-specialist). Don't re-dispatch unless their prior
findings can't be located.

## Immediate next actions, in order

1. Read the result of the in-flight isolated test rerun for
   `chain_1172_mission_686_complete_shows_post_death_dialog_5859`.
2. If it passes: run a full fresh `reload-db.sh` + `cargo nextest ... -E
   "test(gc1_escort) or test(mission_686_straegis)"` to confirm no
   regression, then `git add`/commit/push the fix to `pkg/gc1-marsh-escort`,
   comment on PR #655 summarizing the two fixes, and merge once CI is
   green (checked with `gh pr checks 655`).
3. If it still fails: debug per the note above before pushing anything.
4. Mark GC1 (a/b1/b2) Done in `work-packets.md` with PR #655's number, as
   its own small docs commit/PR (matching the C08b-Done-doc pattern, PR
   #654) — or fold into the same push if convenient.
5. Dispatch C06 (now unblocked) via a fresh `pkg/c06-<name>` branch
   following the same pipeline.
6. Check in on GC2's two idle research teammates before deciding whether
   to scope/dispatch GC2.
7. Continue the "open PRs regularly, one review round each" cadence until
   the ledger's remaining packets (C06, GC2 if scoped, C10) are done. GC1c
   and GC3 stay out of scope pending new evidence / a user decision.

## Cross-session context

Three sibling campaigns share this machine's build lane and disk:
Harset (`cimmeria-3c`), Castle 701-708 (`cimmeria-e5`, handoff at
`docs/analysis/castle-rebuild/handoffs/session-resume.md` on branch
`castle/coordinator-session-resume`), and this Cellblock campaign. Build
infra changed mid-session (peer `cimmeria-e5`): `lane.sh` is now a
counting semaphore (not a hard mutex) with per-worktree `target/` dirs +
sccache, and `reload-db.sh`/`live-db-test.sh` now target a
per-worktree database `sgw_<worktree-dir-name>` (this worktree:
`sgw_ccb_coordinator`) instead of one shared `sgw` DB — same entry
points, just no longer need to worry about another session's reload
landing under your tests.
