# Legacy Command Work Packets

> Type: how-to. Audience: Claude Code coordinator and Sonnet packet workers.
> Updated: 2026-09-16. Companions: [launch prompt and decisions](README.md), [static audit](audit.md), [testing playbook](../../../TESTING.md).

## Dispatch Rules

This is the mutable execution ledger; [audit.md](audit.md) remains the source baseline. Initial state: documentation prepared against `6279fcfb53a7325ec9f00ff705e027b6e0d39ce0`; no implementation, builds, runtime tests or client UAT run. Implementation-session authorization is still required. Every baseline command maps to a packet below; G groups retain larger work instead of hiding it as parity.

Status vocabulary: **Ready** means dependencies permit dispatch after session authorization; **BlockedDependency** means listed prerequisites must integrate first; **BlockedDesign** means user approval and explicit bounded child packets are required. Later use **Writing**, **Review**, **Integrated**, **UATPending**, **Done**, **BlockedEvidence**. Record worktree/base, owner, notes/handoff links, validation and decision IDs beside the packet's status as work proceeds. None is Done initially.

P01 is the first Ready packet. Every other P packet initially has status BlockedDependency, at least on P01. Every G group initially has status BlockedDesign, even where evidence gathering can occur independently. `Gxx children` as a dependency means the relevant approved implementation children are integrated, not merely that a design document was written. G groups cannot be dispatched as single implementation tasks. Each G group below supplies required child boundaries and acceptance topics; create numbered child manifests only after design approval.

