# Castle Cellblock Rebuild Work Packets

> Type: how-to. Audience: Claude Code coordinator and Sonnet packet workers.
> Updated: 2026-09-17. Companions: [launch prompt and decisions](README.md), [spec audit](audit.md), [testing playbook](../../../TESTING.md), [parity ledger protocol](../legacy-command-parity/work-packets.md#dispatch-rules).

## Dispatch Rules

This ledger reuses the dispatch, ownership, worknote/handoff and acceptance rules of the [legacy command parity ledger](../legacy-command-parity/work-packets.md#dispatch-rules) verbatim; they are not repeated here. Initial state: documentation prepared against `01ab54b0`; no implementation, build, runtime test or client UAT has run. Implementation-session authorization is required before any packet is dispatched.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDesign**, **BlockedDecision** (needs a D-CB answer from [README.md](README.md#open-decisions)), then **Writing**, **Review**, **Integrated**, **UATPending**, **Done**, **BlockedEvidence**. None is Done initially.

Default writer is `rust-gameserver-dev` on Sonnet. Chain-only packets (SQL seed plus chain-replay test) are the smallest unit; a packet that needs a new executor arm says so explicitly and ships the arm, the seed row and the replay guard together, matching the discipline in [content-engine.md](../../content/content-engine.md#13-recent-additions-last-7-days). `testing-validation-engineer` reviews every regression strategy; `documentation-writer` reviews the [mission-chains.md](../../content/mission-chains.md) updates.

## Worker Input And Ownership

Common read-only entries for every packet: [castle_cellblock_chains.sql](../../../db/resources/Content/Seed/castle_cellblock_chains.sql), [content-engine.md](../../content/content-engine.md) sections 3 and 9, [content-chains.instructions.md](../../../.github/instructions/content-chains.instructions.md), [interaction-flags.md](../../content/interaction-flags.md), and the packet's Python reference file under `deprecated/python/cell/`. The neighboring test fixture is the matching `crates/services/src/cell/content/chain_replay_tests/mission_<id>.rs`.

Chain ID allocation (append to the header of the seed file when used): `1112-1120` C03, `1121-1130` C04, `1131-1140` C05, `1141-1150` C06, `1151-1160` C07, `1161-1170` C08, `1171-1190` GC1 children, `1191-1199` GC2. Castle-side chains use a new file `castle_chains.sql` with range `1201-1250`. Never reuse a purged 5xxx id.

Seed rules: edit `db/resources/` seeds directly (no `db/scripts/` migrations), keep `Castle_Cellblock.*` region keys byte-identical to `point_sets.sql`, and run `crates/content-engine/tests/interact_tag_linter.rs` after every seed change. Live-DB tests use `require_db_or_skip!`, serialized.

## Common Acceptance

Every chain packet ships a chain-replay test that (a) asserts the exact resolved action list for the happy path, (b) asserts the chain does **not** resolve for the adjacent wrong state (wrong archetype, wrong step, already completed), and (c) asserts the relog-restore chain re-paints any interaction bit the packet sets. A test that passes with the new seed rows deleted is not a guard. New executor arms need a unit test on the executor path (does the side effect reach `space_mgr` or the `CellToBaseMsg` outbox) in addition to the replay guard. Client-visible changes (dialogs, sequences, despawns) are UAT gated per the README milestones.

## Foundation

### C00

**Status:** BlockedDependency (C03, for the shared `Action::LaunchAbility` arm). Decision D-CB14 answered 2026-09-17. **Scope title:** Prison Boot movement-lock gate at zone entry (new finding, `SGW_Castle_CellBlock_Dev_Master_v3.xlsx` sheet `03_Minigames_Gates` row "Order 0"). **Depends:** C03 (shares its new `Action::LaunchAbility` executor arm — land paired or immediately after, not duplicated). Should land before or alongside C01/C04 UAT since it gates everything else in the zone, including the already-implemented Frost/guard loot. **Advisor:** minigame-systems-advisor (Livewire wiring), combat-systems-advisor (movement-lock effect semantics), mission-systems-advisor (spawn-time trigger).
**Entries:** item 3438 (`db/resources/Items/Seed/items.sql`), abilities 1597/1598 (`db/resources/Abilities/Seed/abilities.sql`), effects 1939/1942/3081 (`db/resources/Effects/Seed/effects.sql`), `char_creation_choices.sql`/`char_creation_visgroups.sql` (forced-equip evidence), [executor/mod.rs](../../../crates/services/src/cell/content/executor/mod.rs) (needs the same `Action::LaunchAbility` arm C03 adds — reuse, don't duplicate), chains 1001/1002 (`player_loaded Castle_CellBlock`, the precedent trigger), `crates/services/src/minigame/games/livewire.rs` (the minigame D-CB14 picked), [audit.md](audit.md#new-finding-the-prison-boot-movement-lock-gate-v3-sheet-03_minigames_gates-row-order-0) for the full evidence chain.

**D-CB14 (answered 2026-09-17): reuse Livewire.** Already fully wired end-to-end in this repo and used twice more in this same zone (the cell-door hack, the Preparation stasis terminal) — no new minigame infrastructure, matching v3's own recommendation against inventing new infra.

**New finding (2026-09-17, user-supplied reference screenshot): item 3438's `visual_component` is a seed data bug.** Item 3438 currently points at `AR_H_Ballistic00.AR_HM_BB1_BH100` — a generic mesh shared byte-for-byte with 7 other items all literally named "Titanium Ballistic Boots" (ids 2951-2954, 2959-2961). The user supplied a reference image of the intended model: a distinctive bulky, mechanical, gold/orange restraint boot with a status light, not generic armor. A repo search found the correct asset already seeded under a different item: **item 5865, also named "Prison Boots,"** points at `AR_Global.Prisoner_Boots` — a dedicated `AR_Global.Prisoner_*` package family that also has matching pieces: item 3440 "Prison Jacket" → `AR_Global.Prisoner_Torso`, item 3437 "BDU Pants" → `AR_Global.Prisoner_Legs`. `char_creation_choices.sql` already forces 3440+3437 onto the same starting vis-groups 3438 uses (paired torso/legs rows adjacent to 3438's own rows), confirming this is one matching prisoner cosmetic set with 3438 as the odd one out. C00's scope now includes re-pointing the forced-equip choice rows from item 3438 to item 5865 (or retargeting 3438's own `visual_component`) so the movement-lock mechanic is attached to the correct model — a one-line data fix, not new authoring. Item 2949 "Armored Prison Boots" is a red herring (`AR_H_Ballistic00.AR_HM_BB1_BH101`, a different generic ballistic variant, not the `AR_Global` family). Not yet cross-checked against the actual client mesh files (`../sgw/` per the user) to visually confirm `AR_Global.Prisoner_Boots` renders as the screenshot — the seed-data match is strong enough to proceed, but the C00 worker should do that visual check as part of its own verification.

**Scope:** (1) launch ability 1597 on `player_loaded Castle_CellBlock` (mirrors C03's Stasis Sickness load-gate pattern — land after C03 or pair the two `LaunchAbility` chains together to avoid two separate reviews of the same new executor arm); (2) effect 1939 currently has no `script_name` — decide whether movement-lock needs a real effect-script (blocking `client_move.rs`) or whether the client already self-locks on receiving the effect and the server only needs to track the state flag for anti-cheat; (3) wire a Livewire session (same pattern as the existing cell-door-hack/stasis-terminal Livewire triggers in this zone) whose success callback fires ability 1598, which both swaps the item (effect 3081) and clears the lock (effect 1942); (4) relog mid-lock must re-apply the lock, not leave the player stuck unable to receive it again or permanently freed by a relog exploit.
**Acceptance:** a player who has not yet cleared the gate cannot move (or the anti-cheat state flag is set, per whichever design D-CB14 picks) from the moment they load into Castle_CellBlock; clearing the minigame removes the lock exactly once and swaps the item; relog before clearing re-applies the lock; relog after clearing does not re-apply it. Live-DB or unit test on the executor path per the packet's own new work (movement-lock enforcement, minigame callback). UAT: this becomes the very first thing an M1 tester experiences, so fold it into the existing M1 milestone's relog checks rather than opening a new milestone. **Exclude:** designing a brand-new minigame type from scratch if an existing SGW minigame (Hack/Bypass/Livewire etc., `crates/services/src/minigame/`) can be reused — v3 explicitly recommends against inventing new minigame infrastructure here.

### C01

**Status:** Ready. **Scope title:** Purge the auto-exported space seed and repair the Region8 guard aggro. **Depends:** implementation-session authorization. **Decision:** D-CB02. **Advisor:** mission-systems-advisor; testing-validation-engineer for the guard.
**Entries:** [space_castle_cellblock_chains.sql](../../../db/resources/Content/Seed/space_castle_cellblock_chains.sql), [database.sql](../../../db/database.sql) line 343, [mission_638.rs](../../../crates/services/src/cell/content/chain_replay_tests/mission_638.rs), [mission_641.rs](../../../crates/services/src/cell/content/chain_replay_tests/mission_641.rs), [Castle_CellBlock.py](../../../deprecated/python/cell/spaces/Castle_CellBlock.py) `n116`/`n117`/`n120`, [point_sets.sql](../../../db/resources/Events/Seed/point_sets.sql).
**Scope:** delete chains 5000-5029 (audit defects B1/B2) except the one unique behavior, the Region8 guard aggro, which moves into the curated file as a new chain with key `Castle_CellBlock.Region8` (capital B, matching the point set), `once = true`, actions `set_aggression level 1` then `generate_threat 1000` on `ArmYourself_NIDGuard` (the Python values; the export had 5000 and no aggression). Drop the `system_message` rows (log-only today, issue #268). Remove the `\ir` line or leave the file as an empty header per coordinator preference. Do not touch curated chains.
**Acceptance:** replay tests fail before the purge and pass after: Jaffa Region2 entry resolves exactly one `AddDialogSet`/`AddDialog` (5866) and exactly one `AcceptMission 638`; Human resolves only 2794; Jaffa Marsh interact before 641 resolves exactly one `DisplayDialog 5022` and zero 4001; Region3 exit / Region4 / Region5 / Region6 each resolve exactly one accept. Region8 entry resolves the aggro chain with the exact key. Seed linter clean. **Exclude:** any curated-chain behavior change; fixing `system_message` wire format.

### C02

**Status:** Ready. **Scope title:** Chain-replay baseline for missions 640, 680 and 681-686. **Depends:** C01 (so the baseline is not asserting duplicates). **Advisor:** testing-validation-engineer.
**Entries:** [chain_replay_tests/mod.rs](../../../crates/services/src/cell/content/chain_replay_tests/mod.rs), [mission_687.rs](../../../crates/services/src/cell/content/chain_replay_tests/mission_687.rs) as the counter-pattern fixture, seed chains 1041-1046, 1071-1074, 1081-1094.
**Scope:** add `mission_640.rs`, `mission_680.rs`, `mission_681_686.rs` pinning: Livewire victory to 2215 with icon swap, post-hack switch triggers transporter region 1, teleport-in 2 completes 640 only while active; ring switch 2 triggers transporter 2, teleport-in 3 advances 2345, Region9 completes 680 and accepts 681 once; each controller's increment/completion pair including the `counter gte target-1` pre-increment semantics and the priority ordering. Covers spec tests T10, T14, T15.
**Acceptance:** every listed chain has a positive and a negative assertion; the ordering invariant test reproduces the `a51a10d` bug shape (equal priority) as a failing control. **Exclude:** behavior changes; executor tests.

### C03

**Status:** Ready. **Scope title:** Stasis Sickness applied on zone load, cure path verified. **Depends:** C01. **Decision:** D-CB04. **Advisor:** combat-systems-advisor; mission-systems-advisor.
**Entries:** [executor/mod.rs](../../../crates/services/src/cell/content/executor/mod.rs) (add `Action::LaunchAbility` arm), [loader/action.rs](../../../crates/content-engine/src/loader/action.rs) (`launch_ability` already loads), [use_ability/handle.rs](../../../crates/services/src/cell/abilities/use_ability/handle.rs) `handle_use_ability(entity_id, ability_id, target_id, ...)`, abilities 1372/1373/1374 and effects 1634/1636 in `db/resources/Abilities/Seed/abilities.sql` and `db/resources/Effects/Seed/effects.sql`, [items_event_sets.sql](../../../db/resources/Items/Seed/items_event_sets.sql) row `(2, 19, 1374, 5)`, [proposed-extensions.md](../../content/proposed-extensions.md) section 1.4.
**Scope:** (1) executor arm for `LaunchAbility { ability_id, target }` routing to the existing self-target ability path, with the guard failures (`handle_use_ability` returns `false`) logged at warn per the negative-logging convention; (2) chain on `player_loaded Castle_CellBlock` gated `mission_status 639 neq completed` that launches 1372 on the player (the Python launched it unconditionally on every load; the gate is the idempotence the spec asks for, since re-applying an active effect must be a no-op, verify in `cell/effects/`); (3) verify what effect 1634 (flags 524288, no `script_name`) actually does when applied: if it is icon-only, record that and stop; do not invent a DoT. (4) Verify the cure: does the client's `items_event_sets` binding cause a `useAbility 1374` call on vial use, and does 1636 strip 1634? If not, add `launch_ability 1374` to chain 1034 before `remove_item`.
**Acceptance:** executor unit test proves `LaunchAbility` reaches `handle_use_ability`; replay test proves the load chain resolves only while 639 is not completed; live-DB or unit test proves 1634 is present after load and absent after the cure; relog after cure does not re-apply. Covers T09. **Exclude:** Stage 2 escalation (ability 1373 needs the timer primitive, engine Tier 1.2); Symbiote Loss (D-CB12).

### C04

**Status:** BlockedDecision (D-CB03). **Scope title:** Frost's Letter (mission 1360) accepted when the letter is granted. **Depends:** C01. **Advisor:** mission-systems-advisor.
**Entries:** chain 1003 in the curated seed, [mission_622.rs](../../../crates/services/src/cell/content/chain_replay_tests/mission_622.rs), missions/steps 1360/4037/4038 in `db/resources/Missions/Seed/`.
**Scope:** on `dialog_open 3995` while step 2113 is active (extend chain 1003 or add a sibling at priority 0) `accept_mission 1360` when `mission_status 1360 eq not_active`. Step 4037 stays active for the rest of the zone; step 4038 (give the letter to Col. Marsh) belongs to the Castle side and is out of this packet. The mission must survive the cross-world hop (mission state is persisted through `MissionUpdate`; verify nothing on world exit abandons active missions).
**Acceptance:** replay test resolves exactly one accept; a second Frost interaction does not re-accept; T03 assertion that 1360 is active after loot; a live-DB test that the row persists across the 688 transition path. **Exclude:** completing or advancing 1360.

## Tutorial Objectives

### C05

**Status:** BlockedDependency (C02) and BlockedEvidence (cover set id). **Scope title:** Take-cover objective 2484 and indicator hide. **Decision:** D-CB05. **Advisor:** npc-ai-spawn-advisor (cover system owner), mission-systems-advisor.
**Entries:** chains 1032/1033/1035, [event_dispatch/cover.rs](../../../crates/services/src/cell/content/event_dispatch/cover.rs), [cover/detection.rs](../../../crates/services/src/cell/cover/detection.rs), [cover_sets.sql](../../../db/resources/AI/Seed/cover_sets.sql), [cover_demo.rs](../../../crates/services/src/cell/content/chain_replay_tests/cover_demo.rs), objectives 2482/2484 in `mission_objectives.sql`, sequences 10001/10014 (TakeCoverIndicator show/hide, sequence ids from the spec Kismet sheet).
**Scope:** first, an evidence step: identify the cover set that covers the med-station desk near (-234, 66.5, -124.7) from `cover_sets`/`cover_nodes` (candidate `_CA-CellBlock_Int00-15-15`, id 425; unverified). Then: replace demo chain 1035 with a chain on `player_entered_cover <set>` (or `player_in_cover_duration 2:<set>`) gated `step_status 639 2144 active` that runs `complete_objective 639/2484` and `play_sequence 10014`; change chain 1033 (drone death) to `complete_objective 639/2482`; add a completion chain gated on both `objective_status` keys that advances to 2343. Relog restore: if the player relogs mid-2144 with 2484 done, do not re-show the indicator.
**Acceptance:** replay tests for the three-way split (cover only, kill only, both) prove step 2144 advances only when both objectives are complete; a test proves the indicator-hide sequence resolves once. Covers T08. **Exclude:** NPC cover AI changes; new cover sets.

### C06

**Status:** BlockedDecision (D-CB05, gating) and BlockedDependency (C05 pattern). **Scope title:** Flanking objectives 2725 (Mess Hall long table) and 2731 (Hallway05). **Advisor:** npc-ai-spawn-advisor, combat-systems-advisor.
**Entries:** chains 1085-1087 and 1092-1094, [event_dispatch/cover.rs](../../../crates/services/src/cell/content/event_dispatch/cover.rs) (`OnNpcFlanked`, `OnPlayerEnteredCover`), objectives 2724/2725 and 2730/2731.
**Scope:** track the flank objective from `npc_flanked` (guard template 24 flanked while the mission is active) or from `player_entered_cover` on the long-table set, and `complete_objective` it. Per D-CB05's default, the kill counter still completes the mission; the flank objective is tracked but does not gate. If the decision flips to gating, the completion chain gains an `objective_status` condition and needs a soft-lock analysis (guards killed from range before any flank).
**Acceptance:** replay tests for flank-then-kill and kill-without-flank; the mission outcome matches the decision; controller stays hidden. Covers T14 flank clause. **Exclude:** cover-AI tuning, table cover-set authoring.

## Narrative Beats (spec-only content)

### C07

**Status:** BlockedDecision (D-CB08). **Scope title:** Mission prompt blurbs 2305, 4000, 2308, 2518 on accept. **Depends:** C01. **Advisor:** mission-systems-advisor.
**Entries:** chains 1034 (640 accept), 1053/1054 (641), 1061 (680), 1105 (688), `mission_accepted` trigger ([loader/trigger.rs](../../../crates/content-engine/src/loader/trigger.rs)), chain 1097 as the trigger fixture, `DUIST_DefaultBlurb` rows in `db/resources/Dialogs/Seed/`.
**Scope:** four chains on `mission_accepted <id>` each `display_dialog` the blurb. Blurb 2298 already displays via chains 1018/1019 and is the precedent. Precondition: one UAT check that the client does not already surface an accept prompt for these missions (double display would be worse than none).
**Acceptance:** replay tests, one per mission, exactly one display; UAT shows one prompt per accept. **Exclude:** dialog set bindings; 2308's "follow Marsh" text only makes sense if GC1 lands, so 2308 is conditional on GC1 approval.

### C08a

**Status:** Ready. **Scope title:** Honor `content_actions.delay_ms` in the executor. **Depends:** none. **Advisor:** testing-validation-engineer.
**Entries:** [executor/mod.rs](../../../crates/services/src/cell/content/executor/mod.rs) `execute_actions`, [loader/action.rs](../../../crates/content-engine/src/loader/action.rs), [engine_loader.rs](../../../crates/services/src/cell/content/engine_loader.rs) row struct, [chain.rs](../../../crates/content-engine/src/chain.rs) `ResolvedActions`.
**Scope:** carry `delay_ms` from the DB row through the resolved action and schedule delayed actions without blocking the cell message loop (a spawned task that re-enters the executor for the remaining tail, or a per-entity deferred-action queue drained by the existing tick; pick after one local trace of how `start_minigame`'s `on_victory_chains` callback re-enters). Deferred actions must be dropped if the entity leaves the space or disconnects before they fire.
**Acceptance:** unit test with a fake clock proves ordering (immediate actions run now, delayed ones after N ms, in sort order within the same delay); disconnect before expiry executes nothing; zero-delay behavior byte-identical to today. **Exclude:** the general `StartTimer`/`OnTimer` primitive (engine Tier 1.2); this is one field on one action list.

### C08b

**Status:** BlockedDependency (C08a) and BlockedDecision (D-CB07). **Scope title:** Straegis attack scene. **Advisor:** aoi-witness-broadcast (Marsh removal fan-out), mission-systems-advisor.
**Entries:** chain 1094 (686 complete) and 1084 (Region6 accept 687), sequence 1751 (`Castle_Cellblock-fffffffe.Main_Sequence.StraegisAttack`, EventSet 747 event 6000, 10.0096 s camera-only Matinee), [executor/world/mod.rs](../../../crates/services/src/cell/content/executor/world/mod.rs) `destroy_entity`, [cinematic-system.md](../../gameplay/cinematic-system.md), dialog 2516.
**Scope:** on `mission_completed 686`: `play_sequence 1751` (confirm the `viewType`/camera semantics the `onSequence` emitter needs for a Director-track Matinee; the existing emitters hardcode the arg block), `destroy_entity Preparation_ColMarsh` (Marsh is at his Preparation position unless GC1 moves him; despawning him there is invisible but still correct for the aftermath), `display_dialog 2516` with `delay_ms 10100`. Relog restore: if 686 is completed and 687 not accepted, Marsh must not respawn (the spawner respawns from `spawnlist`; verify and gate). Skip blood decal, rift creature and data disc (no asset or item evidence).
**Acceptance:** replay test asserts the three actions with their delays; executor test proves the despawn fans `LeftAoI` to witnesses; UAT T16/T17. **Exclude:** creature spawn, blood VFX, camera authoring.

### GC1

**Status:** Children scoped 2026-09-17 (`npc-ai-spawn-advisor` feasibility pass); ready to dispatch once C01 and C08a are integrated. **Scope title:** Escape escort and lockdown (spec rows 13-14). **Depends:** C01, C08a. **Advisors:** npc-ai-spawn-advisor, movement-teleport-advisor, mission-systems-advisor. **Decision:** D-CB13 (answered: full escort, not dialogs-only).
**Entries:** chains 1061/1071-1074, `set_follow_target` and `set_npc_poi` executor arms, [npc_ai/follow.rs](../../../crates/services/src/cell/service/npc_ai/follow.rs), [construction.rs](../../../crates/entity/src/cell_entity/construction.rs) (hardcoded `move_speed`), [executor/world/mod.rs](../../../crates/services/src/cell/content/executor/world/mod.rs) `move_waypoint`, dialogs 2308/2309/5019/4003, [proposed-extensions.md](../../content/proposed-extensions.md) section 3.3.

**Feasibility findings (evidence pass, 2026-09-17):**

- `set_follow_target` is fully wired (loader → executor → AI tick) but target resolution goes through `find_entity_by_tag`, and player entities have no `tag` — today it can only make one NPC follow another NPC. Needs a `use_player` param mirroring `move_entity`'s existing convention (~15 line change).
- `npc_ai_follow` paths correctly per-space via the navmesh, but `move_speed` is hardcoded at `0.6` units/tick (6.0 u/s) in `construction.rs`, not a DB column — 26% slower than World 12 player run speed (8.125 u/s), so Marsh would never close to the follow band and would trail further every hallway. Fix needs either a `move_speed` column on `entity_templates` or a catch-up multiplier.
- `npc_ai_leash` snaps a fighting NPC back to `spawn_position` on Idle while leaving `follow_target_id` set — a latent one-way trap if Marsh is ever aggroed mid-escort (mitigated today since hostile auto-aggro only targets players, but worth hardening: skip the spawn-snap when `follow_target_id.is_some()`).
- Ring transport is genuinely player-only end to end (`RingTransporter.players`, `FireTeleportIn` hard-rejects entities with no `player_id`) and should **not** be taught to move NPCs. Reuse instead: `move_waypoint` (`executor/world/mod.rs:330`) is an existing, unused (0 seed rows), unwired-by-name-only "instant snap + AoI resync" primitive — exactly what's needed to relocate Marsh on the same `teleport_in` trigger chain 1044 already uses.
- Navmesh flood-fill of `data/spaces/castle_cellblock.nav` (2778 verts / 1479 polys) confirms the Preparation room and the topside route (Ring 3 → Mess Hall → Hallway01-05 → Barracks) are **different, disconnected navmesh components** — component 24 vs component 8. That's *why* the ring exists; it is not a gap to fix. The entire topside route from Ring 3 onward is one connected component, so once Marsh is relocated there, follow-pathing needs no new navmesh work.
- No escort/companion NPC exists anywhere in the codebase today; `move_waypoint` has zero precedent uses. Template 10 (`Preparation_ColMarsh`) is literally named `'Col Marsh (pet)'` in `entity_templates.sql` — corroboration the 2009 devs modeled him as a companion and never shipped it.

**Approve (still open, low stakes, worker may decide with a documented rationale):** which of 2309 (speaker 261 "Col. Marsh") and 5019 (narration, Straegis warning) plays and for whom (5019's class gating is unknown, spec Legacy_Unresolved row); whether 4003 plays on teleport-in 3 without any barrier VFX.

**Child packets:**

- **GC1a — dialogs only.** Seed only, no dependencies. Ready now.
- **GC1b-0 — engine: let an NPC follow a player.** Rust only, no seed, no dependencies. Ready now (may run alongside C01/C08a). Adds `use_player` to `set_follow_target`; fixes the speed deficit (prefer a `move_speed` `entity_templates` column over a follow-tick catch-up multiplier, since it unblocks any future escort); optionally hardens `npc_ai_leash` to skip the spawn-snap while `follow_target_id.is_some()`. Tests: loader unit (`use_player` resolves to the triggering player), speed-scale unit, a `find_path`-returns-`None` guard proving the straight-line-through-geometry fallback is what actually happens today (the silent-failure shape noted above).
- **GC1b-1 — Marsh rides the rings.** Seed only, no dependencies. One `move_waypoint` action on the existing region-3 `teleport_in` trigger, destination inside navmesh component 8 near the Ring 3 pad. Watch for the same AoI-on-spawn failure shape as the open Castle_CellBlock invisible-corpse bug (issue #582's `aoi.create_emit`/`create_send_failed` seams) — exercise deliberately in UAT. Chain-replay test asserts Marsh's post-`teleport_in` position lands in component 8.
- **GC1b-2 — Marsh follows across the topside.** Seed only, hard-depends on GC1b-0. One `set_follow_target` with `use_player: true` fired after GC1b-1's reposition; cleared (no `target_tag`) at the Straegis scene (C08b already despawns Marsh there). Chain-replay test steps the player along hallway waypoints and asserts Marsh's `nav_path` stays non-degenerate (>1 waypoint) — that's what fails if he's cutting straight lines through walls.
- **GC1c — lockdown VFX.** No packet. Confirmed still BlockedEvidence; nothing new recovered (no energy-field actor or Kismet event id; sequence 10000 is already bound to the mission-622 exit door in chain 1004, so it can't double as the lockdown route).

**Suggested sequencing:** GC1a and GC1b-1 are independently shippable the moment C01/C08a land. GC1b-0 can be written in parallel with those two (disjoint files — Rust engine code vs. seed). GC1b-2 waits on GC1b-0. Shipping GC1a + GC1b-1 alone already gets Marsh talking and arriving topside (strictly more than the 2009 server did) even before GC1b-0/GC1b-2 land. **Exclude:** path interpolation or a generic NPC-arrival trigger (engine Tier 3); this reuses `move_waypoint` and `set_follow_target` as they already exist plus the two additive changes GC1b-0 lists.

### GC2

**Status:** Closed, no packet needed. **Scope title:** Aftermath per-class rewards (spec row 22). **Advisors:** items-systems-advisor (DB evidence pass), game-archaeology-specialist (RE pass). **Decision:** D-CB06 (answered, then reversed after evidence).
**Entries:** chains 1098/1099, [Aftermath.py](../../../deprecated/python/cell/missions/Castle_CellBlock/Aftermath.py), [Aftermath.script](../../../deprecated/data-scripts/scripts/missions/Castle_CellBlock/Aftermath.script), dialogs 2517/3942/3943/4408/4409, `db/resources/Items/Seed/items.sql`.

**Resolution (evidence-driven, 2026-09-17):** the user's initial D-CB06 answer ("invest in the per-class split") was made on the spec's guess that dialogs 2517/4408/4409 mapped to unshipped Soldier/Commando/Scientist-Archeologist rewards. A two-stage evidence pass overturned that premise:

1. `items-systems-advisor` (DB-only pass): found the three dialogs are real, unused rows in the same `dialog_set_id = 628` topic group, but have zero DB wiring to any item (no `event_set_id`, no `items_event_sets.sql` row) and no dead/commented branch in `Aftermath.py` to recover a rule from. Flagged as not scriptable from repository data alone.
2. `game-archaeology-specialist` (RE pass): found the decisive artifact — `Aftermath.script`, the raw Atrea node-graph source `Aftermath.py` was compiled from. It's a complete, unbroken graph (every node 2-25 present and enabled, no dead branches), and the original designer's own comments on the two branch-comparison nodes read `"Human"` and `"JAffa"`. **This was always a deliberate two-way species split, not truncated content.** Dialogs 2517/4408/4409 were never wired to this graph at any revision there's evidence of (2517's low id suggests an early draft superseded once the designer settled on the two-branch design). A Ghidra string search of `SGW.exe` for this content found zero hits, confirming BigWorld keeps mission/dialog/item text entirely server-side — there is no client-side reward table to recover from either.

The one structurally real gap the RE pass found — the shipped Python's `archetype < 5 OR archetype == 8` excludes archetype 6 (Goa'uld), who IS a Praxis-reachable starting archetype per `crates/services/src/base/chardef.rs` — turned out to already be a non-issue in this repository: unlike the Python, Cimmeria's chains 1098/1099 gate on `archetype neq 8` / `archetype eq 8` (verified directly, `castle_cellblock_chains.sql` lines 1386/1413), the same "non-Jaffa vs Jaffa" pattern used consistently everywhere else in the Cellblock seed (chains 1011/1012, 1051/1052, 1056/1057). Goa'uld already receives the Human-branch stealth-set reward today, not nothing — a Cimmeria-side deviation from the original script that happens to already close the one real gap the archaeology surfaced.

Given this, the user chose (2026-09-17) to keep the shipped two-branch mapping as-is and close GC2 with no further packet. Chains 1098/1099 already implement it faithfully (verified byte-exact against `Aftermath.py` and `chain_replay_tests/mission_687.rs`). **No implementation work follows from GC2.**

## Boundary

### C09

**Status:** Reassigned off this ledger, 2026-09-17. **Scope title:** ~~Castle arrival and Sgt. Gerschon handoff~~.

A sibling session is running its own Castle-zone restoration campaign and owns `castle_chains.sql` as **CA01** in its own ledger. Two sessions both creating that file was the one guaranteed conflict between the two campaigns, so the full authoring scope below moved there. Original scope kept verbatim for reference — do not implement any of it from this ledger:

> new seed file `castle_chains.sql` (`\ir` after the Cellblock files) porting only the handoff slice of `Castle.py`: on `player_loaded Castle` with 701 not active, bind dialog set 3062 to template 149; on interact `Castle_SgtGerschon` display 2573 (archetype neq 8) or 5861 (archetype eq 8, spec-only addition, same shape as the Prisoner 329 branch); on the matching dialog choice accept 701 and remove the set. Verify the arrival coordinate in chain 1109 lands within interaction range of spawn 112 or on the ring platform the comment cites. Mission 1360 must still be active after arrival. Stop at 701 accept; the rest of `Castle.py` (Copplemann, Livewire, 702) is a separate zone campaign.

**What this ledger keeps:** one UAT check, folded into milestone M4 in [README.md](README.md#validation-and-uat-gates) — after CA01 lands on the Castle side, confirm the player arrives near spawn 112 `Castle_SgtGerschon` with mission 1360 (Frost's Letter, C04) still active. No packet, no chain authoring, no `castle_chains.sql` edits from this session. **Entries (read-only, for the UAT check only):** chain 1109 (`cross_world_teleport`, already landed), spawn 112 `Castle_SgtGerschon` (world 8).

### GC3

**Status:** BlockedDesign. **Scope title:** Mission completion XP (spec observed 52 XP for 680/681/686). **Advisors:** combat-systems-advisor, database-persistence.
**Entries:** `Action::GrantXP` (no loader or executor arm), [progression/mod.rs](../../../crates/services/src/base/world_entry/methods/progression/mod.rs) (`GrantXP` sink used by mob kills and `.givexp`), `missions.award_xp` (true) and `reward_xp` (0) columns, `.claude/plans/2026-03-08-xp-leveling-design.md`.
**Approve:** the formula. 52 XP is an observation from `Mob Stats.txt`, not a DB value; every `reward_xp` is 0, so the original computed it (level or difficulty based). Guessing a constant is not restoration. **Required children:** formula evidence (RE or spreadsheet cross-check); `grant_xp` loader plus executor arm reusing the progression sink; seed rows on the completion chains. **Exclude:** a hardcoded 52.

### C10

**Status:** Ready (rolling, coordinator-owned). **Scope title:** Documentation sync. **Advisor:** documentation-writer.
**Entries:** [mission-chains.md](../../content/mission-chains.md) (688 section is missing; Aftermath step 2355 and the Cimmeria re-tags are undocumented; 1360 once C04 lands), [content-engine.md](../../content/content-engine.md) section 3 catalog (after C03/C08a add arms and delay support), [docs/readme.md](../../readme.md) index, [zone-audit.md](../../content/zone-audit.md) Castle_CellBlock row ("13 scripted" is stale; 688 exists).
**Scope:** one doc update per integrated packet, in the same PR. **Acceptance:** `tools/lint-md.ps1 --no-globs <paths>` clean on touched files; index rows resolve.

## Explicit Non-Goals (record as decisions, do not open packets)

| Item | Why |
|---|---|
| Symbiote Loss (ability 1926 / effect 2480) for Jaffa | Never wired in the original; ability has a placeholder name. D-CB12. |
| Stasis Sickness Stage 2 timer (ability 1373) | Needs the engine timer primitive; escalation timing unknown. |
| Hidden mission 642, Frost-alive intro, Future Self dialog | Legacy revision; nothing in the seed references them. |
| Praxis Goa'uld path | Spec excludes it; note only that char defs 10/19 start here and take the Human branch today. |
| Stasis pod actors, energy-field actor, blood decal, rift creature, data disc | No recovered actor, event id, template or item id. |
| Ring ceremony on the 688 exit | Deliberate Cimmeria design in chain 1109; prerelease known issue. |

## Scheduling And Closeout

Dependency roots: authorize implementation, then C01 (purge) and C08a (delay support) can run in parallel with disjoint files; C02 follows C01; C03 and C04 follow C01. C05 needs the cover-set evidence step before its writer starts; C06 follows C05. C07 waits on the UAT precheck in D-CB08. C08b waits on C08a. C09 is reassigned off this ledger (see its entry) — its only remaining item is a UAT check gated on the sibling Castle campaign's own CA01 landing. GC1/GC2/GC3 are design gates: propose children, record a decision id, then dispatch.

Milestones for the user's in-client UAT are in [README.md](README.md#validation-and-uat-gates). A packet becomes Done only after its replay guard, executor test where applicable, documentation update and milestone UAT pass.
