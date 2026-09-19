# Harset coordinator resume (session 3, written 2026-09-19)

Read this first. [session-1-resume.md](session-1-resume.md) still has the branch and worktree map; [session-2-resume.md](session-2-resume.md) has the PR #662 review history. Ledger: [work-packets.md](../work-packets.md). Operator guide: [../../zone-restoration-operator-guide.md](../../zone-restoration-operator-guide.md). This file lives on `content/harset-wave2`; the copy of the handoffs on `main` is stale.

## Where things stand

| Thing | State |
|---|---|
| PR #662 `content/harset-rebuild` (wave 1) | Head `440a75af`, all 11 checks green, `CLEAN`. `origin/main` merged through #680. **Merging waits on the owner's explicit go-ahead.** Prefer a merge commit (review replies quote fix SHAs). Keep `entity_templates` `setval` = 248 in any further merge |
| `content/harset-wave2` (wave-2 base, worktree `harset-integration`) | `ac1b991f` = wave 1 at `440a75af` + H51. Not pushed, no PR yet |
| `harset/H51` (worktree `harset-H51`) | DONE and merged into the wave-2 base. Full suite on its own DB: entity 258, services 2484, 0 failed |
| `harset/H53` (worktree `harset-H53`, branched off H51) | Agent `harset-H53` running at time of writing. **P0**, see below |
| Seed lanes `harset/jaffa`, `harset/opcore`, `harset/goauld` | Agents of the same names running at time of writing; each merges the wave-2 base, finishes the dead worker's output, applies the playtest authoring rules, commits, does not push |
| `harset/H06`, `harset/H08`, `harset/H09`, `harset/H50` | One WIP commit each (`wip(harset): session-1 worker output ...`), unbuilt, still based on the OLD wave-2 base. Not started this session |
| Test Postgres :5433 | Was down (crashed 2026-09-18 21:02 CDT, exception 0xC0000142, same as the day before). Restarted with `pg_ctl start -D server/pgdata -l server/logs/postgresql.log -o "-p 5433"`; all databases intact |

## What session 3 did