The default writer for each P packet is `rust-gameserver-dev` on Sonnet; its Advisor field adds domain review. G groups are coordinated by the named advisors, with the same writer for approved children. `testing-validation-engineer` reviews every regression strategy and `documentation-writer` reviews corresponding public documentation. Discover actual agent availability/model overrides as described in [README.md](README.md#agent-selection); do not create agents.

## Worker Input And Ownership

Give each worker only its packet, relevant audit rows and decision IDs, common safety rules, owned-path manifest and dependency handoffs. Do not dump all 116 rows into every context. Start with 4-8 nearby source files: the common three below plus the packet's linked entries. If commands span legacy families, include only the relevant function bodies in those family files. These are entry sets, not permission for recursive whole-subsystem exploration.

Common read-only entries are [registry.rs](../../../crates/services/src/cell/console/registry.rs), [dispatch.rs](../../../crates/services/src/cell/console/dispatch.rs), and the relevant imported legacy family file linked by the [audit](audit.md). P01 uses ConsoleCommands.py as that third file. Read repo instructions and TESTING.md once before authoring; use [console/tests.rs](../../../crates/services/src/cell/console/tests.rs) as the neighboring fixture when applicable. Count it in the eight-file working set if opened.

The coordinator assigns exact owned paths before dispatch. Entries below are candidate implementation surfaces; shared registry/dispatch, module exports, message enums, common fixtures and shared docs remain read-only unless exclusively assigned. Workers include requested integration edits in their handoff. Keep tests near existing tests, and never share a mutable test file between concurrent writers. One behavior contract per packet: when a leaf requires independent state machines, more than eight starting files, or an unresolved design, stop and split before widening.

Use `worknotes/<packet-id>.md` and `handoffs/<packet-id>.md` for future durable artifacts. Link them only once created. Record source/base, ownership, hypothesis/evidence, exact commands and exit codes/skips, regression proof, review, dependencies, blockers and UAT pending/results. Carry both into integration. A context-pressure handoff is a valid stopped packet, not permission to omit evidence.

## Common Acceptance

All leaves need exact command spelling/arity/defaults, finite/range-checked numeric input, legacy boolean/string contracts, no-target/wrong-type/stale/cross-space rejection as applicable, modern GM denial and caller/subject separation. Invalid requests leave state, DB, messages and event counts unchanged. Assert exact recipient sets and outcomes; queue acceptance is not success. A command whose effect is source-implemented still needs meaningful verification, not a smoke-only dispatch assertion.

Test contracts below are planned, not existing/passed tests. At dispatch, name concrete tests and select a unique filter such as `legacy_p09_`; record the actual names in the worknote. Under the single machine-wide build lane, run `cargo check -p cimmeria-services`, then `cargo test -p cimmeria-services --lib <filter> -- --test-threads=1` for the touched tests. Target another crate explicitly if the owned model lives there. Use `cargo nextest run --profile=ci-live-db -p cimmeria-services --lib <filter>` for live-DB acceptance with DATABASE_URL configured; record skip status and never equate missing DB with passed durability. Read current test-policy commands before execution. A filter matching zero tests fails acceptance.

For every runtime leaf, add/update a regression that fails without the fix. Model-only checks do not replace command -> operation -> sink -> fresh-hydration checks for persistence. Client effects require byte-exact serializer and transport/fan-out tests where applicable, followed by milestone UAT from README. Tests use existing fixtures, no hardcoded seed IDs, exact sentinel cleanup. Update the corresponding CLAUDE-mapped behavior docs and lint touched paths. Full pre-PR gates belong to the coordinator, serialized, with all five required workspace exclusions. No build runs in the current documentation task.

## Foundation And Quick Wins

### P01

**Status:** Ready. **Commands:** `.help`, `.searchitem`, `.searchmission`, `.searchtemplate`; catalogue/target regression foundation. **Depends:** implementation-session authorization. **Advisor:** server-authority-enforcer.
**Entries:** [query](../../../crates/services/src/cell/console/query.rs), [chat gate](../../../crates/services/src/cell/chat.rs), [console tests](../../../crates/services/src/cell/console/tests.rs), [search sink](../../../crates/services/src/base/console_authoring.rs).
**Scope:** restore argument-level help; pin baseline names/arity/target contracts and protect Rust-only names. Verify search adapters and real result routing; restore literal substring matching while retaining the 25-result bound with explicit truncation feedback. Do not register unimplemented commands merely to satisfy the catalogue test; stage expected implemented subsets as packets integrate, preserving the full backlog denominator. Split search into a bounded child if DB checks expand the foundation packet.
**Acceptance:** exact sorted/filter/help argument output; live-DB one/two-token case-insensitive literal search, including `%`, `_`, backslash, exactly 25 and more than 25 matches, and empty/error responses; GM denial, target validation and caller attribution. **Exclude:** generic commands-crate registry, broad parser redesign, fake parity stubs.

### P02

**Commands:** `.info`, `.facing`, `.combatinfo`. **Depends:** P01. **Advisor:** combat-systems-advisor.
**Entries:** [native queries](../../../crates/services/src/cell/cell_methods/gm/query.rs), [console query](../../../crates/services/src/cell/console/query.rs), [entity model](../../../crates/entity/src/cell_entity/mod.rs).
**Scope:** read-only detailed entity/geometry/combat feedback; selection overrides explicit info ID. Reuse actual flags/geometry and legacy template, weapon, ability-set and ability-type diagnostics; label unavailable fields rather than inventing them.
**Acceptance:** selected versus explicit ID precedence, unknown ID, radians/degrees/class/distance fixtures, missing template/weapon/ability-set cases, exact ability-type counts and caller-only output. **Exclude:** mutation, damage or AI instrumentation; split if geometry/model work ceases to be read-only.

### P03

**Commands:** `.stats`, `.primarystats`, `.speedstats`, `.armorstats`, `.qrstats`, `.absorbstats`, `.stealthstats`. **Depends:** P01. **Advisor:** combat-systems-advisor.
**Entries:** [console stats](../../../crates/services/src/cell/console/stats.rs), [console tests](../../../crates/services/src/cell/console/tests.rs).
**Scope:** one shared stat-readout contract; add basic four-stat group and verify the six existing exact sets. This is one table-driven behavior, not seven subsystem rewrites.
**Acceptance:** exact names/current/max values for every group, missing-stat behavior, selected-target and caller feedback isolation. **Exclude:** stat setters, balance changes, fabricated default values.

### P04

**Commands:** `.listabilities`, `.players`. **Depends:** P01. **Advisor:** server-authority-enforcer.
**Entries:** [query](../../../crates/services/src/cell/console/query.rs), [ability manager](../../../crates/entity/src/abilities/manager.rs), [ability definitions](../../../crates/services/src/cell/spawner/abilities.rs).
**Scope:** read-only roster feedback: selected abilities with names/fallback; service-local online names/worlds including transitions. Split if online indexing needs new lifecycle state.
**Acceptance:** unknown ability fallback, deterministic output, two loaded spaces plus transitioning player, no mutation and caller-only results. **Exclude:** cluster/offline roster, ability grants.

### P05

**Commands:** `.givecash`, `.givexp`. **Depends:** P01. **Advisor:** database-persistence.
**Entries:** [native give](../../../crates/services/src/cell/cell_methods/gm/give.rs), [progression sink](../../../crates/services/src/base/world_entry/methods/progression/mod.rs).
**Scope:** selected-player typed grants through existing sinks, carrying separate caller feedback identity. Keep current amount bounds; resolve signed input before unsigned conversion.
**Acceptance:** exact cash/XP/level/TP totals after DB reload, distinct caller unaffected, target-only UI, overflow/invalid/no-DB failure truthfulness. **Exclude:** level-setting, training, global economy refactor.

### P06

**Commands:** `.giveitem`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence.
**Entries:** [grant-item sink](../../../crates/services/src/base/world_entry/methods/inventory/grant/grant_item.rs), [inventory executor](../../../crates/services/src/cell/content/executor/inventory.rs).
**Scope:** adapt selected-player design-ID/quantity grant using correct container routing and existing transaction/outbox synchronization.
**Acceptance:** merge/new-stack quantities and ownership exactly match after reload; inventory-full, invalid design and DB failure do not report success; target UI/caller feedback split. **Exclude:** name-based lookup, inventory redesign.

### P07

**Commands:** `.removeitem`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence. **Decision:** D07.
**Entries:** [remove-by-type](../../../crates/services/src/base/world_entry/methods/inventory/core/remove_by_type.rs), [inventory dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/inventory_dispatch.rs), [legacy inventory](../../../deprecated/python/cell/Inventory.py).
**Scope:** atomic exact-quantity removal across matching design stacks, retaining inventory lock/outbox rules. Keep native instance-ID removal separate.
**Acceptance:** multiple stacks with exact remainder; insufficient aggregate, locked/ineligible stock and injected mid-operation failure leave every row unchanged; two-player isolation and exact updates. **Exclude:** treating partial removal as success, changing UseInventoryItem consumption.

## World Authoring

### P08

**Commands:** `.spawn`, `.despawn`. **Depends:** P01. **Advisor:** npc-ai-spawn-advisor, aoi-witness-broadcast.
**Entries:** [native GM](../../../crates/services/src/cell/cell_methods/gm/mod.rs), [console spawn](../../../crates/services/src/cell/console/spawn.rs), [space entities](../../../crates/services/src/cell/space_manager/entities.rs).
**Scope:** ephemeral lifecycle adapters, validated template, caller placement/heading and safe selected-entity destruction. Trace actual creation result, not only enqueue.
**Acceptance:** one created entity, exact position/heading, wrong template rejected, despawn cleanup and witness removal; no spawnlist mutation. **Exclude:** autosave/persistence and deleting arbitrary players through an NPC-only primitive.

### P09

**Commands:** `.savespawn`. **Depends:** P08. **Advisor:** database-persistence, npc-ai-spawn-advisor.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn.rs), [authoring sink](../../../crates/services/src/base/console_authoring.rs), [seed recording](../../../crates/services/src/cell/console/seed.rs).
**Scope:** typed correlated insert result assigns spawn_id to the same surviving entity; update exact row including world/template/placement/tag. Preserve live SQL/log/buffer semantics.
**Acceptance:** saving twice leaves exactly one row and stable ID; update all fields; failure/despawn-before-result cannot attach ID to another entity; truthful feedback. **Exclude:** generic rowcount as identity, appearance persistence, seed permission redesign.

### P10

**Commands:** `.delspawn`. **Depends:** P09. **Advisor:** database-persistence.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn.rs), [authoring sink](../../../crates/services/src/base/console_authoring.rs).
**Scope:** delete exact persistent row, clear runtime spawn_id only after success, leave entity alive. `.despawn` remains a separate runtime operation.
**Acceptance:** DB row absent and same runtime entity present; failed delete leaves both unchanged; sibling rows untouched; authoring output accurate. **Exclude:** unconditional runtime destruction or broad-key deletes.

