# Crafting Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-27 (CR-13 close-out). Companions: [launch prompt and decisions](README.md), [audit](audit.md), [session resume](handoffs/session-resume.md), [testing playbook](../../../TESTING.md), [ability-tree ledger](../ability-trees/work-packets.md) (same dispatch rules).

## Dispatch rules

- One worktree per worker, created with `bash tools/build-lane/mk-worktree.sh craft/<packet>-<slug> <name>`, which junctions `external/`. Remove the junction non-recursively (`cmd /c rmdir <worktree>\external`) before removing the worktree.
- Every compiling cargo call goes through `bash tools/build-lane/lane.sh cargo <cmd> -p <crate>`. Never `--workspace`, never `--exclusive` while other sessions work.
- Live-DB tests: `bash tools/build-lane/live-db-test.sh <filter>`, which reloads the worktree's own `sgw_<worktree>` database. Sentinels in `0x7000_Cxxx` (crafting's block; #800 records the old `0x7000_2000` collision, which CR-01 moves).
- After a dependency change: `cargo hakari generate && cargo hakari manage-deps --yes`, and `python tools/crate-graph/crate_graph.py --check`.
- Squash-merge after green CI. When a PR's CI predates the latest `main`, rebase and re-test first.
- Initial state: documentation only, against `main` @ `70795027`.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

`rust-gameserver-dev` is the default writer. Read-only advisors: `items-systems-advisor` (inventory transactions, CR-06 to CR-09), `server-authority-enforcer` (every verb packet, against CAT-F), `database-persistence` (CR-01, CR-04, CR-10), `bigworld-engine-advisor` (CR-02), `aoi-witness-broadcast` (CR-05), `testing-validation-engineer` (CR-06 regression strategy), `documentation-writer` (doc updates each packet owes).

## Contract fixed by this ledger

Parallel packets build against these names. A worker who needs to change one raises it with the coordinator instead of renaming locally.

**Where the logic lives.** Crafting state and inventory are base-owned and persisted. Every verb is one database transaction on the base, locking the `sgw_player` row `FOR UPDATE`. The cell does only what needs space: argument parsing, the station gate and `onUpdateCraftingOptions`.

```text
client ─95..100─► cell: parse args, station gate (CR-05) ─CellToBaseMsg::Crafting(CraftRequest)─► base
base: validate against CraftingCatalog + DB state
   ├─ reject → feedback text (+ onErrorCode where coded) + client state correction      (D-CR14)
   ├─ spend / respec → one transaction → 136 / 137 / property 2 / 139                   (no induction)
   └─ craft / research / reverse engineer / alloy → CraftingSession queue (≤ 10, FIFO)   (D-CR13)
          → onTimerUpdate(type 16, absolute expiry)                                     (D-CR20)
          → at expiry: one transaction re-validates, consumes, grants, gains expertise  (D-CR12)
          → onRemoveItem / onUpdateItem, 136, text line
```

**Rust names** (CR-01 creates them; later packets fill them in):

