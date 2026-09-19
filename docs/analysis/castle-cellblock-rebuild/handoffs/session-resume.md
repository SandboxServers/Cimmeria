# Castle Cellblock campaign — session resume handoff

> Last updated 2026-09-18 (~11:45 CDT) ahead of an expected usage-limit
> rollover (owner notice: weekly usage ~85%). If this session dies mid-work,
> `/clear` and relaunch from this file rather than replaying the transcript.
> Update this file in place (don't add a second one) whenever the state
> below goes stale.

## Operating mode

Autonomous per-packet pipeline (user instruction: "work autonomously until
all work packets are complete; open PRs at regular intervals; run at least
one independent review round per PR"). Coordinator git operations happen
in the dedicated worktree `.claude/worktrees/ccb-coordinator` (never the
shared primary checkout) — checkout `pkg/<name>` off fresh `origin/main`
per packet, validate (fmt/clippy/check/live-DB), push, `gh pr create`,
run `Skill(code-review high --comment <PR-URL>)`, address findings, merge.

**Owner constraint (relayed 2026-09-18 by peer `cimmeria-f0`):** usage is
scarce. Finish and commit what is closest to done, start no new large
scopes, keep at most 2-3 subagents, commit WIP early. Do not fan out.

## PR / branch state

| Packet | Branch | PR | State |
|---|---|---|---|
| C05 (take-cover objective) | `pkg/c05-cover-objective` | #653 | **Merged** |
| C08b (Straegis scene) | `pkg/c08b-straegis-scene` | #650 | **Merged** |
| C08b doc sync | `docs/mark-c08b-done` | #654 | **Merged** |
| main fmt fix | (peer `cimmeria-e5`) | #658 | **Merged** |
| GC1 (Marsh escort) | `pkg/gc1-marsh-escort` | #655 | **Merged** (2026-09-18T16:32Z, squash `627be660`); all CI green including the review-fix commit `ad4e0748`. |

### GC1 / PR #655 detail (merged)

The independent review round found two real issues, both fixed in
`ad4e0748` and summarised in a PR comment:

1. Chain 1172 (post-death blurb 5859) raced C08b's chain 1161 on the same
   `mission_completed 686` trigger. `delay_ms` is now 10600 (1161's dialog
   2516 is at 10100) and `gc1_escort.rs` pins the exact delay.
2. Chain 1175's comment had the execution order vs. chain 1161's
   `destroy_entity` inverted; corrected.

The earlier "got delay=0" live-DB failure was **transient, not a code
bug**: an isolated rerun passed, then a fresh `reload-db.sh` + all 147
`chain_replay_tests` passed. It coincided with the shared Postgres
crashing (see Infra notes), which is the likely cause. No loader/resolver
change was needed.

## Worktree map

| Worktree | Branch | State |
|---|---|---|
| `.claude/worktrees/ccb-coordinator` | `pkg/gc1-marsh-escort` | Clean apart from this handoff dir (untracked until committed). Everything else is pushed. |
| `.claude/worktrees/ccb-c08b-fixup` | `docs/cellblock-session-resume` | Not needed for the coordinator; check for uncommitted work before removing. |
| `.claude/worktrees/ccb-c07-fixup` | `pkg/c07-accept-blurbs` | Stale (C07 merged); safe to remove once confirmed clean. |

`agent-af06a84d79c558a5c`'s directory may still exist on disk (Windows
"device or resource busy" on delete); retry `rm -rf` later if present.

## Ledger status (`docs/analysis/castle-cellblock-rebuild/work-packets.md`)

Done (merged): C00, C01, C02, C03, C04, C05, C06, C07, C08a, C08b, GC1a,
GC1b-0, GC1b-1, GC1b-2 (GC1a/b-1/b-2 = PR #655, C06 = PR #671; in-client
UAT still pending for all of them).

C06 shipped as chains 1141/1142 on a NEW trigger `player_flanked_npc`
(`npc_flanked` runs its actions on the NPC with player id 0 and cannot
complete a player's objective). Known limits, recorded in the ledger: the
event needs a guard already holding a cover slot, and credits only the
top-threat player, so the objectives may be unreachable in rooms without
cover data (follow-up: cover authoring for the Mess Hall / Hallway05).

Not yet started / still open:

- **Follow-up candidates (not started, not required):** cover authoring
  for Mess Hall / Hallway05 so flank objectives are reachable; issue #656.
- **GC1c** (lockdown VFX) — BlockedEvidence, out of scope.
- **GC2** — chain range 1191-1199 reserved, not scoped. Idle research
  teammates `gc2-item-research` and `gc2-re-itemids` may hold findings —
  check `ListAgents`/message them before re-researching.
- **GC3** (mission-completion XP formula) — BlockedDesign, needs an owner
  decision (D-CB10 default: stays out of scope).
- **C10** (rolling doc sync) — ongoing, not a one-shot packet.

Issue #656 tracks: `advance_step` never sends `ON_OBJECTIVE_UPDATE` for
implicitly completed objectives (forced hand-split chains for missions 688
and 639). Follow-up, not fixed inline.

## Immediate next actions, in order

1. Merge docs PR #672 (UAT guide + ledger sync + this handoff) once its
   checks are green (`gh pr checks 672`; if `CONFLICTING`, merge
   `origin/main` into `docs/cellblock-uat-guide` first).
2. The owner runs the in-client checks in
   [uat-guide.md](../uat-guide.md) (24+ scenarios incl. T29 flank) and
   fills its results table. Failures become issues/fixes; nothing else in
   the campaign is gated on code right now.
3. Remaining ledger items are all blocked: GC1c (client evidence), GC2
   (not scoped), GC3 (owner decision). Do not start them without new
   evidence or a decision.
4. Peer `cimmeria-f0` (Harset coordinator) owns the cross-zone operator
   guide (`docs/analysis/zone-restoration-operator-guide.md` on branch
   `content/harset-wave2`); send it a one-line correction if #671/#672
   change what it says (C06 is now merged; T29 covers it).

## What is gated on the owner in-client (nothing has been run)

All in-client UAT is pending. Milestones are in
[README.md](../README.md#validation-and-uat-gates):

- **M1** — one Prisoner 329 dialog + one Marsh briefing per Jaffa/Human;
  hallway controllers accept once; Region8 pistol guard aggros; Stasis
  Sickness icon on load / cleared on cure; relog at each step.
- **M2** — Frost's Letter in the log; cover indicator shows on vial pickup
  and hides on taking cover; step 2144 needs both objectives; flank
  objectives 2725/2731 (T29; may be unreachable without guard cover).
- **M3** — one prompt per accept; Straegis camera plays once, control
  returns, Marsh gone, 2516 once, then 5859 ~10.6s after scene start;
  relog after the scene does not replay it. For GC1: Marsh follows the
  player topside after the ring hop — watch for the invisible-spawn shape
  of issue #582.
- **M4** — arrive near Gerschon with mission 1360 still active; depends on
  the Castle campaign's CA01.

Owner decision still open: GC3 (XP formula).

## Infra notes

- **Shared Postgres (:5433)** runs from `server/pgdata` in the primary
  checkout: `postgres.exe -D .../Cimmeria/server/pgdata -p 5433`. It
  crashed at 10:24 on 2026-09-18 (exception `0xC000026B`) and was
  restarted ~11:20 with
  `external/postgresql_server/bin/pg_ctl.exe -D <that pgdata> -o "-p 5433" -l server/logs/postgresql.log start`
  (crash recovery is quick to replay, but the initial data-dir fsync took
  a couple of minutes). `db.bat start` does NOT work — it looks for
  `external/postgresql_server/data`, which doesn't exist. If live-DB tests
  fail with "connection refused", check this first.
- **Lane / DB scripts** live in
  `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\` (not in the repo):
  `lane.sh` (counting semaphore, 2 slots, per-worktree `target/` + sccache),
  `reload-db.sh`, `live-db-test.sh`. `live-db-test.sh` takes a plain
  substring filter (e.g. `chain_replay_tests`), NOT a nextest `-E`
  expression — an `-E`-style string matches zero tests and exits 4. This
  worktree's DB is `sgw_ccb_coordinator`.
- Building in this worktree rewrites `Cargo.lock` (a `thiserror` 2.0.19 ->
  2.0.20 bump unrelated to this campaign). `git checkout -- Cargo.lock`
  before committing.
- Sibling campaigns sharing this machine: Harset (`cimmeria-3c` /
  coordinator `cimmeria-f0`), Castle 701-708 (`cimmeria-e5`, handoff at
  `docs/analysis/castle-rebuild/handoffs/session-resume.md`), and this
  Cellblock campaign. Never drop or reload another campaign's DB.