### P11

**Commands:** `.autosavespawn`. **Depends:** P08, P09. **Advisor:** npc-ai-spawn-advisor.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn.rs), [space entities](../../../crates/services/src/cell/space_manager/entities.rs), [seed](../../../crates/services/src/cell/console/seed.rs).
**Scope:** consume per-GM preference on successful `.spawn` result for the newly created entity. Do not copy Python's stale-target autosave bug.
**Acceptance:** enabled saves exactly the new entity once, disabled saves none, two-GM isolation, invalid spawn produces no row, disconnect cleans preference. **Exclude:** retroactive saving, implicit random-spawn autosave without an approved contract.

### P12

**Commands:** `.spawnrandom`. **Depends:** P08. **Advisor:** npc-ai-spawn-advisor.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn.rs), [legacy Resource](../../../deprecated/python/cell/commands/Resource.py).
**Scope:** independent uniform XZ offsets inside rectangle, same Y and caller heading, current 1-50 safety bound. Reuse existing RNG conventions and injectable seed for tests.
**Acceptance:** reproducible seeded interior points, exact count/Y/heading, range and malformed argument checks, partial creation failures reported accurately. **Exclude:** deterministic ring, unbounded spawning, new RNG framework.

### G01

**Status:** BlockedDesign. **Commands:** `.respawnall`. **Depends:** P08, G09 resource-cache child. **Advisors:** npc-ai-spawn-advisor, database-persistence, aoi-witness-broadcast.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn.rs), [space lifecycle](../../../crates/services/src/cell/space_manager/lifecycle.rs), [legacy Resource](../../../deprecated/python/cell/commands/Resource.py).
**Approve:** snapshot scope, template reload, removal/loading failure semantics and exclusion of players/unrelated instances. Existing field reset is not full parity.
**Required children before dispatch:** template/spawn snapshot staging (failed load preserves old world); runtime cleanup/replacement (death/effects/threat ownership cleaned, unrelated instance unchanged); witness recreation/result reporting (exact disappear/appear sets). **Exclude:** global world reset, partial-field reset presented as full reload.

### P13

**Commands:** `.name`, `.nameid`, `.alignment`, `.faction`. **Depends:** P01. **Advisor:** aoi-witness-broadcast.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [cell entity](../../../crates/entity/src/cell_entity/mod.rs).
**Scope:** validated named mappings and authoritative identity-property publication for selected beings/spawnables as registered. Faction/level fields already exist; do not invent duplicate storage.
**Acceptance:** exact legacy mapping values, target and observers update immediately, unrelated entity unchanged, invalid names fail; feedback makes no false savespawn promise. **Exclude:** adding appearance fields to spawn persistence or character-rename policy changes without approval.

### P14

**Commands:** `.eventset`, `.interactiontype`, `.tag`. **Depends:** P01; serialize with P13/P15 if sharing entity file. **Advisor:** aoi-witness-broadcast, mission-systems-advisor.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [console spawn](../../../crates/services/src/cell/console/spawn.rs).
**Scope:** publish event/interaction property changes and verify tag clear/update semantics. Distinguish runtime authoring from fields actually saved by savespawn.
**Acceptance:** immediate target/observer bytes, interaction click reaches intended server path after flag update, `none` clears tag, tag save roundtrip when P09 is available. **Exclude:** new content chains, claiming eventset persists through spawn SQL.

### P15

**Commands:** `.staticmesh`, `.bodyset`, `.addcomponent`, `.delcomponent`, `.dynamicupdate`. **Depends:** P01. **Advisor:** aoi-witness-broadcast.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [base AoI dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/aoi_dispatch.rs).
**Scope:** one authoritative appearance-refresh contract across player/NPC representations. Follow RefreshAppearance to final assembly before editing; split player/NPC children if they require distinct state machines.
**Acceptance:** final emitted appearance contains edited mesh/body/components, remove is real, repeated add does not duplicate, NPC refresh works, exact self/witness recipients. **Exclude:** marking an enqueue as visual proof or changing equipment persistence.

### P16

**Commands:** `.visible`. **Depends:** P01. **Advisor:** aoi-witness-broadcast, server-authority-enforcer.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [space entities](../../../crates/services/src/cell/space_manager/entities.rs).
**Scope:** authoritative reversible visibility with selection-first/explicit-ID fallback. Resolve state ownership before implementation; new shared visibility policy requires a design child if no suitable abstraction exists.
**Acceptance:** hide removes exactly current witnesses, hidden entities do not reappear on ordinary AoI updates, show re-enters correctly, repeated calls stable, late observer behavior correct. **Exclude:** feedback-only show, broad stealth/gameplay redesign.

### P17

**Commands:** `.setstate`, `.unsetstate`, `.setcombatant`, `.unsetcombatant`. **Depends:** P01. **Advisor:** combat-systems-advisor, server-authority-enforcer.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [cell entity](../../../crates/entity/src/cell_entity/mod.rs), [legacy Entity](../../../deprecated/python/cell/commands/Entity.py).
**Scope:** named BSF state-field and PLAYER_STATE combatant domains, preserving effect/refcount ownership. Resolve named mapping once, then mutate correct domain through its canonical operation.
**Acceptance:** each command alters only intended domain; repeated set/clear and overlapping effect ownership remain consistent; exact wire flags and invalid suffix rejection. **Exclude:** numeric index treated as mask, clearing another owner's state.

### P18

**Commands:** `.location`, `.rotation`, `.lookat`. **Depends:** P26. **Advisor:** movement-teleport-advisor, aoi-witness-broadcast.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs), [teleport](../../../crates/services/src/base/world_entry/teleport.rs).
**Scope:** selected spawnable read/set placement; zero or complete XYZ tuple, look-at orientation publication. Reuse snap abstraction; if full Euler orientation cannot be represented, stop for explicit design rather than silently dropping axes.
**Acceptance:** exact final XYZ/orientation, malformed partial tuple causes no mutation, spatial index/validator/witness/client agree for player and NPC. **Exclude:** world transfer, read-only replacements for legacy setters.

