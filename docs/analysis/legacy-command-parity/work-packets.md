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

**Status:** Integrated (2026-09-17, `legacy-command-parity`, merge of `p01-quick-wins-console` @ 540c4f72). Reviewed by testing-validation-engineer (regression-guards independently reproduced, fmt/clippy clean). UATPending — bundled into the M1 milestone pause once P02-P07 land. Worknote/handoff: [worknotes/p01.md](worknotes/p01.md), [handoffs/p01.md](handoffs/p01.md). Known gap carried forward: `.help`'s per-argument detail view (`registry.rs::arg_specs`) is only populated for `help`/`searchitem`/`searchmission`/`searchtemplate`; other commands' `ArgSpec` rows are added by the packet that restores that command. **Commands:** `.help`, `.searchitem`, `.searchmission`, `.searchtemplate`; catalogue/target regression foundation. **Depends:** implementation-session authorization. **Advisor:** server-authority-enforcer.
**Entries:** [query](../../../crates/services/src/cell/console/query.rs), [chat gate](../../../crates/services/src/cell/chat.rs), [console tests](../../../crates/services/src/cell/console/tests.rs), [search sink](../../../crates/services/src/base/console_authoring.rs).
**Scope:** restore argument-level help; pin baseline names/arity/target contracts and protect Rust-only names. Verify search adapters and real result routing; restore literal substring matching while retaining the 25-result bound with explicit truncation feedback. Do not register unimplemented commands merely to satisfy the catalogue test; stage expected implemented subsets as packets integrate, preserving the full backlog denominator. Split search into a bounded child if DB checks expand the foundation packet.
**Acceptance:** exact sorted/filter/help argument output; live-DB one/two-token case-insensitive literal search, including `%`, `_`, backslash, exactly 25 and more than 25 matches, and empty/error responses; GM denial, target validation and caller attribution. **Exclude:** generic commands-crate registry, broad parser redesign, fake parity stubs.

### P02