1. WIP-committed all seven dirty wave-2 worktrees before anything else. They had been uncommitted for a day.
2. Merged `origin/main` (#673-#680) into PR #662. Found a silent merge defect: `fire_stargate_dialed`, `fire_stargate_crossed` and `fire_player_flanked_npc` came from `main` without `populate_world_context`, so any `world`-gated chain on them failed closed. Fixed, guarded by `event_dispatch/world_context_contract_tests.rs`. **Check every new `fire_*` dispatcher after any future merge of `main`.**
3. Cross-checked the [2026-09-18 Castle playtest report](../../playtests/2026-09-18-colo-castle/README.md) against Harset. Results are packets H51 (shipped), H52 and H53 (opened) and nine authoring rules in the ledger's "Worker Input And Ownership" section.

## The playtest cross-check, finding by finding

| Castle finding | Harset verdict | Action |
|---|---|---|
| H1 `pack_angle` | Fixed globally by #677, now merged in | none |
| H4b attackers cannot turn | **Applies, worse:** 13 stationary Harset mobs return in `stationary_holds` before #677's re-face | H51 |
| H3 fails silently without a mesh | **Applies through a partial mesh:** 9 of those 13 stand in `harset.nav` holes, where the raycast said "blocked" to everything; they could never fire | H51 (tri-state line of sight) |
| (new, not in the Castle report) partial mesh as a containment gate | **P0:** `harset.nav` covers 57% of the gate plaza, has a 30-unit hole across the walk to the Command Center door, 17% around Petbe and `FirstBug`. Non-GM players are snapped back off-mesh, so an ordinary player cannot walk Harset. Every tester so far was a GM | H53 |
| H8 respawn kills region hints | **Applies, worse:** rings, Command Center doors and the gate volume are all region-driven | H51 (regions re-registered after reanchor; mechanism inferred, needs UAT, operator guide C6.3) |
| H9 edge fires before the step activates | Applies to every `enter_region` step (H21, H23) | H52 opened; seed lanes told to keep such steps reachable without the edge |
| H5 `aggression` absent, Idle NPCs never ticked | Hub must stay non-hostile (D-H03); applies to mission spawns | authoring rule: `spawn_entity` carries `aggression > 0` |
| H10 no `INT_DHD` interact arm | Already closed by H01 in #662 | none |
| DoT kills bypass the death path | Already closed by review fix R1 in #662 | none |
| Two Zuritskas both painted | Applies to hostile twins 221/222/223 and instance clones | authoring rule |
| Spawn headings all 0, Romney in a sealed wing | The 23 existing Harset rows carry real authored headings; new rows are M0 pins | operator guide C5.4a-c: real heading, reach every spot on foot, capture pins with `.bug pin <what>` |
| H6 leash, H7 escort robustness, H2 `0x18` grounding, castle.nav | Global, owned by the playtest follow-up and the Castle nav spike (peer session `cimmeria-79` was told about H53) | none here; H7 bites H37's 1372 later |

## Next actions, in order

1. Collect the four running agents (`git -C .claude/worktrees/<wt> log --oneline content/harset-wave2..HEAD`, `git status --short`). If the session that launched them died, their committed work is still on the branches; anything uncommitted is finishable by a fresh `rust-gameserver-dev` agent from the ledger packet text.
2. Integrate into `content/harset-wave2`, one at a time: H53 first, then `harset/goauld`, `harset/jaffa`, `harset/opcore` (`chain_replay_tests/mod.rs` `mod` lines conflict trivially, keep alphabetical; `harset-tags.md` is touched by more than one lane). After each: fmt, clippy, full `ci-live-db` suite on the worktree's own DB.
3. Batch 2, at most three agents: H06 (also closes Copilot C2 on #662), H08, H09. Each starts by merging the current wave-2 base. H06's WIP edits the OLD `cell/gate_travel.rs`, which #663 turned into `cell/gate_travel/`; port the hunks into `gate_travel/mod.rs` by hand.
4. Batch 3: H50 (repo-wide objective persistence, fixes #657) and H52.
5. Push `content/harset-wave2` and open PR "wave 2" against `content/harset-rebuild` if #662 is still open, or against `main` once it has merged. Tell `cimmeria-79` the H53 column and predicate names.
6. Still gated on the owner in-client (M0): H12, H14, H15, every spawn-position mission packet, respawner rows 20/22/23, gate-3 arrival, enabling chain 6007. H53 changes what M0 needs for arrivals: once world 57 is advisory, an off-mesh pin is acceptable as long as the owner walked to it.

## Decisions taken this session that the owner can veto

- **Off-mesh line of sight counts as clear** (H51). It affects every meshed world. The alternative makes any NPC or player standing in a mesh hole unable to attack or be attacked.
- **Harset runs with an advisory navmesh** (H53) until GH1 regenerates the mesh. Player containment in world 57 falls back to bounds, speed and teleport checks.
- **#662 was not merged.** It is ready.

## Update 2026-09-19 (later): all shelved work picked back up, on the owner's instruction

The owner lifted the three-agent cap for this purpose. `harset/goauld` and `harset/jaffa` are finished and merged into `content/harset-wave2` (`58659038`). Still running, each told to merge the wave-2 base first and to reassess its dead worker's draft rather than trust it:

| Agent / branch | What the reassessment added to the packet |
|---|---|
| `harset-opcore` | The edge-race class (a chain keyed on `player_loaded` and gated on a state that opens in the same world never fires; fix is a second `mission_completed` trigger) and vacuous `dialog_choice` / `item_use` guards, both found live by the other two lanes |
| `harset-H06` | `cell/gate_travel.rs` became a directory (#663), so the dial check is a hand port; one containment function, not two (`main` now has `playtest_friction::region_contains_xz`); a rejected hint must log and reach the player journal; tolerance for a player running through a thin door volume |
| `harset-H08` | Review fix R1 made DoT pulses lethal, so a DoT applied before the surrender now kills the surrendered NPC, which the draft did not cover; crossings are sampled at the health-application seam; the exit must clear path and velocity, face the player and log a `decision_outcome` |
| `harset-H09` | The draft accepted melee swings at 30 m. With thirteen stationary sentries that is the primary attack of a pinned Goa'uld, so the chooser now range-gates melee |
| `harset-H50` | Restore `hidden` / `optional`, not only ids (else mission 1200 completes early after a relog); invert the pinned 742 defect test; correct Castle 701-708 tests that encoded the old `MissionUpdate` shape; keep #680's step-activation seam; leave a callable mission-log resend seam for respawn |
| `harset-H53` | New this session, P0 (advisory navmesh) |

H52 is not dispatched: it edits the same step-activation seam in `progression.rs` as H50, so it goes after H50 merges. H05 (minigame competency) is unblocked since Castle #652 merged and has never been started.