## Missions

### P19

**Commands:** `.missionlist`, `.missionlistfull`, `.missiondetails`. **Depends:** P01. **Advisor:** mission-systems-advisor.
**Entries:** [native missions](../../../crates/services/src/cell/cell_methods/gm/missions.rs), [mission model](../../../crates/entity/src/missions.rs), [console mission](../../../crates/services/src/cell/console/mission.rs).
**Scope:** selected-player readouts including hidden active missions and clear status labels. Display existing detail honestly; document fields awaiting G03 instead of fabricating objectives/script state.
**Acceptance:** mixed hidden/visible active/completed/failed fixture, exact list membership/detail values, absent ID, caller-only output. **Exclude:** mutation or new persistence representation in a query packet.

### G03

**Status:** BlockedDesign. **Commands:** foundation for P20-P25 and G04/G09 mission work. **Depends:** P01. **Advisors:** mission-systems-advisor, database-persistence, server-authority-enforcer.
**Entries:** [mission model](../../../crates/entity/src/missions.rs), [mission sink](../../../crates/services/src/base/world_entry/methods/missions.rs), [player hydration](../../../crates/services/src/cell/service/base_messages/player_init/mod.rs), [content mission](../../../crates/services/src/cell/content/executor/mission.rs).
**Approve:** exact snapshots/failed objectives, hidden/optional metadata, missing-step representation, mutation/commit/event ordering, definition flags and mission-bound cleanup. Do not revive stale claims that the current UPSERT omits repeats.
**Required children:** snapshot/model contract (real objective IDs and all history/repeats); durable sink/delete contract (exact ownership and rollback); hydration contract (roundtrip including missing step/failed objectives/hidden metadata). Each child needs exact serialized live-DB fixtures; subsequent lifecycle leaves consume them. **Exclude:** one Sonnet packet replacing the mission subsystem.

### P20

**Commands:** `.missionaccept`. **Depends:** G03 children. **Advisor:** mission-systems-advisor.
**Entries:** [lifecycle](../../../crates/services/src/cell/missions/lifecycle.rs), [content mission](../../../crates/services/src/cell/content/executor/mission.rs), [native missions](../../../crates/services/src/cell/cell_methods/gm/missions.rs).
**Scope:** checked shared accept, definition/first-step resolution, accurate persistence and acceptance chains, selected recipient.
**Acceptance:** command -> sink -> fresh hydration exact state/objectives, duplicate/invalid accept unchanged, acceptance event exactly once, client status verified. **Exclude:** native assign reused without its missing persistence/events.

### P21

**Commands:** `.missionadvance`, `.missionreset`. **Depends:** P20 and G03 children. **Advisor:** mission-systems-advisor.
**Entries:** [progression](../../../crates/services/src/cell/missions/progression.rs), [content mission](../../../crates/services/src/cell/content/executor/mission.rs), [legacy manager](../../../deprecated/python/cell/MissionManager.py).
**Scope:** normal forward-only versus forced-order step policy; both require active mission and owned step. Preserve accurate objective history under the approved design.
**Acceptance:** forward/same/backward/foreign-step/terminal matrices, exact snapshots after relog, intended events once, malformed requests unchanged. **Exclude:** reset as clear/reaccept or resurrection of terminal history.

### P22

**Commands:** `.missionabandon`, `.missionfail`. **Depends:** G03 children. **Advisor:** mission-systems-advisor, server-authority-enforcer.
**Entries:** [console mission](../../../crates/services/src/cell/console/mission.rs), [lifecycle](../../../crates/services/src/cell/missions/lifecycle.rs), [mission sink](../../../crates/services/src/base/world_entry/methods/missions.rs).
**Scope:** failed-history transition with approved abandonment flags/forced-fail policy, failed-objective persistence and cleanup. Correct Python wrapper accept bugs.
**Acceptance:** active/hidden/non-abandonable cases, exact failed history after fresh hydration, dialogs cleaned, target-only UI and caller audit, no duplicate transition. **Exclude:** forgetting the record or copying internal status integers blindly onto wire.

### P23

**Commands:** `.missionclear`. **Depends:** G03 children. **Advisor:** mission-systems-advisor, database-persistence.
**Entries:** [lifecycle](../../../crates/services/src/cell/missions/lifecycle.rs), [mission sink](../../../crates/services/src/base/world_entry/methods/missions.rs), [console mission](../../../crates/services/src/cell/console/mission.rs).
**Scope:** durable forget of one exact player/mission record plus mission-bound cleanup, separate from fail/history-retaining abandon.
**Acceptance:** record absent after relog, unrelated player/mission untouched, failed DB delete preserves approved runtime state, exact removal notification. **Exclude:** range deletion, failed status as substitute for deletion.

### P24

**Commands:** `.missionclearactive`, `.missionclearhistory`. **Depends:** P23. **Advisor:** mission-systems-advisor.
**Entries:** [mission model](../../../crates/entity/src/missions.rs), [console mission](../../../crates/services/src/cell/console/mission.rs), [mission sink](../../../crates/services/src/base/world_entry/methods/missions.rs).
**Scope:** status-partitioned bulk use of approved clear semantics including hidden missions. Declare bulk failure/atomicity policy before editing; split a new batch sink if necessary.
**Acceptance:** mixed fixture proves exact complementary sets survive after reload; unrelated player unchanged; failure outcome matches policy. **Exclude:** legacy clearactive-all bug and ambiguous partial-success feedback.

### P25

**Commands:** `.missioncomplete`. **Depends:** G03 children. **Advisor:** mission-systems-advisor, server-authority-enforcer. **Decision:** D08.
**Entries:** [mission progression](../../../crates/services/src/cell/missions/progression.rs), [content mission](../../../crates/services/src/cell/content/executor/mission.rs), [mission sink](../../../crates/services/src/base/world_entry/methods/missions.rs).
**Scope:** guard completion before any mutation/DB/event; share safe operation with content caller where applicable, accurate snapshot, no automatic catalog payout.
**Acceptance:** absent/failed/completed requests leave state/repeats/DB/events unchanged; active completes once and survives relog; cash/XP/items unchanged absent separately authorized chain effects. **Exclude:** guard only on follow-up event, reward claim implementation.