**Status:** Integrated (2026-09-17, `legacy-command-parity`, merge of `p02-info-facing-combatinfo` @ 9ba50c43). Reviewed by testing-validation-engineer (`.facing` geometry independently verified line-for-line against `SGWSpawnableEntity.py`; regression guards reproduced; fmt/clippy clean). UATPending — bundled into M1 once P03-P07 land. Worknote/handoff: [worknotes/p02.md](worknotes/p02.md), [handoffs/p02.md](handoffs/p02.md). **Known gaps carried forward, not design gates:** (1) `.combatinfo` implements 2 of legacy's 3 checks (no-template, no-ability-set); the weapon-presence check and the `ABILITY_TYPE_*` DD-count bucket have no equivalent concept in the current entity/ability model (no per-NPC-template weapon field, no ability-type taxonomy at all). This is a data-model absence, not an approval question, so it doesn't need a G-group — whichever future packet adds an ability-type taxonomy (if one ever is) should also close this gap in `query.rs::combat_info`. (2) **Update (2026-09-17, P08 fix-up round): the ambiguity below is now resolved — `direction.y` is confirmed to be yaw, always, for both players and NPCs.** `.facing`'s caller-yaw extraction (`query.rs::facing_angle`, reading `caller_dir.x`/`.z` via `atan2` as a Cartesian vector) is a confirmed bug, not merely a candidate one. Proof: the wire packer is unconditional on entity type — `mercury/aoi/{create,update}.rs` both do `pack_angle(direction[1]) // yaw`, `pack_angle(direction[0]) // pitch`, `pack_angle(direction[2]) // roll` — and NPC movement writes the identical convention directly (`cell/service/ticks/npc_movement.rs`: `npc.direction = Vector3::new(0.0, yaw, 0.0)`, comment "pack_angle reads direction.y"). `direction` is never a Cartesian facing vector to run through `atan2`; `.y` **is** the yaw, unconditionally. This fixed two of the four call sites the original note flagged: `console/spawn.rs`'s `heading_of` (now `console/spawn/mod.rs` and `spawn/authoring.rs` post-P08-split) was fixed in P08's own PR fix-up round after Copilot and CodeRabbit independently flagged it — a tautological test (`place_caller` in `console/tests/p08.rs`) had been asserting against the same wrong formula it was testing, which is why the bug shipped past the packet's own regression guards; the fixture and assertions were corrected to assert `direction.y` directly with a distinctive non-zero pitch/roll so a future regression fails instead of coincidentally passing. The other two call sites — `query.rs::facing_angle` (`.facing`/`.combatinfo`, this packet) and `gm/query.rs::handle_show_rotation` (native `.rotation`) — are **not** fixed here; see [P48](work-packets.md#p48), opened to fix both together since they're the same one-line change (`d.x.atan2(d.z)` → `d.y`) in already-shipped, unrelated-to-P08 files. `npc_movement.rs`'s own `atan2` calls (lines ~83/90/136) are unaffected — they compute a *new* yaw from movement-delta components (`dx`/`dz`, not `direction.x`/`.z`) and correctly write the result into `.y`; that's a different, correct calculation, not an instance of this bug. **Commands:** `.info`, `.facing`, `.combatinfo`. **Depends:** P01. **Advisor:** combat-systems-advisor.
**Entries:** [native queries](../../../crates/services/src/cell/cell_methods/gm/query.rs), [console query](../../../crates/services/src/cell/console/query.rs), [entity model](../../../crates/entity/src/cell_entity/mod.rs).
**Scope:** read-only detailed entity/geometry/combat feedback; selection overrides explicit info ID. Reuse actual flags/geometry and legacy template, weapon, ability-set and ability-type diagnostics; label unavailable fields rather than inventing them.
**Acceptance:** selected versus explicit ID precedence, unknown/malformed ID, radians/degrees/class/distance fixtures, missing-template and missing-ability-set cases, caller-only output. Weapon-presence and exact ability-type counts are deferred acceptance criteria — see the `.combatinfo` gap note above; they re-enter this line once an ability-type taxonomy exists. **Exclude:** mutation, damage or AI instrumentation; split if geometry/model work ceases to be read-only.

### P48

**Status:** Ready (opened 2026-09-17, P08 fix-up round). **Commands:** `.facing`, `.combatinfo`, native `.rotation` (bug fix to already-shipped commands, not a new-command packet). **Depends:** P02 (integrated). **Advisor:** movement-teleport-advisor.
**Entries:** [console query](../../../crates/services/src/cell/console/query.rs) (`facing_angle`), [native query](../../../crates/services/src/cell/cell_methods/gm/query.rs) (`handle_show_rotation`).
**Scope:** both functions read a caller/target's own facing via `dir.x.atan2(dir.z)`, treating `CellEntity.direction` as a Cartesian facing vector. It never is one — confirmed by direct wire-format evidence (see the P02 gap note this packet was opened from): `mercury/aoi/{create,update}.rs` pack `direction[1]` as yaw unconditionally for every entity type, and `cell/service/ticks/npc_movement.rs` writes NPC yaw straight into `.y`. Replace both reads with `dir.y` directly, matching the fix already applied to `console/spawn/mod.rs`'s `caller_placement` and `spawn/authoring.rs`'s `.savespawn` heading capture in P08. `query.rs::facing_angle`'s `bearing = dx.atan2(dz)` (computed from *position deltas* between caller and target, not from `direction`) is unrelated and correct — do not touch it.
**Acceptance:** a regression test that gives the fixture a distinctive non-zero pitch/roll (so `atan2(pitch, roll)` and `.y` diverge, the same pattern `console/tests/p08.rs::place_caller` now uses) and asserts the reported heading/facing-class/facing-angle matches `.y` exactly; existing `.facing`/`.combatinfo`/`.rotation` tests must still pass after the fix (their current fixtures may happen to use a `direction` shaped so the old formula coincidentally matched — check before assuming a green re-run means nothing changed). **Exclude:** any other `.facing`/`.combatinfo`/`.rotation` behavior — this packet is a targeted correctness fix, not a re-scope.

### P03

**Status:** Integrated (2026-09-17, `legacy-command-parity`, merge of `p03-stats` @ 931aeedd). The six pre-existing groups already matched legacy field-for-field on inspection against `Entity.py:309-431` — no field changes needed, just regression coverage that didn't exist before. `.stats` added new (health/focus/healthRegen/focusRegen, from `entityStats`, `Entity.py:294-306`). UATPending — bundled into M1 once P04-P07 land. Worknote/handoff: [worknotes/p03.md](worknotes/p03.md), [handoffs/p03.md](handoffs/p03.md). **Note:** "missing-stat behavior" is tested at the formatting-function level (`stats.rs::format_stat_line`), not via a full `CellEntity` fixture — `StatList::new()` unconditionally populates every stat id these seven groups use and exposes no removal API, so no real fixture can produce an absent-stat entity today; a small `StatList::remove` was proposed but not added (outside a `crates/services`-scoped packet's owned paths). **Commands:** `.stats`, `.primarystats`, `.speedstats`, `.armorstats`, `.qrstats`, `.absorbstats`, `.stealthstats`. **Depends:** P01. **Advisor:** combat-systems-advisor.
**Entries:** [console stats](../../../crates/services/src/cell/console/stats.rs), [console tests](../../../crates/services/src/cell/console/tests/mod.rs).
**Scope:** one shared stat-readout contract; add basic four-stat group and verify the six existing exact sets. This is one table-driven behavior, not seven subsystem rewrites.
**Acceptance:** exact names/current/max values for every group, missing-stat behavior (see integration note above), selected-target and caller feedback isolation. **Exclude:** stat setters, balance changes, fabricated default values.

### P04

