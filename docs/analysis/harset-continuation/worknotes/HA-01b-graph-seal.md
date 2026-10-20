# HA-01b graph seal for mission 1664 (PROVISIONAL)

> Type: design note. Audience: Harset coordinator, owner, the later HA-01b builder. Packet PU-05, mission-systems-advisor, 2026-10-19. Base: `040ef6394` (combined Castle + Harset branch). Docs only: no seed, code, ID allocation, lab or cargo. Nothing here has been seen in the client.

**Every design below is PROVISIONAL (O18-13).** It stays so until its evidence and in-game behaviour are checked. This note does not call anything "sealed" in the ledger sense: it freezes the *shape* of the graph so a builder has nothing left to invent, and lists exactly what the owner still decides. HA-D05 stays `Provisional`.

Labels: **SOURCE** (read in a file at this base), **FROZEN** (taken from `castle-continuation/evidence/1664.md`, not re-derived), **INFERENCE** (reasoned, with confidence), **PROVISIONAL** (a choice made here), **OWNER** (a decision this note cannot make).

## 1. Verdict

| Leg | State | Owner of the build |
|---|---|---|
| Castle grant (chain 1398) | FROZEN, copied verbatim in section 3. Not in any seed at this base (SOURCE: no `1398` chain row under `db/resources/Content`). | Castle CS-06a |
| Harset receiver: Kaerlia, 5224 to 5225 | Shape frozen below, PROVISIONAL | HA-01b build |
| Harset receiver: door, 5225 to 5226 | Shape frozen below, PROVISIONAL; cursor on a bind-only prop unproven | HA-01b build |
| Step 5226 debrief (6061 Marsh, 6062 Moh'katan) | Out of scope here; HA-00c owns the faction/objective graph | HA-01b build, after HA-00c |
| Abandon path without 1399 | Proposal in section 6; **OWNER decides (O18-02)** | n/a |
| ID collision check | Done in section 7: zero row collisions; one historical *file-level* reservation to note | n/a |

Deferred, not blocked: per the PU table the seal can run now and the build is deferred (D-PU1).

## 2. Mission facts the graph rests on (SOURCE)

- Mission 1664 `Debriefing`, label `General`: `can_abandon` true, `can_fail` false, `num_repeats` 1, `award_xp` true, `reward_naq` 0, `reward_xp` 0, not hidden, level 1 (`db/resources/Missions/Seed/missions.sql:781`).
- Steps: 5224 index 0 "Speak to Sgt. Kaerlia by the Stargate", 5225 index 1 "Make your way to the Command Center", 5226 index 2 "Report for debriefing" (`mission_steps.sql:2879-2883`). All `step_enabled` false in the seed, like every other mission, so the first step is activated by accept (`executor/mission.rs:39-60` takes `def.step_id` and builds the objective list).
- Objectives: 6063 (5224), 6059 optional + 6060 required (5225), 6061 and 6062 required (5226). Seen in HA-01b1 and the ledger; not re-read.
- `can_abandon` is a journal flag only. SOURCE: no read of `can_abandon` exists in any `crates/**/*.rs` outside tests and fixtures (`CS-04cS2.md` line 114 says the same of `cell-content` and `entity`). The server refuses an abandon only when the mission is not active (`cell/missions/lifecycle.rs:368-386`).
- Accept does four things in order: offer guard, `send_mission_update` (persist), `fire_mission_accepted`, `fire_step_activation_regions` (`executor/mission.rs:51-105`). The persist is best-effort (FROZEN, 1664.md section 4).

## 3. Castle grant: chain 1398 (FROZEN, copied for the builder)

Source of truth is `docs/analysis/castle-continuation/evidence/1664.md` section 2. It is reproduced so the Harset side can be read alone; if the two ever differ, the evidence file wins.

- Trigger `mission_completed`, key `'708'`, scope `player`, `once` false. Conditions: `world 8`; `mission_status 708 eq completed`; `mission_status 1664 eq not_active`. Action: `accept_mission 1664`, params `'{}'`, `delay_ms` 0.
- The chain runs inside chain 1360's batch, before `perform_gate_travel`, so the player is still in Castle (world 8) and 1664 is ACTIVE with step 5224 before the world change (FROZEN, 1664.md section 1).
- Never twice (FROZEN, 1664.md section 3): `mission_completed 708` cannot recur; the `not_active` predicate is the only guard for a COMPLETED 1664 inside `num_repeats` 1, because the offer guard does not refuse that case.
- Harset consequence (INFERENCE, medium-high): the player arrives in world 57 with 1664 active on 5224 and **a fresh cell entity with no dialog binds**. Binds are in memory (agent memory `castle-cellblock-chains.md`), so every Harset cue must be added by a `player_loaded 'Harset'` chain. That trigger is already used for restore chains in this world (`harset_goauld_chains.sql:658`, `harset_jaffa_chains.sql:786`).
- **Chain 1399 is not part of this seal** (O18-02, section 6). The 1399 rows in 1664.md stay un-seeded.
- Drift note: 1664.md cites `event_dispatch/mission.rs:159-168` for the abandon event. At this base the function is `fire_mission_abandoned` at `event_dispatch/mission.rs:178`. Behaviour is unchanged; cite the new line.

## 4. Harset graph (PROVISIONAL)

All chains are world 57. Candidate chain band is 6008-6020 (13 ids; section 7). Nine are used; 6017-6020 stay spare. Ids are *candidates*, not allocations (HA-D09 keeps them inactive until the coordinator activates them).

Actors, from HA-01b2 and HA-01b1 (not re-derived):

| Actor | Shape | Ids needed |
|---|---|---|
| Sgt. Kaerlia | Template `Harset_Kaerlia`, `name_id` 27315, talk bit 134217728, `speaker_id` 3219; spawn at (-5, -68.95, 30), world 57, tag `Harset_Kaerlia` | template 260, spawn 360 (candidates) |
| Kaerlia "!" cue | Existing dialog-set-map **7016** (set 1992, dialog 6169, flags 16777216), bound per player with `{"slot": 260}`. Nothing else uses 7016 (SOURCE: grep of `Content/Seed` finds 1992/7016/6169 only in the `harset_jaffa_chains.sql:531-593` comment block) | none |
| 5225 door | World object, **existing template 22** ("Door control panel", `interaction_type` 0), static spawn, tag proposed `Harset_CmdDoor`. Per-player cue: NULL-dialog bind carrying `INT_MissionWorldObject` (1073741824) | spawn (see section 7), one dialog-set + one dialog-set-map row |
| Feedback lines | Two to three barks or monologue dialogs (never `system_message`, a log-only stub) | authored dialog + screen ids, owned by HA-00c |

### Chains

Trigger keys are shared by siblings, and one event fires **every** matching chain, so siblings on a key are made pairwise disjoint (agent memory `multi-chain-dispatch-semantics.md`). "Active" means `mission_status 1664 eq active`.

| Cand. id | Trigger (key) | Conditions | Actions, in order |
|---|---|---|---|
| 6008 K1 talk, credit | `interact_tag` `Harset_Kaerlia` | `world 57`; `step_status 1664/5224 eq active` | 1 `remove_dialog_set 7016` slot 260. 2 `display_dialog 6169`. 3 `advance_step 1664` to 5225. 4 `add_dialog_set` door bind |
| 6009 K2 talk, already past 5224 | same | `world 57`; active; `step_status 1664/5224 neq active` | `npc_bark` "report to the Command Center" |
| 6010 K2 talk, no mission | same | `world 57`; `mission_status 1664 eq not_active` | `npc_bark` neutral line |
| 6011 K2 talk, finished | same | `world 57`; `mission_status 1664 eq completed` | `npc_bark` neutral line |
| 6012 D1 door, credit | `interact_tag` `Harset_CmdDoor` | `world 57`; `step_status 1664/5225 eq active` | 1 `remove_dialog_set` door bind. 2 `advance_step 1664` to 5226. 3 `cross_world_teleport Harset_CmdCenter` (0, 0.355, -20) |
| 6013 D2 door, anyone else | same | `world 57`; `step_status 1664/5225 neq active` | `cross_world_teleport Harset_CmdCenter` (0, 0.355, -20) |
| 6014 R1 restore Kaerlia cue | `player_loaded` `Harset` | `world 57`; `step_status 1664/5224 eq active` | `add_dialog_set 7016` slot 260 |
| 6015 R2 restore door cue | `player_loaded` `Harset` | `world 57`; `step_status 1664/5225 eq active` | `add_dialog_set` door bind |
| 6016 A1 abandon cleanup | `mission_abandoned` `1664` | none (any world) | 1 `remove_dialog_set 7016` slot 260. 2 `remove_dialog_set` door bind |

Why this shape (the traps it avoids):

- **No `complete_objective`.** `complete_objective 6060` auto-completes the mission once every required objective is done, which skips step 5226 (HA-01b1 hazard 1). `advance_step` force-credits 6059/6060 and does not run that check (agent memory `advance-step-vs-complete-objective.md`). The same holds for 6063.
- **Cleanup before the mission mutation, setup after.** K1 and D1 remove the old bind first, then advance. K1 adds the door bind last. If that last add is lost, R2 restores it on the next login (it is not a dead end, just a missing cursor until relog; INFERENCE, medium).
- **Failure between cleanup and advance (K1).** K1 runs `remove_dialog_set 7016` as action 1, before `display_dialog 6169` and `advance_step` (actions 2 and 3). If the cue is removed and the step is not advanced, the player sits on 5224 with no cue until relog (6014 restores it only on login). SOURCE: `crates/content-engine/src/chain/mod.rs:352-384` runs every action in order; an `ActionResult::Error` is logged (`warn!`, "Action failed") and the loop continues. So a failed action does **not** abort the rest of the chain, and a failed `display_dialog` still reaches `advance_step`. The stuck case needs the batch itself to stop (a panic, or the player leaving mid-batch). That is not tested: add a replay test that fails action 2 and asserts 1664 is on 5225 with the door bind present. Safer ordering, for the builder and owner to choose (not applied to the table above): `advance_step 1664` to 5225 first, then `display_dialog 6169`, then `remove_dialog_set 7016`. A failure then leaves the step advanced and at worst the Kaerlia cue on 5225, which a click answers with 6009's bark and which 6014 does not restore on relog. It gives up cleanup-before-mutation for this one edge.
- **Disjointness.** K1/K2 split on step 5224 and on mission status. D1/D2 split on step 5225. States tested by the builder: no 1664, on 5224, on 5225, on 5226, completed. Exactly one K chain and one D chain must fire per state. `neq` is a parsed operator; that it is true for a player with **no** 1664 row is SOURCE: `crates/cell-content/src/cell/content/mission_context.rs:98-108` documents that an absent step or mission reads `not_active` via the evaluator's `unwrap_or("not_active")` fallback, so `neq active` is true. The reason 6010/6011 use `eq not_active` / `eq completed` instead of `neq`. D2's `neq` has no such alternative and needs the replay test (section 8).
- **Duplicate click and relog.** After K1 the step is 5225, so a second click lands on 6009. After D1 the step is 5226 and the door bind is gone, so a second click is D2 (teleport only). Neither relies on `once`, which resets on relog (agent memory `content-engine-once-semantics.md`).
- **Teleport coordinates** are copied from chain 6006's destination; `cross_world_teleport` cannot set yaw (`harset_space_chains.sql:189-192`).
- **Range.** The 5-unit gate runs before every chain (`cell_methods/player/interaction/interact.rs`, `MAX_INTERACT_DISTANCE`); no chain condition is needed.
- **Binds are per player, in memory.** Do not use `set_interaction_type` on the shared door prop; the Castle header forbids clearing the last shared bit (`castle_706_708_chains.sql:135-189`). The Kaerlia *tag* still needs one paired `set_interaction_type` row for the `interact_tag_linter` (HA-01b2 section 2).
- **First-press feedback.** Every click path produces something visible: dialog, bark, or a world change. The door's out-of-range case already gets the engine's chat line.

### Door soft-lock and the open door (OWNER, from HA-01b1)

- Chain 6006 (walk-in teleport to the Command Center) stays enabled for everyone. A player on 5225 who walks into the box lands in world 68 with 5225 still active, because the credit lives only on the click. Two options: (a) add a credit-only `enter_region` chain on point set 2078 gated on `step_status 1664/5225 eq active`, no teleport (it fires alongside 6006; one spare id, 6017); (b) accept the risk. This note recommends (a), PROVISIONAL, but it adds a second credit path next to the right-click door, which O18-12 asked to preserve, so it is the owner's call.
- A 1664 player still on 5224 who clicks the door is not refused (D2 is open to all, for parity with 6006). Gating the click would be cosmetic while 6006 is open. Recommendation: leave open. OWNER.

## 5. Kaerlia (HA-01b2, restated for the seal)

Only the points the graph depends on; the design card remains the source.

- Identity: female, SGC uniform clone of template 370's components, faction 1, level 50, class `mob`, `event_set_id` 570, `has_dynamic_properties` true, vendor/trainer/loot/ability/weapon NULL. All PROVISIONAL (O18-01).
- Placement: primary (-5, -68.95, 30), fallback (8, -68.95, 36), `is_stationary` true; nav-probed on component 11, outside point set 1001. Heading unverified.
- She owns only the 5224 to 5225 edge. She never touches 5225 or 5226.
- Open client items (unchanged): does `name_id` 27315 render a nameplate; heading convention; is 6169 truly her dialog (INFERENCE, medium-high). UAT rows B7 and B9 in `uat.md` stay PROVISIONAL.
- **Seed hygiene for the builder:** the tag `Harset_Kaerlia` is absent from `spawnlist.sql` (SOURCE: grep, 0 hits), so no tag collision.

## 6. Abandon path without 1399 (PROPOSAL, OWNER decides: O18-02)

### What O18-02 fixes

SOURCE (owner register, O18-02): 1399 is omitted; **neither HA-01b nor CS-06 may ship 1399 or any other re-grant of 1664 after abandonment**; "what a player who abandons 1664 gets instead" is explicitly undecided. FROZEN (1664.md section 4): abandon removes the instance, so 1664 then reads `not_active` with 708 still completed, and no tracker state tells "abandoned" from "never accepted".

So with the baseline in section 4, a player who abandons 1664 has no way back to the debrief. 1398 cannot refire (`mission_completed 708` is a one-time edge), and K2 (6010) just tells them nothing is pending. Characters who finished 708 before 1398 shipped, or whose accept failed to persist, are in the same position. Whether that matters is gameplay-dependent: SOURCE shows no mission prerequisite or reward row referencing 1664 (`reward_naq`/`reward_xp` 0, no match under `db/resources/Missions` or `Content`), but whether later Harset content expects 1664 completed is **not checked in this note** (open item).

### Options

| Option | What it does | Cost | Respects O18-02 as written |
|---|---|---|---|
| P0 baseline | Abandon is final. 6016 only tidies binds. Players who lose it are re-granted by a GM only (verify a GM accept path exists before relying on this) | none | Yes |
| P1 seed flag | Set `can_abandon` false on mission 1664 so the client greys the Abandon button. Journal-only: the server does not enforce it, so a hand-built abandon still works | One-column edit to a stock mission row in `Missions/Seed/missions.sql` (stock data; needs the owner's OK). If `can_abandon` is also baked into client-side mission defs, the server value may not reach the journal (INFERENCE, low; unchecked) | Yes |
| P2 server refusal | New code: refuse `abandon_mission` for missions on an allow-list. Combine with P1 | A new executor/lifecycle rule, a test, and a rule for every other mission that the list would grow | Yes, but it is new engine behaviour |
| P3 player-initiated re-offer | Chain 6017: `interact_tag Harset_Kaerlia`, conditions `world 57`, `mission_status 708 eq completed`, `mission_status 1664 eq not_active`, actions `accept_mission 1664`, `display_dialog 6169`, `advance_step 1664` to 5225 (and the door-bind add). Fed by moving 6010 ("no mission") to also require `mission_status 708 neq completed` so 6010 and 6017 stay disjoint | One more chain and test pair. Anti-repeat: it needs a click on Kaerlia in Harset, never fires on login, never fires when 1664 is ACTIVE or COMPLETED, and the offer guard refuses a COMPLETED repeat past `num_repeats` | **No.** It is a re-grant after abandon. It ships only if the owner reverses that clause of O18-02 |

### Recommendation (PROVISIONAL)

1. Ship **P0** at first seal, as O18-02 requires. It needs no work beyond chain 6016 and the 6010/6011 barks.
2. Raise **P1** with the owner as the cheapest guard: it removes the self-inflicted case at the source and needs no engine change. Treat P2 as not worth building unless playtests show hand-built abandons.
3. Hold **P3** as the justification O18-02 asked for if the owner wants a recovery route: it is player-driven (no login loop, which was 1399's flaw), re-offers only inside the Harset hub at the NPC who owns the debrief, covers the same pre-1398 and persist-failure cases 1399 covered, and cannot loop. Its remaining cost is that a player can abandon and re-accept repeatedly; with `reward_xp`/`reward_naq` at 0 there is nothing to farm, but check `award_xp` true against any per-step reward before saying so (open item).

### OWNER decisions requested

| Id | Question | Recommendation |
|---|---|---|
| O18-02a | Ship the first seal with P0 (abandon is final)? | Yes |
| O18-02b | Set `can_abandon` false on 1664 (P1)? | Yes, if the client honours it |
| O18-02c | Reverse the "no re-grant after abandon" clause to allow P3? | Not for the first seal; revisit after UAT B7/B9 |

## 7. ID collision check (HA-D09)

Run at base `040ef6394` in worktree `pu-05`, PowerShell and Grep, seed style `INSERT INTO <t> (cols) VALUES (id, ...)` on one line. This is a point-in-time check of committed seed text; it asserts nothing about the live DB, uncommitted sibling work, or branches created after the scan.

| Candidate | Check | Result |
|---|---|---|
| Chains 6008-6020 | Any `content_chains/triggers/conditions/actions` row with these ids under `db/resources`; any hard-coded use in `crates/**/*.rs` | **0 rows, 0 code hits.** Harset seed holds no chain between 6007 and 6101 |
| Chains 1398, 1399 | Same, plus Castle neighbours | **0 rows.** Taken neighbours: 1394-1397 (`castle_567_chains.sql`) and 1400. 1398/1399 are free |
| Template 260 (`Harset_Kaerlia`) | `entity_templates` ids | **Free.** No id in 255-275. (Template **360** exists: the Debug Hub Goauld pet trainer. A different table and namespace from spawn 360, but easy to confuse in review) |
| Spawn 360 | every `spawnlist*.sql` (base, Dakara, Tollana, Debug Area) | **Free.** Nothing in 360-389. (Spawn **260** exists, `Castle_Pop_CommsDoor_1`; also a different value, noted because 260 is the template candidate) |
| Tag `Harset_Kaerlia`, `name_id` 27315 | spawnlist tags, `texts.sql` | Tag free. 27315 present once, as the intended text |
| Dialog-set-map 7016, set 1992, dialog 6169 | seeds | Present, used only by the Jaffa-chain comment block; no chain binds them yet |
| Other worktrees | `git grep` of the same chain, spawn and template patterns over **173 distinct worktree HEADs** (point-in-time count at the scan; 187 worktrees exist as of 2026-10-19, not rescanned) | **0 hits** |

### Findings the check surfaced

1. **Historical file-level reservation, not a row collision.** `harset_space_chains.sql` (lines 17-21, 288, 301) says the file "owns 6001-6099" and parks **6008-6020 as "relog restore, RESERVED, deliberately unused"** for "a later packet in this file's range". HA-01b's band is the same 13 ids, and 6014/6015 are exactly relog-restore chains, so the claim is *consistent*. Two consequences: the builder either appends to that file or opens a new `harset_*` seed and must then amend that comment so the file no longer claims the block; and `id-allocation.md` should cite this comment. No change made here.
2. **The graph needs ids the allocation table does not hold.** HA-01b's candidates cover 6008-6020, spawn 360 and template 260 only. This design also needs: (a) a spawn for the door prop. Spawns 360-379 are already assigned (361-364 are HA-01a optional doors, 365-379 other packets); **380 is free** (nothing in 380-389) and is proposed, PROVISIONAL, for the coordinator to claim; (b) a dialog-set and a dialog-set-map row for the NULL-dialog door bind; (c) authored dialog and screen rows for 2-3 barks. (b) and (c) fall under HA-00c's missing authored-id audit (new ids in this repo sit at 100001+ and 1000018+ in `dialog_sets`/`dialog_set_maps`; the stock-pak check is HA-00c's). Not allocated here.
3. **Door tag.** `Harset_CmdDoor` is a proposal; no tag in the seed matches `Harset_.*Door`.
4. **Static check only.** Per HA-D09, individual ids stay inactive until the coordinator signs the activation; the seed-uniqueness test (`seed_id_uniqueness.rs`) will catch a real collision when the rows are added.

## 8. Tests the build owes (each must fail on the named revert)

Chain replay (`chain_replay_tests/mission_1664.rs`, shared with CS-06a for the Castle rows) and live-DB:

| Test | Revert that must fail it |
|---|---|
| Castle 708 completion accepts 1664 with active step 5224; not outside world 8; not when 1664 ACTIVE or COMPLETED | Remove 1398 or its `not_active` predicate |
| K1 on 5224 advances to 5225, binds the door cue, leaves 1664 ACTIVE | Replace `advance_step` with `complete_objective 6063`; drop the step gate |
| Exactly one of 6008-6011 fires in each of five states (no 1664, 5224, 5225, 5226, completed) | Remove the 5224 gate from K1, or give 6009 and 6008 the same conditions |
| D1 on 5225 advances to 5226 and does **not** complete 1664 | Swap `advance_step` for `complete_objective 6060` |
| D2 with `neq` evaluates true for a player with no 1664 row (SOURCE, `mission_context.rs:98-108`), and D1/D2 are disjoint in all five states | Drop D2's `neq` gate |
| Duplicate door click after a refused teleport credits once and does not stick | Remove the 5225 gate from D1 |
| `player_loaded 'Harset'` re-adds the Kaerlia cue on 5224 and the door cue on 5225, nothing otherwise | Delete 6014/6015 or their step conditions |
| Abandon (6016) removes both binds; the abandon event sees the mission as already removed | Delete 6016; reorder so it reads the mission row |
| 6006 still teleports with no mission effect | Add a mission action to 6006 |
| Spawn 360 on the world-57 mesh, outside point set 1001, at least 5 units from respawner 20; template has NULL vendor/trainer/loot/ability/weapon and faction not 0 | Move the spawn; add a vendor id |
| `name_id` 27315 present in `texts` | Change the id |
| The `interact_tag_linter` accepts `Harset_Kaerlia` | Drop the paired `set_interaction_type` |
| No chain in the seed contains id 1399 or any other 1664 re-grant action (P0 only) | Add a 1664 `accept_mission` anywhere but 1398 |

Telemetry owed: one structured row per Kaerlia/door click with chain id, player, 1664 step and outcome (`advanced`, `bark`, `parity_teleport`); one per bind add/restore/remove with its reason; the existing `interaction.out_of_range` row; and a friction counter that must stay zero for `Harset_Kaerlia` and `Harset_CmdDoor`.

## 9. Ownership: what the owner and others must decide

| Id | Decision | Whose | Needed by |
|---|---|---|---|
| O18-02a/b/c | Abandon path: P0 now, P1 flag, whether to allow P3 (section 6) | Owner | HA-01b build |
| new | Door soft-lock guard on region 2078 (section 4)? Recommended yes | Owner | HA-01b build |
| new | Leave the door click open to a player still on 5224? Recommended yes | Owner | HA-01b build |
| new | Claim spawn 380 for the door prop; activate 6008-6020 and note the `harset_space_chains.sql` comment | Coordinator (HA-D09, O14-04) | HA-01b build |
| new | Authored dialog, screen and dialog-set-map ids for barks and the door bind | HA-00c | HA-01b build |
| unchanged | 5225 cursor from a bind alone, nameplate 27315, heading, mesh for the door (UAT B7, B9) | Client look, ask the user before any lab use | After the build, PROVISIONAL until then |
| unchanged | If the cursor never appears: new template `interaction_type`, or a client change | Maintainer | Only if B7 fails |
| open | Does later Harset content require 1664 completed? Not checked | Coordinator | Before choosing P0 vs P3 |