### G04

**Status:** BlockedDesign. **Commands:** `.missionrewards`. **Depends:** G03 children, P25. **Advisors:** mission-systems-advisor, items-systems-advisor, database-persistence, server-authority-enforcer.
**Entries:** [reward groups schema](../../../db/resources/Missions/Tables/mission_reward_groups.sql), [reward items schema](../../../db/resources/Missions/Tables/mission_rewards.sql), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), [alias.xml](../../../entities/defs/alias.xml), [incoming dispatch](../../../crates/services/src/cell/cell_methods/player/world/mod.rs).
**Approve:** eligibility, completed-mission offers, pending authority, exactly-once key, rollback, choice counts/groups, no-item rewards, disconnect/relog retry and completion ordering. Plain preview is not accepted.
**Required children:** catalog/serializer (exact client-127 bytes and incoming choices-then-missionId order); authoritative offer state (forged/stale offers denied); atomic claim (duplicate/concurrent/retry and inventory-full/DB rollback); post-commit sync/completion (single payout, exact client totals, relog recovery). Approve each manifest before coding. **Exclude:** separate grant messages asserted to be an atomic claim.

## Player Administration

### P26

**Commands:** `.gotoxyz`. **Depends:** P01. **Advisor:** movement-teleport-advisor, aoi-witness-broadcast.
**Entries:** [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs), [teleport receiver](../../../crates/services/src/base/world_entry/teleport.rs), [AoI update](../../../crates/services/src/mercury/aoi/update.rs).
**Scope:** typed selected-or-caller in-space snap with finite coordinates, movement validator/spatial updates, correct player persistence; NPC stays cell-side.
**Acceptance:** exact final positions, forced-position bytes and observer updates; caller unchanged when moving selection; DB failure handled truthfully. **Exclude:** streaming hint alone, cross-space transfer, fake native input buffers.

### G05

**Status:** BlockedDesign. **Commands:** `.goto`, `.summon`, `.gotolocation`. **Depends:** P26. **Advisors:** movement-teleport-advisor, aoi-witness-broadcast, database-persistence.
**Entries:** [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs), [gate-travel](../../../crates/services/src/base/world_entry/gate_travel/mod.rs), [space lifecycle](../../../crates/services/src/cell/space_manager/lifecycle.rs).
**Approve:** service-local name resolution/ambiguity and transfer target identity, loaded-instance addressing, validation before teardown, failure/disconnect ownership and rollback.
**Required children:** online name/destination resolution (selection semantics and in-transition cases); typed instance-targeted transfer (same-world distinct-instance test, exact destination instance ID); command adapters/failure recovery (world validity, disconnect at each stage, AoI teardown/re-entry). **Exclude:** offline/cluster routing and find-or-create-by-world as proof of joining a named player's instance.

### P27

**Commands:** `.kill`. **Depends:** P01. **Advisor:** combat-systems-advisor.
**Entries:** [death](../../../crates/services/src/cell/abilities/death.rs), [damage apply](../../../crates/services/src/cell/abilities/damage_apply/mod.rs), [native GM](../../../crates/services/src/cell/cell_methods/gm/mod.rs).
**Scope:** selected-being death including players through canonical lifecycle; reuse NPC kill where valid. Resolve kill-versus-god policy under G06 before their joint acceptance.
**Acceptance:** player and NPC terminal flags, health, cleanup and notifications exactly once; already-dead behavior stable; caller not killed accidentally. **Exclude:** health-zero-only implementation and NPC handler relabeled as player support.

### P28

**Commands:** `.revive`. **Depends:** P27. **Advisor:** combat-systems-advisor, aoi-witness-broadcast.
**Entries:** [respawn](../../../crates/services/src/cell/cell_methods/player/combat/respawn.rs), [death](../../../crates/services/src/cell/abilities/death.rs), [stat executor](../../../crates/services/src/cell/content/executor/stats.rs).
**Scope:** in-place revival with canonical dead-state cleanup and stat/client synchronization. Establish which respawn-only side effects must not run.
**Acceptance:** exact unchanged position/instance, restored living state and approved stats, corpse/effect/AI consistency for player/NPC, repeat stable and observers updated. **Exclude:** teleport-to-respawner, unrelated cooldown reset without contract evidence.

### G06

**Status:** BlockedDesign. **Commands:** `.god`. **Depends:** P27, P28 for lifecycle integration. **Advisors:** combat-systems-advisor, server-authority-enforcer.
**Entries:** [damage pipeline](../../../crates/services/src/cell/combat/damage/pipeline.rs), [effect scripts](../../../crates/services/src/cell/effects/scripts.rs), [damage apply](../../../crates/services/src/cell/abilities/damage_apply/mod.rs).
**Approve:** invulnerability lifetime, persistence/relog, hostile/environmental/script/direct-stat damage, and explicit GM kill override policy.
**Required children:** authoritative toggle/state (default true, false restoration and two-player isolation); canonical damage-path guards (enumerated path tests including direct script health changes); lifecycle/relog cleanup (approved kill/revive/transfer/disconnect behavior). **Exclude:** guarding only one calculator or repeatedly topping up health after damage.

### P29

**Commands:** `.giveability`, `.clearabilities`. **Depends:** P01. **Advisor:** combat-systems-advisor, database-persistence.
**Entries:** [ability manager](../../../crates/entity/src/abilities/manager.rs), [ability-granted mirror](../../../crates/services/src/cell/service/base_messages/ability_granted.rs), [progression sink](../../../crates/services/src/base/world_entry/methods/progression/mod.rs).
**Scope:** free persisted ability membership mutation with shared post-commit mirror and weapon-granted tracking. Split grant/revoke child if shared atomic membership operation does not already fit.
**Acceptance:** grant costs zero TP, duplicate idempotence, invalid ID rejection, clear list after fresh login, weapon-granted consistency, exact target update. **Exclude:** calling paid training as a grant or adding a declared-but-unhandled content action.

### P30