**Status:** Integrated (2026-09-17, `legacy-command-parity`, merge of `p04-listabilities-players`). `.players` was filtering the already-CellApp-wide `all_player_entity_ids()` down to the caller's own space — bug fixed, not scope creep. `.listabilities` added new, resolving ability ids via the startup-loaded `space_mgr.ability_defs` cache. UATPending — bundled into M1 once P05-P07 land. Worknote/handoff: [worknotes/p04.md](worknotes/p04.md), [handoffs/p04.md](handoffs/p04.md). **Known gap, not a design gate:** legacy's `"In transition"` player state (connected but not yet bound to a space) has no equivalent cell-side today — `SpaceManager` has no broader "known players" registry beyond each space's own `players` set, and base-side connection state isn't visible from the cell service. Every player `all_player_entity_ids()` returns already has a space, so the fallback string is unreachable in practice. This is the "split if online indexing needs new lifecycle state" case this packet's own scope line anticipated — building real in-transition tracking needs new cross-service state, out of scope for a read-only query packet. **Commands:** `.listabilities`, `.players`. **Depends:** P01. **Advisor:** server-authority-enforcer.
**Entries:** [query](../../../crates/services/src/cell/console/query.rs), [ability manager](../../../crates/entity/src/abilities/manager.rs), [ability definitions](../../../crates/services/src/cell/spawner/abilities.rs).
**Scope:** read-only roster feedback: selected abilities with names/fallback; service-local online names/worlds including transitions. Split if online indexing needs new lifecycle state.
**Acceptance:** unknown ability fallback, deterministic output, every loaded space's players (in-transition deferred — see gap note above), no mutation and caller-only results. **Exclude:** cluster/offline roster, ability grants.

### P05

**Status:** Integrated (2026-09-17, `legacy-command-parity`, merge of `p05-givecash-givexp`). First mutating packet in the campaign. Fixed a real caller/subject feedback conflation in the shared sink: `CellToBaseMsg::GrantCash`/`GrantXP`'s `notify_gm: bool` (always fed back to `entity_id`, wrong once caller != target) is now `gm_feedback_to: Option<u32>`. Native `gmGiveCash`/`gmGiveXp` (caller grants to self) and the two non-GM sinks (mob-kill XP, loot pickup) are unchanged in observable behavior — independently verified, not just asserted. UATPending — bundled into M1 once P06/P07 land. Worknote/handoff: [worknotes/p05.md](worknotes/p05.md), [handoffs/p05.md](handoffs/p05.md). **Known follow-on, not a design gate:** `GrantItem`/`RemoveInventoryItem`/`GrantExpertise`/`GrantAppliedSciencePoints` have the identical caller/subject conflation bug, deliberately left untouched here — whichever packet needs it first (P06 `.giveitem` will, immediately) should apply the same `gm_feedback_to: Option<u32>` pattern. **`registry.rs` is now 698/700 lines (hard cap 700) — P06 needs a split before adding any new specs.** **Commands:** `.givecash`, `.givexp`. **Depends:** P01. **Advisor:** database-persistence.
**Entries:** [native give](../../../crates/services/src/cell/cell_methods/gm/give.rs), [progression sink](../../../crates/services/src/base/world_entry/methods/progression/mod.rs).
**Scope:** selected-player typed grants through existing sinks, carrying separate caller feedback identity. Keep current amount bounds; resolve signed input before unsigned conversion.
**Acceptance:** exact cash/XP/level/TP totals after DB reload, distinct caller unaffected, target-only UI, overflow/invalid/no-DB failure truthfulness. **Exclude:** level-setting, training, global economy refactor.

### P06

**Status:** Ready (P01 integrated 2026-09-17). **Commands:** `.giveitem`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence.
**Entries:** [grant-item sink](../../../crates/services/src/base/world_entry/methods/inventory/grant/grant_item.rs), [inventory executor](../../../crates/services/src/cell/content/executor/inventory.rs).
**Scope:** adapt selected-player design-ID/quantity grant using correct container routing and existing transaction/outbox synchronization.
**Acceptance:** merge/new-stack quantities and ownership exactly match after reload; inventory-full, invalid design and DB failure do not report success; target UI/caller feedback split. **Exclude:** name-based lookup, inventory redesign.

### P07

**Status:** Ready (P01 integrated 2026-09-17). **Commands:** `.removeitem`. **Depends:** P01. **Advisor:** items-systems-advisor, database-persistence. **Decision:** D07.
**Entries:** [remove-by-type](../../../crates/services/src/base/world_entry/methods/inventory/core/remove_by_type.rs), [inventory dispatch](../../../crates/services/src/base/world_entry/cell_dispatch/inventory_dispatch.rs), [legacy inventory](../../../deprecated/python/cell/Inventory.py).
**Scope:** atomic exact-quantity removal across matching design stacks, retaining inventory lock/outbox rules. Keep native instance-ID removal separate.
**Acceptance:** multiple stacks with exact remainder; insufficient aggregate, locked/ineligible stock and injected mid-operation failure leave every row unchanged; two-player isolation and exact updates. **Exclude:** treating partial removal as success, changing UseInventoryItem consumption.

## World Authoring

### P08