- `crates/cell-catalog/src/crafting/`: `CraftingCatalog { disciplines, blueprints (with component sets), items: CraftItemAttrs }`, one loader, and the enums `CraftType` (1/2/4/8), `ItemFlags` (all `EItemFlag` bits), `ENTITYFLAG_CRAFT_*`, `TIMER_CRAFT_INDUCTION = 16`, the crafting `CONDITION_FEEDBACK_*` values.
- `crates/wire`: serializers for 112, 137, 138, 139 (moved out of `map_loaded.rs`) and 140, next to the existing 136, each byte-exact tested. `CellToBaseMsg::Crafting(CraftRequest)` with `CraftRequest { entity_id, player_id, verb: CraftVerb, allowed: u8 }`, where `CraftVerb` is `Spend { discipline_id } | Craft { blueprint_id, items, quantity } | Research { item_id, kickers } | ReverseEngineer { item_id } | Alloy { blueprint_id, current_tier_item_id, lower_tier_items } | Respec`, and `allowed` is the `CraftType` mask the station gate granted.
- `crates/cell-methods/.../player/crafting/`: a directory from day one. `mod.rs` holds dispatch and argument parsing; the verbs forward through one `forward.rs`.
- `crates/base-session/src/base/crafting/`: `session/` (the queue, CR-06), `transaction/` (consume and grant, CR-06), `sync/` (client pushes, CR-03), `telemetry.rs` (shared job ids, counters and checked sends), then one file per verb: `spend.rs`, `craft.rs`, `research.rs`, `reverse_engineer.rs`, `alloy.rs`, `respec.rs`, and `feedback.rs` (CR-01) for the rejection path. `handlers.rs` keeps the GM grants.
- Log target `crafting`; events, fields and metrics per the [telemetry contract](#telemetry-contract).

## Telemetry contract

Owner rule D-CR27. It follows `docs/architecture/instrumentation-discipline.md` (the five rules), `docs/architecture/negative-logging-convention.md` and the target catalog in `docs/architecture/observability.md`.

- **Target.** Everything logs under `crafting` (already in `OTEL_FILTER` with its pin). A new target needs its own `OTEL_FILTER` row and pin in the same packet.
- **Identity on every event.** Every player-activity event carries `account_id`, `player_id` and `entity_id` **on the event itself**, not only on a span (rule 5; OTLP log records do not inherit span fields). CR-03/CR-04 (#884) retrofitted CR-01's `request` and `rejected` events with `account_id`; every refusal goes through `feedback::reject(verb, entity_id, player_id, &why, client)` (or `reject_at_completion` for a refusal raised when a queued job ends, which counts the rejection but not a second request).
- **Spans.** One INFO span per dispatch entrypoint (`crafting.request` on the base, with `verb`); none inside per-tick work such as the induction queue tick (rules 1 and 3).
- **Events.** A DEBUG or INFO event with `event = "…"` on every state transition (rule 2):
  - `request` (the verb and every parsed argument);
  - `rejected` (INFO, with an enumerated `reason`, the same value as the `CraftReject` variant);
  - `queued` / `induction_started` / `induction_expired` (`job_id`, `verb`, `queue_len`; `induction_started` also carries `timer_id` and `expires_at`, the only one of the three that has a deadline);
  - `completed` (`job_id`, `verb`, `blueprint_id` or `item_id`, the consumed inputs as `item_id:type_id:qty_before→qty_after`, the granted outputs as `type_id:bag:slot:qty_before→qty_after` so a stack merge is distinguishable from a new slot, a `result` for verbs that roll (`success | failure`), `expertise_before` / `expertise_after`, `asp_before` / `asp_after`, the RNG roll and chance where a roll was made);
  - `queue_dropped` (`reason = logout | world_change | session_changed | stale_session | not_connected | respec`, `cause`, `jobs_dropped`, `job_ids`; `respec` once CR-10 merges), and `induction_start_skipped` (DEBUG: a job dropped between activation and its bar);
  - `options_changed` (the station entity ids and tool item ids per section);
  - `learned`, `respec_prompted`, `respec`, `paradigm_raised`, `blueprint_learned` (before and after values);
  - `login_sync` (what the login sent) and `login_sync_failed` (WARN, `reason = load | send`), `asp_granted`, `gm_allcraft`, `gm_craftkit` and `gm_learnblueprint` (GM grants and their refusals, before and after), `asp_earned` (level-up grant: `level_before` / `level_after`, `asp_before` / `asp_after`), `tool_table_loaded` (INFO, the Field Crafting Tool table read once per process);
  - already on `main` from CR-01: `malformed`, `no_player`, `forward_failed` (WARN, cell side), `catalog_loaded`, `catalog_load_failed`, and the catalog's data-quality WARNs `catalog_orphan_component` and `catalog_unknown_quality`;
  - negative seams: `persist_failed` (WARN, `phase`, `reason`), `client_sync_failed` (WARN; DEBUG once the session has ended), `push_failed` (WARN, `what`), `lookup_failed` (WARN, `phase`), `feedback_send_failed` (WARN), `allcraft_send_failed` (WARN, cell side: the `.allcraft` forward to the base failed).

  This is the complete list of events under the `crafting` target. A packet that needs another adds it here, in the same PR, before using it. CR-16's grant-path events (`grant_container_chosen`, `loot_restored`) belong to the inventory grant path's own target, not `crafting`.
- **Negative seams.** Every expectation seam logs its failure at the level the convention sets, and has a `LogCapture` test (TESTING.md type 12): a transaction with `rows_affected == 0`, a catalog or inventory lookup miss, a failed client send (`let _ = send` is not allowed), a rollback (`persist_failed` WARN with `phase` and the SQL error class). A DB write that changes fewer rows than it should logs the paired `rows_affected` and `expected` fields, and names its sub-step `phase`, as the convention requires.
- **Metrics.** Enumerated labels only (rule 4), each with one emission point so a request is never counted twice:
  - `crafting_requests_total{verb, outcome}`, emitted once per request when it is answered: `outcome` in `accepted | rejected`;
  - `crafting_jobs_total{verb, outcome}`, emitted once per induction job when it ends: `outcome` in `completed | failed | dropped`;
  - `crafting_rejections_total{verb, reason}`.

  No ids in labels.
- **Catalog.** This section is the plan; the canonical list is the `crafting` row of `docs/architecture/observability.md`. The packet that first emits an event or a metric adds it to that row in the same PR.
- **Acceptance.** Each packet's tests include at least one `LogCapture` assertion per new rejection reason and per new transition event, one that the events carry `account_id`, `player_id` and `entity_id`, and a counter-emission assertion for each new metric.

## Dependency graph and waves

```text
Wave 0 (now, parallel)        Wave 1 (after CR-01, parallel)          Wave 2 (parallel where marked)               Wave 3
CR-01 foundation ─────┬──► CR-03 login sync + paradigm defaults ──┐
CR-E1 client evidence │    CR-04 spend ASP ───────────────────────┼──► CR-07 craft ────────────────┐
CR-E2 cooked items ───┤    CR-05 stations + tools + options + GM ─┤    CR-08 research + rev-eng ────┤
CR-02 game clock ─────┼──► CR-06 induction engine + transaction ──┘    CR-09 alloy ─────────────────┼──► CR-13 close-out ─► CR-14 owner UAT
                      │                                                CR-10 respec ────────────────┤     + /release
                      │                                                CR-11 debug hub (#846) ──────┤
                      │                                                CR-12 ASP earning ───────────┤
                      ├──────────────────────────────────────────────► CR-15 blueprint/guide items ─┤
  Bank BV-01 (#872) ──┴──────────────────────────────────────────────► CR-16 grant fall-through ─────┘
```

CR-01 is the only bottleneck. It is kept small: catalog, constants, serializers, argument parsing and the message shape, with **no behaviour change**. CR-E1, CR-E2 and CR-02 touch no file CR-01 owns.

**Contended files.** The coordinator merges these one packet at a time:

- `crates/wire/src/mercury/world_data/map_loaded.rs`: CR-01 (moves the 139 builder out), then CR-03.
- `crates/cell-methods/.../player/crafting/mod.rs`: CR-01, then CR-05 (gate), then CR-10 (respec arm). Verb packets add files only.
- `crates/base-session/src/base/crafting/mod.rs` and the base dispatch arm: CR-01 adds the `Crafting` arm; later packets add files and `mod` lines only.
- `base/world_entry/methods/progression/mod.rs` (`grant_xp`): CR-12 only.
- `crates/wire/src/cell/messages/cell_to_base.rs` is **over the 700-line cap** (793 lines after CR-01). An enum's variants cannot move to another file, and the only natural seam (the contact-list family) would rename about 40 call sites while two other campaigns add variants. So crafting adds tuple variants whose payloads live in `crates/wire/src/crafting/`, and the split is a separate PR once the parallel campaigns settle.
- `crates/entity/src/cell_entity/entity_struct.rs` is **over the 700-line cap**. CR-05's per-player station state goes in a new file, not in that struct's body.
- `db/resources/Entities/Seed/entity_templates.sql`, `Worlds/Seed/spawnlist.sql`: CR-11 only, inside 310-329 and 410-429. Message the guilds session (cimmeria-fa) and the pets session (cimmeria-b5) before merging.
- `crates/wire/src/containers.rs`, `crates/entity/src/inventory.rs` (`BAG_SIZES`), `inventory/move_/mod.rs` and `inventory/grant/validation.rs`: owned by the Bank/Vault campaign (cimmeria-97, BV-01 lands first; it keeps container 15 movable) and the guilds vault packet (cimmeria-fa, containers 19 and 20). CR-05 may add a post-commit bag-15 notification in `move_/mod.rs`; message both sessions before that packet starts and rebase onto their changes.

## Common acceptance

- Every behaviour change ships a regression guard that **fails when the fix is reverted** ([TESTING.md](../../../TESTING.md)), proven once and recorded in the worknote.
- Each rejection reason gets a test that asserts the feedback is sent (D-CR14), not only that nothing happened.
- Consumption and grants get live-DB tests with exact-sentinel cleanup, including one that proves a mid-induction logout consumes nothing (D-CR12).
- Wire output gets byte-exact tests. Telemetry per the [telemetry contract](#telemetry-contract), with its `LogCapture` tests.
- Randomness goes through an injectable RNG, so tests pin outcomes (CAT-F F-04: outcomes are server-side only).
- Each packet updates the docs it owes: `docs/gameplay/crafting-system.md`, `docs/gap-analysis.md` §19, `docs/protocol/` when a wire message changes, and the crafting findings when evidence shifts.
- Worknote at `docs/analysis/crafting/worknotes/<packet>.md` (CRLF). Workers do not edit this file or the README; the coordinator owns the ledger.

## Wave 0

### CR-01

**Status:** Integrated (#862). **Scope title:** Catalog, constants, serializers, argument parsing, request message. **Depends:** none. **Advisor:** database-persistence, testing-validation-engineer.

**Scope:**

- `CraftingCatalog` and its loader (the contract above), loaded once at cell startup and shared with the base the way `AbilityTreeCatalog` is. Component sets are grouped by `component_set_id`. Item attributes: `flags`, `tier`, `quality_id`, `tech_comp`, `discipline_ids`, `applied_science_id`.
- The enums and constants in the contract, with a test that pins each value to `entities/defs/enumerations.xml`.
- Serializers for 112, 137, 138, 139 and 140 in `crates/wire`, byte-exact tested; `map_loaded.rs` calls the new 139 serializer. Record the `CraftingOptions` layout in `docs/protocol/`.
- Convert `player/crafting.rs` into `player/crafting/`, parse every argument of 95-100 per `SGWPlayer.def:894-949` (including the `ARRAY<ItemID>`s), and forward a `CellToBaseMsg::Crafting(CraftRequest)` with `allowed = 0`. The base arm logs the request at `crafting` and sends the D-CR14 "not available yet" text line, so a press is never silent.
- `base/crafting/feedback.rs`: `reject(entity_id, reason)` sends the text line and, where a code is mapped, `onErrorCode`. One enum of reasons; later packets add variants.
- Move the crafting live-DB sentinels to `0x7000_Cxxx` (#800).

**Acceptance:** catalog loaded from a fresh `db/database.sql` in a live-DB test with the counts in audit C-20 to C-23 (78 / 498 / 2,556, blueprint 21 has no components); round-trip parse tests for every verb's arguments; the forward carries every argument.

### CR-E1

**Status:** Integrated (#858). **Scope title:** Client evidence for the crafting UI. **Depends:** none. **Writer:** game-archaeology-specialist (Ghidra, read-only on the client; the client Lua under `..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI`). Documentation only, plus Ghidra comment fixes.

**Questions, each answered with an address or file:line:**

1. Respec: what `Event_SlashCmd_RespecCraft` (`0x018417f0`) sends, and whether a full respec sends method 100 once or twice (feeds D-CR16).
2. `CraftingOptions`: confirm the nested array encoding from the unpacker (`0x00e49180` → `0x00e47250`), and whether the client itself checks distance to the machine entity or trusts the server.
3. Which feedback reaches the player: does `onErrorCode` with `ERRORCODE_SYSTEM_Ability` and codes 213/214 print text? Where does `Mercury__unknown_00ceae50` write (log or chat)? Which server text path shows in chat?
4. The client clock `FUN_00c6e220`: which message sets it (`SET_GAME_TIME`, `TICK_SYNC`, `UPDATE_FREQUENCY_NOTIFICATION`) and in what unit (feeds CR-02).
5. Alloy: confirm the `0x00e46990` counts, stack-quantity counting, and what the client sends as `aCurrentTierItemId` and `aLowerTierItems` (C-38).
6. Research kickers: confirm the one-per-science rule and whether the kicker's "+1 Expertise" text maps to any client-side number (C-39).

**Output:** `docs/reverse-engineering/findings/crafting-client-ui.md`, indexed in the findings README; corrections for audit C-61 and C-62 in the existing crafting findings and in the Ghidra PRE_COMMENT at `0x00e465d0`; a short `worknotes/cr-e1.md` that feeds D-CR16 and D-CR20.

### CR-E2

**Status:** Integrated (#868). **Scope title:** Blueprint items, Paradigm Guide items and Field Crafting Tools in the client's cooked data. **Depends:** none. **Writer:** game-archaeology-specialist (cooked PAKs through `crates/resources`, Ghidra read-only). Documentation plus a proposed mapping file; no Rust.

**Questions, each answered with evidence:**

1. How does the client link a "Blueprint: …" item (289 in the seed; texts `DN_It_Cft_Blueprint_*`) to a blueprint id? Look for a field in the cooked item or blueprint records (`CookedItems`, `CookedBlueprints`), then fall back to name matching. Produce the full item → blueprint mapping with its method and the unmatched rows.
2. Do the "Racial Paradigm Guide: <paradigm>" items (texts 28224-28234) exist in the client's cooked item data? If so, their item ids and the field that names the paradigm. If not, what a seed-only item needs for the client to render it.
3. Field Crafting Tools (5369, 8402-8466): does cooked data carry an applied-science or tool-type field, or only the name prefix?
4. The use path: what the client sends when the player uses such an item, and whether the client refuses to send it for an item type it considers unusable.

**Output:** `docs/reverse-engineering/findings/crafting-items.md`, indexed in the findings README; the mapping as `docs/analysis/crafting/source/blueprint-items.csv` (item id, blueprint id, method); `worknotes/cr-e2.md` feeding D-CR21 and D-CR22.

### CR-02

**Status:** Integrated (#864). **Scope title:** One consistent game clock for timer expiries. **Depends:** none. **Advisor:** bigworld-engine-advisor, combat-systems-advisor.

**Scope:**

- Make `SET_GAME_TIME`, `TICK_SYNC` and the ongoing tick sync agree on one clock (audit C-13): the declared tick rate matches the rate the counter actually advances, and login sends the current time, not 0.
- One `game_time_secs()` that every `onTimerUpdate` sender can use for `BigWorldTimeComplete`.
- Use it for the ability warmup and cooldown timers that send `0.0` today only if CR-E1 Q4 confirms the domain; otherwise leave them and record why.

**Acceptance:** a unit test that the declared tick rate and the advance per loop agree; a test that a timer built with `game_time_secs() + d` decodes to `now + d` on the same clock. CR-14 checks the bar in game. If the evidence says the client's clock cannot be matched without a client patch, stop and report; D-CR20's fallback then stands.

## Wave 1 (after CR-01 merges)

### CR-03

**Status:** Integrated (#884, with CR-04). **Scope title:** Login sync, ASP display and paradigm defaults. **Advisor:** aoi-witness-broadcast.

**Scope:**

- Call `load_crafting_state` at player load. Send, owner-only: 136 per known discipline, 138 per paradigm, 139, 140 (empty unless CR-05's gate says otherwise), and the ASP property.
- Every ASP change (GM grant, spend, respec, earning) pushes `onEntityProperty(2, total)`, the **total** (audit C-06, C-57). Fix the doc comment (C-63).
- Paradigm defaults per D-CR03 (Common at 5, the other four at 1), applied when a character has no stored levels, so existing characters are covered without a migration. The seed's column default and the character-creation path give new characters the same values.

**Telemetry:** `event=login_sync` (INFO) with the counts sent (`disciplines`, `paradigms`, `blueprints`, `asp`) and `defaults_applied` (bool); a failed load or send is `event=login_sync_failed` (WARN) with `reason` in `load | send` and the error class. The GM ASP grant logs `event=asp_granted` with `asp_before` / `asp_after`.

**Acceptance:** a byte-exact test of the login crafting bundle for a fixture state; a live-DB test that a relog restores disciplines, expertise, paradigms and blueprints; a guard that the GM ASP grant pushes the property.

### CR-04

**Status:** Integrated (#884, with CR-03). **Scope title:** `spendAppliedSciencePoints` (95). **Advisor:** server-authority-enforcer, database-persistence.

**Scope:**

- `base/crafting/spend.rs`: one transaction that checks ASP ≥ 1, the discipline exists, it is not already known, the paradigm level, and every prerequisite known at expertise ≥ 50; learns it at expertise 1 and spends 1 ASP. It grants no blueprints (D-CR04). It is replay-safe: a second identical request finds the discipline known and changes nothing (CAT-F F-02).
- Then 136 and the ASP property.
- Every rejection sends feedback: 214 `NotEnoughAppliedSciencePoints` where it fits, text otherwise (D-CR14).

**Telemetry:** `event=learned` with `discipline_id`, `expertise_before` (0 on a first learn) / `expertise_after`, `asp_before` / `asp_after`; `event=rejected` with `reason` in `no_asp | unknown_discipline | already_known | paradigm_too_low | prerequisite_missing | prerequisite_expertise` plus the values compared (for example `paradigm_level` and `required_level`).

**Acceptance:** a live-DB test per rejection reason and for the success path; a replay test; a guard that fails when the prerequisite check is removed.

### CR-05

**Status:** Integrated (#895). **Scope title:** Stations, tools, crafting options and "craft anywhere". **Advisor:** aoi-witness-broadcast, server-authority-enforcer, items-systems-advisor.

**Scope:**

- **Stations (cell).** A station is any entity whose template `entity_flags` carries `ENTITYFLAG_Craft_*` bits. For each player the cell tracks the nearest station per verb within interaction range (`MAX_INTERACT_DISTANCE`), and reports changes to the base (`CellToBaseMsg::CraftingStations`): entering or leaving range, the station despawning, a world change. The per-player state lives in a new file (the entity struct is over its cap). `CraftRequest.allowed` carries the station mask at request time.
- **Tools (base).** Per D-CR21: a Field Crafting Tool in bag 15 enables craft, research and reverse engineering for its science up to its `tech_comp`. The base re-evaluates when bag 15 changes.
- **Options (base).** The base owns `onUpdateCraftingOptions` (140): per section, the station entity id and the tool item id. It sends it when either input changes, and at login (with CR-03).
- **Gate (base).** A verb is allowed by the station mask, or by a tool whose science and `tech_comp` cover the blueprint's discipline (for research and reverse engineering, one of the item's disciplines). Otherwise it is rejected with the text "No crafting station or tool for …" (D-CR14).
- `.allcraft` and "craft anywhere" per D-CR17, GM-gated.
- Spawning a flagged entity needs no new AoI path. If it does, message cimmeria-b5 first.

**Telemetry:** `event=options_changed` with the station entity ids and tool item ids per section and the cause (`moved | station_despawned | world_change | bag15_changed | login | gm_anywhere`); the gate's refusal as `event=rejected reason=no_station_or_tool` with the verb, the station mask and the tools considered. `.allcraft` logs `event=gm_allcraft` with the before and after counts (disciplines, blueprints, paradigm levels). No per-tick events for the station scan.

**Acceptance:** unit tests of the station set against positions and of the tool rule (science, `tech_comp`, bag 15 only); a byte-exact 140 test; a guard that a forged request with no station or tool is rejected; a test that "craft anywhere" is refused for a non-GM.

### CR-06

**Status:** Integrated (#897). **Scope title:** Induction engine and the consume-and-grant transaction. **Advisor:** items-systems-advisor, testing-validation-engineer, server-authority-enforcer.

**Scope:**

- `session.rs`: a per-player `CraftingSession` with one active induction and a FIFO queue of at most 10 (D-CR13). Starting an induction sends `onTimerUpdate(type 16, SourceID = entity, TotalTime = 3.0, BigWorldTimeComplete = game_time_secs() + 3.0)` (D-CR24). Dropped on logout or world change without consuming. A test clock drives expiry.
- `transaction.rs`: one transaction that locks the player and the named items, checks that each is owned and sits in bag 1 or 15, consumes by design across bags 1 and 15 (C-31), grants outputs by stack merge or free slot, and adjusts expertise. After commit: `onRemoveItem` for drained stacks, `onUpdateItem` for changed ones, and a full resync as the fallback. A failure rolls back and sends feedback.
- An injectable RNG for the verbs that roll.
- No verb uses it yet; CR-07 to CR-09 plug in.

**Telemetry:** `queued`, `induction_started`, `induction_expired`, `completed` and `queue_dropped` per the contract, with a `job_id` that correlates them; `persist_failed` (WARN) names the rollback `phase`; the full consumed and granted item lists with before and after quantities, so an item question is answerable without the database.

**Acceptance:** live-DB tests for a partial stack, a drained stack, a full inventory (rollback plus feedback), an item moved to the bank mid-induction (rollback), and a logout mid-induction (nothing consumed); a queue test for the eleventh request.

## Wave 2

### CR-07

**Status:** Integrated (#905). **Scope title:** `craft` (96). **Advisor:** items-systems-advisor, server-authority-enforcer.

**Scope:** `craft.rs`. Blueprint known, discipline known (D-CR15), not an alloy, `quantity` in 1..=100, the submitted ids pick the component set whose component types they cover, and the bags hold `quantity × component.quantity` of each. At completion: consume, grant `blueprint.quantity × quantity` of the product, +1 expertise, send 136. Blueprint 21 (no components) is rejected.

**Telemetry:** `completed` carries `blueprint_id`, the component set chosen, `quantity`, inputs and outputs; each rejection has its own `reason` (`unknown_blueprint`, `discipline_unknown`, `is_alloy`, `bad_quantity`, `no_component_set`, `insufficient_components`).

**Acceptance:** live-DB end to end with blueprint 412 set 1 (14× 5254) and set 2; a multi-stack requirement (C-31); each rejection with feedback; a guard for the discipline-known check.

### CR-08

**Status:** Integrated (#903). **Scope title:** `research` (97) and `reverseEngineer` (98).

**Scope:**

- `research.rs`: item researchable (`Craft_Research`), kickers flagged `Kicker`, at most one per applied science and none from the item's own (D-CR11). At completion: consume the item and kickers, roll per D-CR15, +5 expertise on success, and a text line either way. A success also teaches the blueprint that makes the researched item, when that blueprint's discipline is known (D-CR04), and sends 139.
- `reverse_engineer.rs`: item reverse-engineerable (`Craft_RevEng`) and produced by at least one blueprint. At completion: consume exactly the named instance (CR-06's transaction checks a named instance's owner and bag but consumes by design, so this packet adds an instance-exact consume step to `transaction/consume.rs`), pick a blueprint and a component set uniformly (no C-50), recover per D-CR06, grant. Up to 10 queued (C-34).

**Telemetry:** both verbs use the contract's `queued` → `induction_started` → `completed` chain and `rejected` for refusals. Research's `completed` adds `eligible_disciplines`, `discipline_id`, `chance`, `roll`, `result`, `expertise_before` / `expertise_after` and any `blueprint_learned` it triggers (also emitted as its own event). Reverse engineering's `completed` adds the chosen `blueprint_id` and `component_set_id`, `bias`, and each component's roll with its recovered quantity. Refusal reasons: `not_researchable`, `not_kicker`, `kicker_same_science`, `kicker_duplicate_science`, `not_reverse_engineerable`, `no_blueprint_for_item`.

**Acceptance:** seeded-RNG tests for success and failure, and for the blueprint taught on success; a burst of 10 reverse-engineer requests completes 10 times; guards for the kicker rules and the zero-expertise case.

### CR-09

**Status:** Integrated (#904). **Scope title:** `alloying` (99). **Advisor:** items-systems-advisor.

**Scope:** `alloy.rs`. An alloy blueprint known with its discipline; the current-tier item is the blueprint's component (C-53 and C-54 fixed); the elementary items are exactly one tier lower, and their summed stack quantity meets the client's count for one quality (Normal 10, Good 5, Great 2, Fantastic 1; D-CR11). At completion: consume, grant 2× product, +1 expertise.

**Telemetry:** `completed` carries the current-tier item, the elementary items with quality and tier, and the quality bucket met; rejections name the rule broken (`wrong_tier`, `count_not_met`, `multiple_buckets`, `not_alloy`).

**Acceptance:** live-DB end to end with blueprint 42; one test per count rule; a guard for the tier rule.

### CR-10

**Status:** Review (branch `craft/cr10-respec`, rebased on `main` after CR-16; PR not yet opened). **Scope title:** `respecCrafting` (100, 112, 137). **Advisor:** server-authority-enforcer, database-persistence.

**Scope:** `respec.rs`, per D-CR16 and D-CR23: a player-usable `.respeccraft` sends the prompt (cost 0, D-CR02), the pending window, then one transaction that clears disciplines and expertise and refunds one ASP per learned discipline. Blueprints and paradigm levels are kept. Then 137 and the ASP property. Nothing to reset gets feedback. Replay-safe.

**Telemetry:** `event=respec_prompted` and `event=respec` with each cleared discipline as `discipline_id:expertise_before→0`, `asp_before` / `asp_after`, and the kept blueprint and paradigm counts; a confirm with nothing pending is `rejected reason=no_pending_respec`.

**Acceptance:** live-DB tests for the refund, the clear, and the kept blueprints and paradigms; a guard that a single send never wipes; a replay test.

### CR-11

**Status:** Integrated (#909). **Scope title:** Crafting stations and supplies in the stasis-room debug hub. **Advisor:** items-systems-advisor.

**Scope:**

- Templates 310-313: the four "<Science> Crafting Station" entities, named from the existing texts (audit C-25), with all four `ENTITYFLAG_Craft_*` bits. Spawns 410-413 in the stasis room, placed per `docs/content/debug-hub.md` (read its placement warning and the hub worker's authoring traps in `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`).
- Template 314 / spawn 414: a "crafting supplies" vendor, at 1 naquadah each: the components of the UAT recipes (audit §2), the four kickers, one Field Crafting Tool per science (the -5 and -50 grades), the Paradigm Guides (7805-7809), and Blueprint item 6483 (teaches blueprint 25).
- `.craftkit <blueprint> [count]` and `.learnblueprint <id>` (D-CR17).
- Add the stations to `docs/content/debug-hub.md`.

**Telemetry:** the seed guard is the test; the stations and vendor need no new events beyond CR-05's `options_changed`. `.craftkit` logs `event=gm_craftkit` with `blueprint_id`, `count`, and each granted item as `type_id:bag:slot:qty_before→qty_after`.

**Acceptance:** a live-DB seed guard that the templates carry the flags and the spawns sit in world 12; the vendor list resolves; a `.craftkit` test.

### CR-12

**Status:** Integrated (#900). **Scope title:** Earning ASP. **Advisor:** combat-systems-advisor (the `grant_xp` path).

**Scope:** per D-CR01: `grant_xp` adds the levels gained to `applied_science_points` in the same statement that raises the level, and pushes the property; new characters start with 1.

**Telemetry:** `grant_xp` emits `event=asp_earned` (target `crafting`) with the full identity (`account_id`, `player_id`, `entity_id`), `level_before` / `level_after` and `asp_before` / `asp_after`.

**Acceptance:** a live-DB test that a level-up grants ASP atomically with the level; a guard that fails when the grant is removed.

### CR-15

**Status:** Integrated (#902). **Scope title:** Blueprint items and Racial Paradigm Guides. **Advisor:** items-systems-advisor, server-authority-enforcer, database-persistence.

**Scope:**

- A seed table (for example `resources.crafting_item_effects (item_id, blueprint_id, racial_paradigm_id)`) filled from CR-E2's mapping per D-CR26 (193 Blueprint items, 8882 teaching two blueprints), plus the five existing Guide items 7805-7809. A generator under `tools/crafting/` builds the seed rows from `source/blueprint-items.csv`, so the mapping has one source.
- The item-use path recognises these items. A Blueprint item teaches its blueprint (139). A Guide raises its paradigm by 1, to at most 10 (138). Either way the item is consumed in the same transaction that changes the crafting state. A known blueprint or a guide at 10 is refused with feedback and consumes nothing.
- Loot: add Guides and Blueprint items to a loot table only where existing content already places crafting drops; otherwise the vendor (CR-11) is the only source for now.

**Telemetry:** `event=blueprint_learned` (`item_id`, and per blueprint `blueprint_id:known_before→known_after`, plus the known-blueprint count before and after) and `event=paradigm_raised` (`paradigm_id`, `level_before` / `level_after`); refusals `already_known` and `paradigm_max` as `rejected`.

**Acceptance:** live-DB tests for each item kind, including the refused cases (nothing consumed); a replay test; a seed guard that every mapped blueprint and paradigm id exists.

### CR-16

**Status:** Integrated (#932). **Scope title:** Grant into the first allowed player container. **Advisor:** items-systems-advisor. Tell cimmeria-97 when it starts.

**Scope:** audit C-29. When an item's `container_sets` lists a storage container (17-20) first, the grant path falls through to the next listed player container (15, then 1) instead of refusing. The loot-into-bank refusal BV-01 adds stays for items that list only storage containers. Every caller benefits: loot, content `grant_item`, GM `gmGiveItem`, vendors, and CR-06's transaction if it reuses the grant path.

Also close the loot data-loss path: `cell-interactions/.../loot.rs:185` removes the item from the corpse on the cell **before** the base accepts the grant, so a refused grant destroys the item. Remove from the corpse only after the base confirms, or restore it on refusal.

**Telemetry:** under the grant path's existing target, `event=grant_container_chosen` with the full identity, `type_id`, `container_sets`, `skipped_storage`, the chosen `container_id` and slot, and the stack quantity before and after; `event=loot_restored` when a refused grant puts the item back on the corpse (`corpse_id`, `type_id`, `qty`). Add both to that target's row in `docs/architecture/observability.md`.

**Acceptance:** a live-DB test that a `{17,15}` component granted by loot, by the content engine and by a GM lands in bag 15; a test that a refused loot grant leaves the item on the corpse; a guard that fails when the fall-through is removed; the BV-01 refusal test still passes for a storage-only item.

### CR-17

**Status:** Integrated (#953). **Scope title:** Trade from the crafting bag (D-CR28). **Advisor:** server-authority-enforcer, items-systems-advisor.

**Scope:** trade accepts offered items from the main bag (1) and the crafting bag (15) and places each received item in the recipient's bag by its `container_sets`; a full crafting bag cancels the trade for both players with a reason line and `onTradeResults(Cancelled)`, so the window closes. The trade swap takes the shared inventory lock order, which also fixed a vendor buyback and sell deadlock. Mail from bag 15 went to the social-systems campaign (#933).

**Telemetry:** `trade.item_moved` carries `container_before` / `container_after`; `trade.refused reason=crafting_bag_full`.

**Acceptance:** live-DB tests for a component traded from bag 15 into the partner's bag 15, a full crafting bag refused with nothing moved, and the buyback and sell lock-order guards (see `worknotes/cr-17.md`).

## Wave 3

### CR-13

**Status:** Writing (branch `craft/cr13-close-out`; merges after CR-10). **Scope title:** Close-out, UAT checklist and release.

**Scope:** `docs/gameplay/crafting-system.md`, `docs/gap-analysis.md` §19, `docs/project-status.md`, the crafting findings (C-60), CAT-F paths (C-64); close or update #567, #723 and #465; write the CR-14 checklist into `handoffs/session-resume.md`; `/release` on the last merged PR.

**Telemetry:** check that the `crafting` row of `docs/architecture/observability.md` lists every event and metric in the telemetry contract, and run each CR-14 query once against a local server to prove it returns the rows it promises. Fix the query table or the code where they disagree.

### CR-14: owner UAT (colo, after the release)

**Status:** BlockedDependency (the `/release` after CR-13).

Run as GM in the stasis-room debug hub, and use `.bug <note>` at each oddity. The canonical checklist and its SigNoz query table are in the [session resume](handoffs/session-resume.md#cr-14-owner-uat-checklist), written by CR-13 against the code as merged; the tester-facing copy is the [Crafting section of the unified UAT guide](../../guides/unified-uat.md#crafting). The 16-step draft that stood here was replaced because parts of it no longer matched the code: `/showracialparadigmlevels` is not implemented, a Field Crafting Tool cannot sit in the main bag, alloy blueprint 42 takes five Good tier-1 Cells rather than ten Normal ones, the GM crafting commands need a selected player target, and the reverse-engineering query filtered on `verb = 'reverse_engineer'` where the code logs `reverseEngineer`.

Metrics: `crafting_requests_total` and `crafting_jobs_total` by `verb` and `outcome`, and `crafting_rejections_total` by `reason`.