**Commands:** `.givetp`. **Depends:** P05. **Advisor:** database-persistence.
**Entries:** [progression](../../../crates/services/src/base/world_entry/methods/progression/mod.rs), [native give](../../../crates/services/src/cell/cell_methods/gm/give.rs).
**Scope:** direct checked TP grant, authoritative DB total, connection cache and client property update.
**Acceptance:** exact TP after relog, no XP/level changes, selected-player isolation, invalid/overflow/DB-failure unchanged. **Exclude:** indirect XP grants or silently changing training cost policy.

### P31

**Commands:** `.level`. **Depends:** P05. **Advisor:** combat-systems-advisor, database-persistence.
**Entries:** [cell entity](../../../crates/entity/src/cell_entity/mod.rs), [progression](../../../crates/services/src/base/world_entry/methods/progression/mod.rs), [legacy Entity](../../../deprecated/python/cell/commands/Entity.py).
**Scope:** set existing level through validated being/player progression semantics. Confirm derived-stat/XP/TP policy locally; if no canonical set-level operation defines it, request a design child.
**Acceptance:** selected player/NPC level and derived state coherent, player reload/cache/client agree, no accidental grant, range rejection. **Exclude:** new duplicate level field or arbitrary progression redesign.

### G07

**Status:** BlockedDesign. **Commands:** `.giveaddress`, `.removeaddress`. **Depends:** P01. **Advisors:** movement-teleport-advisor, database-persistence.
**Entries:** [map-loaded](../../../crates/services/src/mercury/world_data/map_loaded.rs), [player schema](../../../db/sgw/Players/Tables/sgw_player.sql), [legacy SGWPlayer](../../../deprecated/python/cell/SGWPlayer.py).
**Approve:** hidden/known mutually exclusive state, serialization and DB representation compatible with existing gate authorization.
**Required children:** grant/revoke durable state (catalog validation and mutually exclusive lists); hydration/live wire integration (hidden list survives relog, revoke immediately visible); command adapters (selected-target/caller isolation). **Exclude:** append-only known list masquerading as hidden support.

### G08

**Status:** BlockedDesign. **Commands:** `.giverespawner`, `.removerespawner`. **Depends:** P01. **Advisors:** combat-systems-advisor, database-persistence.
**Entries:** [player schema](../../../db/sgw/Players/Tables/sgw_player.sql), [defeat choices](../../../crates/services/src/cell/abilities/damage_apply/mod.rs), [respawn](../../../crates/services/src/cell/cell_methods/player/combat/respawn.rs).
**Approve:** per-player unlock selection policy and compatibility with world eligibility/default respawners; current choices use global catalog.
**Required children:** durable unlock/revoke and hydration (exact set after relog); choice filtering/authorization (locked respawner unavailable, valid fallback defined); dot adapters/UI (two-player isolation and stale choice rejection). **Exclude:** DB-only append with unchanged gameplay choices.

## Maintenance And Diagnostics

### G09

**Status:** BlockedDesign. **Commands:** `.save`, `.reloadmap`, `.reloadres`, `.missionreload`, `.reloadscripts`. **Depends:** P01; mission restore also G03 children. **Advisors:** database-persistence, mission-systems-advisor, server-authority-enforcer.
**Entries:** [console server](../../../crates/services/src/cell/console/server.rs), [admin content route](../../../crates/admin-api/src/routes/content.rs), [cell loop](../../../crates/services/src/cell/service/message_loop.rs), [engine loader](../../../crates/services/src/cell/content/engine_loader.rs).
**Approve:** scope, complete state inventory, snapshot/commit/ack semantics and failure retention per operation. Python reload mechanism does not transfer literally; global chain reload is one existing capability only.
**Required separate children:** caller save flush (all approved state durable or explicit failure); selected-player reloadmap (state and instance preserved); resource-cache category/all reload (atomic usable snapshot, invalid category fails); mission-instance restore (variables/progress/bindings preserved, no duplicated rewards); global content reload adapter for reloadscripts (old engine retained on load failure). **Exclude:** one giant maintenance rewrite, global reload substituted for missionreload.

### G10

**Status:** BlockedDesign. **Commands:** `.loglevel`, `.logclient`. **Depends:** P01. **Advisors:** network-security-auth, server-authority-enforcer.
**Entries:** [console server](../../../crates/services/src/cell/console/server.rs), [observability crate](../../../crates/observability/src/), [server startup](../../../crates/server/src/main.rs).
**Approve:** legacy levels/categories mapped to tracing targets, scope/lifetime, redaction/rate/backpressure policy and privilege boundaries.
**Required separate children:** runtime filter reload (actual events change at runtime, invalid filter preserves old one); per-GM client subscription (toggle, redaction, bounded forwarding, disconnect cleanup and no leakage to others). **Exclude:** startup-env instructions as implementation or unbounded global callback forwarding.

### P32

**Commands:** `.debug_inven`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence.
**Entries:** [inventory methods](../../../crates/services/src/base/world_entry/methods/inventory/), [inventory dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/inventory_dispatch.rs).
**Scope:** read-only current-model inventory diagnostic. Choose the owning snapshot/model file inside the entry directory before dispatch; list real ownership/slots/orphans and supported pending operations, not fictional Python queues.
**Acceptance:** exact normal/orphan/duplicate-slot fixtures, caller-only selected-player report, no DB/runtime mutation, absent fields explicitly labeled. **Exclude:** repair, reload, unlocking or deleting inventory.

### G11

**Status:** BlockedDesign. **Commands:** `.debug_invreload`. **Depends:** P32. **Advisors:** items-systems-advisor, database-persistence, server-authority-enforcer.
**Entries:** [inventory methods](../../../crates/services/src/base/world_entry/methods/inventory/), [inventory dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/inventory_dispatch.rs), [player hydration](../../../crates/services/src/cell/service/base_messages/player_init/mod.rs).
**Approve:** quiescence/locks, DB authority, equipment/bandolier reconciliation, outbox ordering and failed-load rollback.
**Required children:** safe reload snapshot (load failure keeps state); reconciliation/swap (locked/trading/equipped/bandolier cases consistent); client resync adapter (exact final snapshot, no duplicate outbox effects, selected-player isolation). **Exclude:** blindly rerunning login hydration over active inventory.

### G12