**Status:** Integrated (2026-09-17, `legacy-command-parity`). `.spawn <templateId>` places one entity at the caller's exact position and facing via the existing `GmSpawnNpc`/`GmSpawnNpcReady` round-trip, whose message gained a `heading` field (was hardcoded to `0.0` — every `.spawn`'d entity faced the same way regardless of caller facing; native `gmSpawnByCmd` still sends `0.0` since its wire signature carries no rotation). `.despawn` destroys the selected NPC through new `SpaceManager::despawn_npc`, which fans `LeftAoI` to every current observer immediately and scrubs witness sets, rather than relying on the next AoI tick to notice (`destroy_entity` alone does not do witness cleanup). **Real bug fixed, not just ported:** legacy registered `.despawn` against `SGWSpawnableEntity`, which `SGWPlayer` derives from, so a literal port would let a GM destroy a logged-in player; corrected to `Target::Mob` at the registry *and* `despawn_npc` independently refuses a player target, so the guard survives a future registry edit (D02). Split `console/spawn.rs` (508 lines) into `spawn/mod.rs` (lifecycle: `.spawn`/`.despawn`/`.respawnall`/`.spawnrandom`) + `spawn/authoring.rs` (persistence: `.savespawn`/`.delspawn`/`.autosavespawn`) — P09/P10/P11 own `authoring.rs`, P12/G01 own `mod.rs`. **Fix-up round (same PR, post Copilot+CodeRabbit review):** (1) the shared `heading_of(dir) = dir.x.atan2(dir.z)` helper was a confirmed bug, not a working conversion — see the corrected P02 gap note above for the wire-format proof; replaced with a direct `direction.y` read at both call sites (`caller_placement` for `.spawn`/`.spawnrandom`, `.savespawn`'s heading capture), and the `console/tests/p08.rs` fixture's own tautological assertion (computed its expected value with the same wrong formula) was corrected too. (2) `.delspawn` now calls `despawn_npc` instead of bare `destroy_entity`, closing the witness-cleanup gap immediately rather than deferring it to P10. (3) `gm_spawn.rs`'s DB-query-failure feedback no longer claims the template is missing when the real cause is a DB error. **Deferred, tracked (CodeRabbit found, not fixed here):** `.respawnall` (`spawn/mod.rs`) moves an NPC via direct `position` field assignment, bypassing `SpaceManager::update_entity_position`'s spatial-grid update — real bug, but CodeRabbit's proposed one-line fix (call `update_entity_position` with the entity's current direction/velocity) would silently corrupt NPC facing: that function's `direction: [i8; 3]` parameter is the player-wire-packet convention (values written straight into `direction` via `as f32`, no unpacking), while NPCs store yaw as radians directly in `direction.y` — passing an NPC's current `Vector3` through as `[i8; 3]` reinterprets radians as small integers. Needs a grid-only position-update primitive that doesn't touch `direction`, not a call to the existing one. Out of P08's scope (pre-existing code, just relocated by the `spawn.rs` split) — whichever packet next touches `.respawnall` should add that primitive. `entities.rs` (590 lines) and `console/tests/p08.rs` (534 lines) are both over the 500-line soft cap with a CodeRabbit-suggested seam (movement-validation methods; `.spawn`/`.despawn` suite split) but under the 700 hard cap — deferred as a non-blocking follow-up, same disposition as P26's `registry/commands.rs` note. Worknote/handoff: [worknotes/p08.md](worknotes/p08.md), [handoffs/p08.md](handoffs/p08.md). **Commands:** `.spawn`, `.despawn`. **Depends:** P01. **Advisor:** npc-ai-spawn-advisor, aoi-witness-broadcast.
**Entries:** [native GM](../../../crates/services/src/cell/cell_methods/gm/mod.rs), [console spawn](../../../crates/services/src/cell/console/spawn/mod.rs), [space entities](../../../crates/services/src/cell/space_manager/entities.rs).
**Scope:** ephemeral lifecycle adapters, validated template, caller placement/heading and safe selected-entity destruction. Trace actual creation result, not only enqueue.
**Acceptance:** one created entity, exact position/heading, wrong template rejected, despawn cleanup and witness removal; no spawnlist mutation. **Exclude:** autosave/persistence and deleting arbitrary players through an NPC-only primitive.

### P09

**Commands:** `.savespawn`. **Depends:** P08. **Advisor:** database-persistence, npc-ai-spawn-advisor.
**Entries:** [console spawn authoring](../../../crates/services/src/cell/console/spawn/authoring.rs), [authoring sink](../../../crates/services/src/base/console_authoring.rs), [seed recording](../../../crates/services/src/cell/console/seed.rs).
**Scope:** typed correlated insert result assigns spawn_id to the same surviving entity; update exact row including world/template/placement/tag. Preserve live SQL/log/buffer semantics.
**Acceptance:** saving twice leaves exactly one row and stable ID; update all fields; failure/despawn-before-result cannot attach ID to another entity; truthful feedback. **Exclude:** generic rowcount as identity, appearance persistence, seed permission redesign.

### P10

**Commands:** `.delspawn`. **Depends:** P09. **Advisor:** database-persistence.
**Entries:** [console spawn authoring](../../../crates/services/src/cell/console/spawn/authoring.rs), [authoring sink](../../../crates/services/src/base/console_authoring.rs).
**Scope:** delete exact persistent row, clear runtime spawn_id only after success, leave entity alive. `.despawn` remains a separate runtime operation.
**Acceptance:** DB row absent and same runtime entity present; failed delete leaves both unchanged; sibling rows untouched; authoring output accurate. **Exclude:** unconditional runtime destruction or broad-key deletes.