**Status:** BlockedDesign. **Commands:** `.debug_controller`, `.debug_paths`, `.debug_nav`, `.debug_events`, `.debug_ai`. **Depends:** P01. **Advisors:** npc-ai-spawn-advisor, network-security-auth, server-authority-enforcer.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [spatial path query](../../../crates/services/src/cell/space_manager/spatial.rs), [legacy Misc](../../../deprecated/python/cell/commands/Misc.py), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def).
**Approve:** debug state owner, per-GM subscriptions, lifetime/rate bounds and supported wire display. Confirm client onShowPath support before implementing visualization; escalate ambiguous wire evidence only as needed.
**Required separate children:** controller tick/cleanup (real bounded movement and cancel); path subscription/wire (toggle, exact recipient and disconnect removal); rolling nav query (caller markers, unreachable result); event/chain inspector (truthful Rust model and caller semantics); AI instrumentation subscription (selected mob, bounded output and teardown). **Exclude:** loglevel as AI debug, subscriptionsByEvent invented in Rust, new navmesh engine.

### P33

**Commands:** `.debug_velocity`, `.debug_follow`. **Depends:** P01. **Advisor:** npc-ai-spawn-advisor.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [follow tick](../../../crates/services/src/cell/service/npc_ai/follow.rs), [cell entity](../../../crates/entity/src/cell_entity/mod.rs).
**Scope:** prove velocity consumption in the real tick and preserve legacy cancel-existing-action follow behavior through canonical AI state changes. Split into G12 child if a new controller is required.
**Acceptance:** deterministic tick displacement rather than array-only assertion; idle -> follow and existing action -> cancel, caller disappearance cleanup, actual observer movement. **Exclude:** manual state assignments declared sufficient without tick tests.

### P34

**Commands:** `.aggression`, `.threaten`. **Depends:** P01. **Advisor:** combat-systems-advisor, npc-ai-spawn-advisor.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [damage apply](../../../crates/services/src/cell/abilities/damage_apply/mod.rs), [cell entity](../../../crates/entity/src/cell_entity/mod.rs).
**Scope:** checked aggression and threat-generated orchestration, including threatened_mobs/combat transition invariants; choose the canonical AI/threat operation after one local trace.
**Acceptance:** exact threat values, intended transition and ownership bookkeeping, invalid level/nonfinite threat unchanged, unrelated mobs unaffected. **Exclude:** balance tuning or direct map add as complete parity.

### P35

**Commands:** `.health`, `.focus`, `.speed`. **Depends:** P27 for health lifecycle. **Advisor:** combat-systems-advisor, movement-teleport-advisor.
**Entries:** [native stats](../../../crates/services/src/cell/cell_methods/gm/stats.rs), [stat executor](../../../crates/services/src/cell/content/executor/stats.rs), [legacy Entity](../../../deprecated/python/cell/commands/Entity.py).
**Scope:** shared checked integer current-stat setters; health zero follows lifecycle and speed sets both movementSpeedMod and rotationSpeedMod (100 is normal), publishing dirty stats.
**Acceptance:** bounds/current/max semantics, no mutation on bad input, exact stat messages, health death cleanup and deterministic movement/rotation speed effect. **Exclude:** balance changes, bypassing invulnerability policy once G06 integrates.

## Crafting And Network

### P36

**Commands:** `.appliedscience`, `.racialparadigm`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence.
**Entries:** [crafting handlers](../../../crates/services/src/base/crafting/handlers.rs), [crafting persistence](../../../crates/services/src/base/crafting/persistence.rs), [crafting model](../../../crates/entity/src/crafting.rs).
**Scope:** selected-player crafting progression mutations; reuse ASP persistence and add live sync, validated paradigm set-level on existing representation. Split adapters if no small shared mutation contract fits.
**Acceptance:** exact ASP/paradigm values in DB and live client, relog identical, bad catalog ID/level and failure unchanged, caller not modified. **Exclude:** entire crafting engine or allcraft batch.

### P37

**Commands:** `.learndiscipline`, `.forgetdiscipline`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence.
**Entries:** [console crafting](../../../crates/services/src/cell/console/crafting.rs), [crafting handlers](../../../crates/services/src/base/crafting/handlers.rs), [crafting persistence](../../../crates/services/src/base/crafting/persistence.rs).
**Scope:** real catalog-validated membership add/remove; additive expertise for known discipline and approved bounds. Forget removes known ID/map entry, not merely expertise subtraction.
**Acceptance:** absent/known/invalid catalog cases, exact membership after relog, repeated forget never creates membership, client removal/update correct. **Exclude:** claiming -100 is rejected by the receiver or zero expertise equals forgotten.

### G13

**Status:** BlockedDesign. **Commands:** `.allcraft`. **Depends:** P36, P37. **Advisors:** items-systems-advisor, database-persistence, server-authority-enforcer.
**Entries:** [console crafting](../../../crates/services/src/cell/console/crafting.rs), [crafting handlers](../../../crates/services/src/base/crafting/handlers.rs), [legacy Crafter](../../../deprecated/python/cell/Crafter.py).
**Approve:** four option groups, full blueprint catalog, paradigm 7 and discipline 50 initialization, existing-discipline behavior, batching/atomicity/idempotence and client payload limits.
**Required children:** catalog batch plan (exact relationship-based expected sets); persisted application (rollback/retry, no duplicate grants); client/crafting-options synchronization (exact final state and relog). **Exclude:** unbounded per-item enqueue flood, text-only success, arbitrary hardcoded seed IDs.

### P38

**Commands:** `.net_timer`. **Depends:** P01. **Advisor:** aoi-witness-broadcast.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [SGWBeing.def](../../../entities/defs/interfaces/SGWBeing.def), [dispatch table](../../protocol/client-method-dispatch-table.md).
**Scope:** seek/reuse canonical timer serializer, otherwise exact six-field payload: selected source, secondaryId and game-time plus duration; Type follows signed INT8 definition, caller is recipient.
**Acceptance:** byte-exact 21-byte payload with nonzero source/secondary and controlled clock, optional defaults, invalid/nonfinite duration/type rejected, caller-only delivery. **Exclude:** relative completion time and unsigned-type assumption from stale comments.

### P39

**Commands:** `.net_mapinfo`. **Depends:** P01. **Advisor:** aoi-witness-broadcast.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), [space lifecycle](../../../crates/services/src/cell/space_manager/lifecycle.rs).
**Scope:** actual caller world resource ID and position, selected-player recipient; strict optional bool/sysTypeId parsing and checked field conversion.
**Acceptance:** exact bytes where world ID deliberately differs from instance ID, caller/target recipient split, delete/defaults and malformed options leave no packet. **Exclude:** describing existing serializer as absent or changing global map state.

### P40

**Commands:** `.net_seq`, `.net_seqto`, `.net_seqfrom`, `.net_challenge`. **Depends:** P01. **Advisor:** aoi-witness-broadcast.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [SGWBeing.def](../../../entities/defs/interfaces/SGWBeing.def), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), [dispatch table](../../protocol/client-method-dispatch-table.md).
**Scope:** verify existing debug serializers/directions/default enums with minimal repairs. Split if evidence reveals a new sequence state machine.
**Acceptance:** exact source/destination/broadcaster combinations, caller fallback, default and explicit view values, exact WSTRING challenge bytes and caller recipient. **Exclude:** fabricated slash bindings or interpreting challenge object as integer.

### P41

**Commands:** `.net_speak`. **Depends:** P01. **Advisor:** aoi-witness-broadcast, network-security-auth.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [legacy Net](../../../deprecated/python/cell/commands/Net.py), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def).
**Scope:** intended twelve named channels, selected speaker name, caller-only debug message; repair contradictory Python annotation rather than following it.
**Acceptance:** table-driven exact channel IDs including gaps, names for player/NPC, invalid channel rejected, one caller packet and zero observer packets. **Exclude:** broadcasting debug speech as ordinary chat or broad quote/parser redesign.

### P42

**Commands:** `.net_dialog`. **Depends:** P01; coordinate G02 shared dialog ownership. **Advisor:** mission-systems-advisor.
**Entries:** [console net](../../../crates/services/src/cell/console/net.rs), [legacy SGWPlayer](../../../deprecated/python/cell/SGWPlayer.py), [interaction dialog](../../../crates/services/src/cell/cell_methods/player/interaction/dialog.rs).
**Scope:** trace displayDialog with optional target and retain caller-recipient debug behavior through a typed dialog operation. Unresolved optional-target meaning becomes BlockedEvidence, not an invented default.
**Acceptance:** selected/absent/stale target and invalid dialog fixtures, exact caller dialog bytes, no unintended mission event/target mutation. **Exclude:** implementing privileged interact bypass policy without G02 approval.

### P43

**Commands:** `.net_dhd`, `.net_timeofday`. **Depends:** P01. **Advisor:** movement-teleport-advisor, aoi-witness-broadcast.
**Entries:** [map-loaded](../../../crates/services/src/mercury/world_data/map_loaded.rs), [SGWPlayer.def](../../../entities/defs/SGWPlayer.def), [legacy Net](../../../deprecated/python/cell/commands/Net.py).
**Scope:** display-only DHD with explicit/default world-gate origin and safe missing-target handling; time-of-day sends caller float/float/int packet despite required selected-player registration. Establish whether absent DHD selection rejects or defaults to caller before coding; do not copy Python crash.
**Acceptance:** exact display/time bytes, explicit and world-derived origin, missing gate/target cases, recipient assertions and no gate travel. **Exclude:** native gmDHD dial operation or global weather simulation.

### G14

**Status:** BlockedDesign. **Commands:** `.net_minigame`. **Depends:** P01. **Advisors:** minigame-systems-advisor, server-authority-enforcer.
**Entries:** [minigame dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/minigame.rs), [content executor](../../../crates/services/src/cell/content/executor/mod.rs), [legacy Net](../../../deprecated/python/cell/commands/Net.py).
**Approve:** debug game ID/difficulty/tech validation, session host/lifetime and result feedback, reusing actual StartMinigame pipeline.
**Required children:** typed debug parameter adapter (defaults 1/1, valid ranges and selected recipient); session/result integration (one session, completion/cancel/disconnect cleanup and caller result). **Exclude:** rewriting all minigames or routing into known stub native handlers.

## Privileged Interaction Gate

### G02

**Status:** BlockedDesign. **Commands:** `.interact`, `.initialresponse`, `.adddialog`, `.removedialog`. **Depends:** P01. **Advisors:** mission-systems-advisor, server-authority-enforcer.
**Entries:** [normal interact](../../../crates/services/src/cell/cell_methods/player/interaction/interact.rs), [normal dialog](../../../crates/services/src/cell/cell_methods/player/interaction/dialog.rs), [console entity](../../../crates/services/src/cell/console/entity.rs), [legacy Entity](../../../deprecated/python/cell/commands/Entity.py).
**Approve:** intended privileged aggression/listener bypass versus mandatory authority/invariants, selected target rather than stale last_interaction_target, and per-player dialog mapping lifetime.
**Required children:** add/remove selected-player mapping (catalog validation, real UI/cache use); typed privileged interact (approved bypass and retained guard tests); selected initialresponse (explicit setMapId/target ownership and stale-context rejection). **Exclude:** ordinary dispatch claimed equivalent without policy, or forging invoking player context.

## Scheduling And Closeout

Initial dependency roots: authorize implementation, integrate P01, then choose P02-P07 quick wins or P08 world work. P26 may run early because P18 depends on authoritative snap reuse. G03 can be designed while world packets proceed; mission mutations wait for its approved durability children. G05 transfer waits for P26, not an arbitrary priority milestone. G04 rewards waits for mission foundations and guarded completion. G13 waits for crafting primitives. Treat all design gates as pending approvals even when their prerequisite code is present.

Keep disjoint worktree manifests, single shared-file owner, one integration queue, one Cargo/rustc lane and serialized live-DB tests. Do not allocate P13/P14/P15 or P38/P39/P40/P41 to concurrent writers while they own the same existing file. Design-only review can overlap execution; integration and validation still serialize.

At each README milestone, set affected packets UATPending and pause for the user. Record GM/target/observer/non-GM evidence and relog/DB results; same-world different-instance travel is mandatory, not substituted by a world change. A packet becomes Done only after its automated, documentation, review and applicable UAT gates pass. Record explicit blockers for every remaining command; do not edit the frozen audit to make outstanding work disappear.