### P11

**Commands:** `.autosavespawn`. **Depends:** P08, P09. **Advisor:** npc-ai-spawn-advisor.
**Entries:** [console spawn authoring](../../../crates/services/src/cell/console/spawn/authoring.rs), [space entities](../../../crates/services/src/cell/space_manager/entities.rs), [seed](../../../crates/services/src/cell/console/seed.rs).
**Scope:** consume per-GM preference on successful `.spawn` result for the newly created entity. Do not copy Python's stale-target autosave bug.
**Acceptance:** enabled saves exactly the new entity once, disabled saves none, two-GM isolation, invalid spawn produces no row, disconnect cleans preference. **Exclude:** retroactive saving, implicit random-spawn autosave without an approved contract.

### P12

**Commands:** `.spawnrandom`. **Depends:** P08. **Advisor:** npc-ai-spawn-advisor.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn/mod.rs), [legacy Resource](../../../deprecated/python/cell/commands/Resource.py).
**Scope:** independent uniform XZ offsets inside rectangle, same Y and caller heading, current 1-50 safety bound. Reuse existing RNG conventions and injectable seed for tests.
**Acceptance:** reproducible seeded interior points, exact count/Y/heading, range and malformed argument checks, partial creation failures reported accurately. **Exclude:** deterministic ring, unbounded spawning, new RNG framework.

### G01

**Status:** BlockedDesign. **Commands:** `.respawnall`. **Depends:** P08, G09 resource-cache child. **Advisors:** npc-ai-spawn-advisor, database-persistence, aoi-witness-broadcast.
**Entries:** [console spawn](../../../crates/services/src/cell/console/spawn/mod.rs), [space lifecycle](../../../crates/services/src/cell/space_manager/lifecycle.rs), [legacy Resource](../../../deprecated/python/cell/commands/Resource.py).
**Approve:** snapshot scope, template reload, removal/loading failure semantics and exclusion of players/unrelated instances. Existing field reset is not full parity. **Known bug to fix as part of this packet (P08 fix-up round, 2026-09-17):** the current `.respawnall` moves an NPC via direct `CellEntity.position` assignment, bypassing `SpaceManager::update_entity_position`'s spatial-grid update — `get_entities_in_range`/AoI queries read the grid directly, so a respawned NPC's grid entry goes stale. Needs a grid-only position-update primitive (NOT a call to `update_entity_position` itself — its `direction: [i8; 3]` parameter is the player-wire-packet convention, incompatible with how NPCs store yaw as radians directly in `direction.y`; passing one through the other corrupts facing).
**Required children before dispatch:** template/spawn snapshot staging (failed load preserves old world); runtime cleanup/replacement (death/effects/threat ownership cleaned, unrelated instance unchanged); witness recreation/result reporting (exact disappear/appear sets). **Exclude:** global world reset, partial-field reset presented as full reload.

### P13

**Commands:** `.name`, `.nameid`, `.alignment`, `.faction`. **Depends:** P01. **Advisor:** aoi-witness-broadcast.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [cell entity](../../../crates/entity/src/cell_entity/mod.rs).
**Scope:** validated named mappings and authoritative identity-property publication for selected beings/spawnables as registered. Faction/level fields already exist; do not invent duplicate storage.
**Acceptance:** exact legacy mapping values, target and observers update immediately, unrelated entity unchanged, invalid names fail; feedback makes no false savespawn promise. **Exclude:** adding appearance fields to spawn persistence or character-rename policy changes without approval.

### P14

**Commands:** `.eventset`, `.interactiontype`, `.tag`. **Depends:** P01; serialize with P13/P15 if sharing entity file. **Advisor:** aoi-witness-broadcast, mission-systems-advisor.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs), [console spawn](../../../crates/services/src/cell/console/spawn/mod.rs).
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

**Status:** Ready (P26 integrated 2026-09-17). **Commands:** `.location`, `.rotation`, `.lookat`. **Depends:** P26. **Advisor:** movement-teleport-advisor, aoi-witness-broadcast.
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

**Status:** Integrated (2026-09-17, `legacy-command-parity`, merge of `p26-gotoxyz`). Split `console/registry.rs` (698/700, hard cap) into `registry/mod.rs` (types) + `registry/commands.rs` (the `COMMANDS` array) — the pattern for the next packet that needs registry headroom. `.gotoxyz` reuses the native `gmGotoXYZ`/`gmSummon` mechanism directly; `TeleportPlayer` has no GM-feedback field to conflate (unlike P05's `GrantCash`/`GrantXP`), so no shared-message-struct change was needed. NPC targets get the spatial-grid update without a client push (no client to push to); witnesses still see the move. UATPending — bundled into M1. Worknote/handoff: [worknotes/p26.md](worknotes/p26.md), [handoffs/p26.md](handoffs/p26.md). This packet unblocks [P44](work-packets.md#p44)/[P45](work-packets.md#p45)/[P46](work-packets.md#p46) (G05's children) and [P18](work-packets.md#p18). **Deferred, tracked follow-up (CodeRabbit review, not blocking):** `registry/commands.rs` is at 571/700 lines — under the hard cap, but the `COMMANDS` array already has natural family-comment seams (`── A. entity/content authoring ──` etc.) that CodeRabbit flagged as worth splitting into per-family files now rather than waiting for another hard-cap emergency. Deferred as a heavy-lift refactor touching every future packet's shared registration point; whichever packet next pushes this file back toward 700 should do the family split then, reusing the seams already marked in the file. **Commands:** `.gotoxyz`. **Depends:** P01. **Advisor:** movement-teleport-advisor, aoi-witness-broadcast.
**Entries:** [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs), [teleport receiver](../../../crates/services/src/base/world_entry/teleport.rs), [AoI update](../../../crates/services/src/mercury/aoi/update.rs).
**Scope:** typed selected-or-caller in-space snap with finite coordinates, movement validator/spatial updates, correct player persistence; NPC stays cell-side.
**Acceptance:** exact final positions, forced-position bytes and observer updates; caller unchanged when moving selection; DB failure handled truthfully. **Exclude:** streaming hint alone, cross-space transfer, fake native input buffers.

### G05

**Status:** Approved (D15, 2026-09-17) — children enumerated below, each still individually Ready/BlockedDependency per its own line, not dispatchable as one lump. P44 and P45 are both Integrated; **P46 is now Ready** (both its dependencies are landed). **Commands:** `.goto`, `.summon`, `.gotolocation`. **Depends:** P26. **Advisors:** movement-teleport-advisor, aoi-witness-broadcast, database-persistence.
**Entries:** [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs), [gate-travel](../../../crates/services/src/base/world_entry/gate_travel/mod.rs), [space lifecycle](../../../crates/services/src/cell/space_manager/lifecycle.rs).
**Approved (D15):** `.gotolocation` joins the first/default loaded instance of a named world (no new instance-selector argument). The entity being moved is players-only for any cross-space/cross-world leg of these three commands — NPC targets are rejected with a clear error (matches P26's same-space-only precedent; cross-world transfer needs the player-client world-entry handshake NPCs don't have). Name resolution follows D05 as-is (online, same-service, exact match, join the actual instance) — legacy's own `PlayersByName` lookup (`deprecated/python/cell/commands/Player.py:298-365`, confirmed by direct read) is a flat exact-match dict with no ambiguity handling to design around.
**Children (numbered, dispatch individually in this order):** [P44](work-packets.md#p44) (online name resolution) → [P45](work-packets.md#p45) (typed instance-targeted transfer primitive, reusing `handle_gate_travel`) → [P46](work-packets.md#p46) (the three command adapters). **Exclude:** offline/cluster routing and find-or-create-by-world as proof of joining a named player's instance.

### P44

**Status:** Integrated (2026-09-17, `legacy-command-parity`). `SpaceManager::find_online_player_by_name` — CellApp-wide, case-sensitive, exact-match, four outcomes (`Found { entity_id, space_id }` / `InTransition` / `NotFound` / `Ambiguous`) mapping to legacy's two GM error strings plus a uniqueness-violation guard legacy's dict couldn't produce. Pure read-only addition, no command registered, no `dev-console-channel.md` row needed. `Found.space_id` is the target's *actual* instance — P45/P46 must use it directly rather than re-resolving the world name through the default-instance rule. **Documented gap (tracked, not blocking):** `InTransition` is correct per the data model but not reachable via the case a GM would actually hit — mid-gate-travel, `character_name` is cleared before `InitPlayerState` re-caches it on the freshly created entity, so `.goto`/`.summon` will report `NotFound` (legacy's "not available on this CellApp") in a case where legacy's surviving dict key would have said "not on any reachable space". Closing it needs a name roster that outlives the entity — out of scope for P44/P46; the `InTransition` variant is the splice point for whichever future packet adds it. Worknote/handoff: [worknotes/p44.md](worknotes/p44.md), [handoffs/p44.md](handoffs/p44.md). **Commands:** supports `.goto`/`.summon` (no direct dot command of its own). **Depends:** P26. **Advisor:** movement-teleport-advisor.
**Entries:** [space manager queries](../../../crates/services/src/cell/space_manager/queries.rs) (`all_player_entity_ids`, the same enumeration P04's `.players` fix reuses), [cell entity](../../../crates/entity/src/cell_entity/mod.rs) (`character_name`).
**Scope:** exact-match online-player-name → entity_id lookup, service-wide (every loaded space, matching P04's `.players` CellApp-wide scope, not the caller's space only). Case sensitivity: match legacy's raw Python `in` dict-key check (case-sensitive) unless evidence says otherwise. Not-found and found-but-not-in-a-space ("in transition" — matches P04's documented gap) are the two failure shapes; both must be distinguishable in the returned result so P46's command adapters can produce legacy's exact two error messages.
**Acceptance:** exact match succeeds, near-miss/case-mismatch fails (unless case-insensitivity is confirmed from further legacy evidence), a name with no matching online player fails distinctly from a name matching a player who's mid-transition, deterministic with two+ same-named... (character names are unique — assert the lookup doesn't silently pick one of several if that invariant is ever violated). **Exclude:** offline/cluster lookup, fuzzy/partial matching.

### P45

**Status:** Integrated (2026-09-17, `legacy-command-parity`). The premise that `handle_gate_travel` was a thin-wrapper reuse target did not survive tracing its call graph — it is only the *back half* of a transfer (its own doc comment: "the CellService has already removed the entity from its old space"), so "validate before teardown" is structurally a cell-side responsibility. Implemented as new `cell::space_transfer::transfer_player_to_space` (a free function, not a `SpaceManager` method — `crates/services/src/cell/space_transfer/mod.rs`): strict validate → flush bandolier ammo → checked enqueue → teardown ordering, so a rejected transfer leaves origin state completely untouched. **D15 correction:** `resolve_space_id_fallback` is not a "first/default loaded instance" mechanism as D15 assumed — it's a hardcoded 3-entry table returning Castle_CellBlock for everything else, with a sibling `register_space` that writes a map nothing reads. D15's actual intent is implemented as new `SpaceManager::default_space_for_world` (deterministic: startup space, else lowest loaded instance id). Three real fixes to the shared `handle_gate_travel`/`CreateEntity` path, not just additions: (1) it had no way to join a *specific* loaded instance — `find_or_create_space` always allocated a new space for an instanced world, which would have put a `.goto` target alone in an empty copy of the map; added `destination_space_id` through `GateTravel` → `CreateEntity`, re-validated on arrival; (2) two fail-closed `active_player_id` guards sat after the point of no return (hoisted above the `CreateEntity` round-trip); (3) disconnect mid-round-trip could resurrect a clientless ghost entity, since `CreateEntity`/`DisconnectEntity` share one FIFO — closed with a post-create session re-check. **Two independent reviews (testing-validation-engineer, server-authority-enforcer) found five real must-fix defects, all fixed before merge:** the disconnect-reap in `gate_travel` treated "entity id recycled to a different session" the same as "unmapped" — since ids come off a free list, this could have destroyed a different live player's entity; a forced transfer skipped `cancel_trade_on_disconnect`, stranding the subject's trade partner with a dangling reference to a freed entity id (now called between the confirmed enqueue and teardown, matching the existing lifecycle-arm pattern); the fail-closed abort for a missing `active_player_id` left a live client connected with its entity in no space at all — fixed by ending the session on that path (not just the test) so a reconnect recovers; the primitive's only pre-teardown mutation (the bandolier flush) was unreachable from every test, leaving the packet's own "origin unchanged on rejection" criterion unproven for the one statement that could actually mutate anything; a one-token regression in `is_instanced`'s derivation could have shipped a fresh instanced space the BaseApp never registers with zero NPCs, now covered by an explicit `SpaceData`/NPC-count assertion. Coordinator re-ran the full live-DB suite post-merge (1817 tests, 0 failed) including the one test the worker flagged it couldn't execute locally. Worknote/handoff: [worknotes/p45.md](worknotes/p45.md), [handoffs/p45.md](handoffs/p45.md). **Tracked follow-ups, not blocking:** (1) `world_spaces` can outlive its `spaces` entry (`find_or_create_space`/`default_space_for_world` return a cached id without checking it still exists; `destroy_space` never clears `world_spaces`) — **implemented in this packet**: `create_startup_spaces` (`space_manager/xml.rs`) now skips any world listed with `Instanced="true"`, so the dangling-cache scenario is code-enforced unreachable, not just an XML-convention accident. (2) The same recycled-entity-id hazard exists on the pre-existing `DisconnectEntity` queued during `destroy_client_entities` — not P45's own regression, but the same class. (3) P46 contract notes: match `TransferOutcome` exhaustively (`SameSpace` performs no position move — an `if result.is_ok()` adapter would silently do nothing), and `TransferDestination::in_world` on an instanced world resolves to the *oldest* loaded instance, so P46 should pass the subject's current `space_id` when the destination world matches their own to get the cheap same-space snap instead of a full loading screen. (4) **From CodeRabbit's review of this PR (2026-09-17), deliberately deferred as heavy-lift, not silently dropped:** the `entity_to_addr` snapshot in `gate_travel`'s disconnect-reap (`mod.rs:252`) is read before the `connected` check (`:263`) with no shared lock across both — on a multi-threaded runtime, another thread's concurrent `destroy_client_entities` could theoretically recycle the entity id in that gap, stranding the stale `addr_still_mapped` value and reaping a different live player's entity. The fully correct fix (a session-generation/ownership token checked atomically with the destroy) is real, but the window is a handful of CPU instructions on top of an already-narrow recycling scenario two independent reviews already assessed as low-probability; not attempting a partial fix under review pressure here to avoid introducing a subtler bug in code that's already been through two review rounds. (5) Similarly deferred: `handle_create_entity`'s `Err(e)` arm (`lifecycle.rs:99-113`) drops `reply_tx` with no failure channel, which — absent the (4) mitigation above — would make `handle_gate_travel` fall back to `resolve_space_id_fallback` instead of aborting. Already marked `KNOWN GAP` in the code with the same tracking; CodeRabbit independently found the same shape. The real fix (widening the shared `reply_tx: Sender<u32>` to `Sender<Result<u32, CreateEntityError>>`) touches every entity-creation caller, not just P45's, so it's out of scope here. **Commands:** supports `.goto`/`.summon`/`.gotolocation` (no direct dot command of its own). **Depends:** P26. **Advisor:** movement-teleport-advisor, database-persistence.
**Entries:** [cross-space transfer primitive](../../../crates/services/src/cell/space_transfer/mod.rs) (new), [gate-travel](../../../crates/services/src/base/world_entry/gate_travel/mod.rs), [space manager queries](../../../crates/services/src/cell/space_manager/queries.rs) (`default_space_for_world`).
**Scope:** the shared cross-space/cross-world transfer primitive all three G05 commands call into: validate the destination (world exists, instance resolves) BEFORE tearing down the entity's current space/AoI state, then drive the same teardown → `pending_world_entry` → re-enter flow `handle_gate_travel` already uses for player-initiated stargate travel.
**Acceptance:** same-world distinct-instance test (destination instance ID is exact — `Found.space_id` from P44 is used directly, not re-resolved through the default-instance rule; `.gotolocation`'s own instance selection uses the D15 default via `default_space_for_world`), failure before teardown leaves the entity's origin state completely unchanged (bandolier flush included — see must-fix above), disconnect at each stage of the flow is handled without leaving the entity un-spaced. **Exclude:** building new disconnect-recovery state if `handle_gate_travel` already provides it — reuse, don't duplicate.

### P46

**Commands:** `.goto`, `.summon`, `.gotolocation`. **Depends:** P44, P45. **Advisor:** movement-teleport-advisor, aoi-witness-broadcast.
**Entries:** [console entity](../../../crates/services/src/cell/console/entity.rs) or a new sibling console file, [native travel](../../../crates/services/src/cell/cell_methods/gm/travel.rs) (reference only — native travel is same-service numeric/same-space, not proof of these three commands' named/cross-instance contract).
**Scope:** the three thin command adapters over P44+P45: `.goto <name>` moves caller-or-selected entity to the named player's position/instance; `.summon <name>` moves the named player to the caller-or-selected entity's position/instance; `.gotolocation <worldName> <x> <y> <z>` moves caller-or-selected entity to explicit coordinates in the named world (D15's first/default-instance rule), rejecting an unknown world with legacy's exact wording (`deprecated/python/cell/commands/Player.py:344-365`, confirmed: `"Unable to find world: %s"`). Legacy's own `'Player is not available on this CellApp'` / `'Player is not on any reachable space'` / `'Teleporting to player <%s>'` / `'Summoning player <%s>'` wording (Player.py:298-341) should be matched or deliberately deviated-from with a note, not silently reworded.
**Acceptance:** all three commands' happy paths, not-available/not-reachable/unknown-world failure wording, NPC-target rejection (D15), caller-vs-target precedence matching legacy's `entity = target or player`, exact destination instance ID assertions reusing P45's test fixtures. **Exclude:** any policy already settled by D05/D15 — this packet is wiring, not design.

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

**Commands:** `.health`, `.focus`. **Depends:** P27 for health lifecycle. **Advisor:** combat-systems-advisor.
**Entries:** [native stats](../../../crates/services/src/cell/cell_methods/gm/stats.rs), [stat executor](../../../crates/services/src/cell/content/executor/stats.rs), [legacy Entity](../../../deprecated/python/cell/commands/Entity.py).
**Scope:** shared checked integer current-stat setters; health zero follows lifecycle. `.speed` split out to [P47](work-packets.md#p47) (2026-09-17, coordinator scoping call) — it doesn't touch death/health lifecycle, so it doesn't need P27 and shouldn't wait on it.
**Acceptance:** bounds/current/max semantics, no mutation on bad input, exact stat messages, health death cleanup. **Exclude:** balance changes, bypassing invulnerability policy once G06 integrates.

### P47

**Commands:** `.speed`. **Depends:** P01 only (split from P35 — no death/health lifecycle dependency). **Advisor:** movement-teleport-advisor.
**Entries:** [native stats](../../../crates/services/src/cell/cell_methods/gm/stats.rs), [stat executor](../../../crates/services/src/cell/content/executor/stats.rs), [legacy Entity](../../../deprecated/python/cell/commands/Entity.py).
**Scope:** checked integer current-stat setter for movement; sets both `movementSpeedMod` and `rotationSpeedMod` together (100 is normal per legacy), publishing dirty stats so the client actually sees the change. Legacy `setSpeed` (`deprecated/python/cell/commands/Entity.py:537-548`, confirmed by direct read) sets only `.setCurrent(speed)` on both stats (not max), calls `sendDirtyStats()`, and feeds back `'Set speed of entity %d to %f'` — match or deliberately deviate with a note.
**Acceptance:** bounds/current/max semantics, no mutation on bad input, exact stat message, deterministic movement/rotation speed effect (a real tick-level movement assertion, not just the stat value — matches this ledger's general "prove it in the real tick" standard from P33). **Exclude:** balance changes, bypassing invulnerability policy once G06 integrates.

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
