# Crafting Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-26. Companions: [launch prompt and decisions](README.md), [audit](audit.md), [session resume](handoffs/session-resume.md), [testing playbook](../../../TESTING.md), [ability-tree ledger](../ability-trees/work-packets.md) (same dispatch rules).

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
- `crates/base-session/src/base/crafting/`: `session.rs` (the queue, CR-06), `transaction.rs` (consume and grant, CR-06), then one file per verb: `spend.rs`, `craft.rs`, `research.rs`, `reverse_engineer.rs`, `alloy.rs`, `respec.rs`, and `feedback.rs` (CR-01) for the rejection path. `handlers.rs` keeps the GM grants.
- Log target `crafting`, with events `request`, `rejected reason=…`, `induction_started`, `completed`, `persist_failed`. Add it to `OTEL_FILTER` with its pinning assertion.

## Dependency graph and waves

```text
Wave 0 (now, parallel)        Wave 1 (after CR-01, parallel)          Wave 2 (parallel where marked)            Wave 3
CR-01 foundation ─────┬──► CR-03 login sync + paradigm defaults ──┐
CR-E1 client evidence │    CR-04 spend ASP (+ D-CR04 blueprints) ─┼──► CR-07 craft ─────────────┐
CR-02 game clock ─────┼──► CR-05 stations + options gate + GM ────┤    CR-08 research + rev-eng ─┼──► CR-13 close-out ─► CR-14 owner UAT
                      └──► CR-06 induction engine + transaction ──┘    CR-09 alloy ──────────────┤     + /release
                                                                       CR-10 respec (D-CR02) ─────┤
                                                                       CR-11 debug hub (#846) ────┤
                                                                       CR-12 ASP earning (D-CR01) ┘
```

CR-01 is the only bottleneck. It is kept small: catalog, constants, serializers, argument parsing and the message shape, with **no behaviour change**. CR-E1 and CR-02 touch no file CR-01 owns.

**Contended files.** The coordinator merges these one packet at a time:

- `crates/wire/src/mercury/world_data/map_loaded.rs`: CR-01 (moves the 139 builder out), then CR-03.
- `crates/cell-methods/.../player/crafting/mod.rs`: CR-01, then CR-05 (gate), then CR-10 (respec arm). Verb packets add files only.
- `crates/base-session/src/base/crafting/mod.rs` and the base dispatch arm: CR-01 adds the `Crafting` arm; later packets add files and `mod` lines only.
- `base/world_entry/methods/progression/mod.rs` (`grant_xp`): CR-12 only.
- `crates/entity/src/cell_entity/entity_struct.rs` is **over the 700-line cap**. CR-05's per-player station state goes in a new file, not in that struct's body.
- `db/resources/Entities/Seed/entity_templates.sql`, `Worlds/Seed/spawnlist.sql`: CR-11 only, inside 310-329 and 410-429. Message the guilds session (cimmeria-fa) and the pets session (cimmeria-b5) before merging.
- `crates/wire/src/containers.rs` and `inventory/move_/mod.rs`: no packet here should need them. If one does, message cimmeria-fa first; its vault packet edits both for containers 19 and 20.

## Common acceptance

- Every behaviour change ships a regression guard that **fails when the fix is reverted** ([TESTING.md](../../../TESTING.md)), proven once and recorded in the worknote.
- Each rejection reason gets a test that asserts the feedback is sent (D-CR14), not only that nothing happened.
- Consumption and grants get live-DB tests with exact-sentinel cleanup, including one that proves a mid-induction logout consumes nothing (D-CR12).
- Wire output gets byte-exact tests. New WARNs get `LogCapture` tests.
- Randomness goes through an injectable RNG, so tests pin outcomes (CAT-F F-04: outcomes are server-side only).
- Each packet updates the docs it owes: `docs/gameplay/crafting-system.md`, `docs/gap-analysis.md` §19, `docs/protocol/` when a wire message changes, and the crafting findings when evidence shifts.
- Worknote at `docs/analysis/crafting/worknotes/<packet>.md` (CRLF). Workers do not edit this file or the README; the coordinator owns the ledger.

## Wave 0

### CR-01

**Status:** Ready. **Scope title:** Catalog, constants, serializers, argument parsing, request message. **Depends:** none. **Advisor:** database-persistence, testing-validation-engineer.

**Scope:**

- `CraftingCatalog` and its loader (the contract above), loaded once at cell startup and shared with the base the way `AbilityTreeCatalog` is. Component sets are grouped by `component_set_id`. Item attributes: `flags`, `tier`, `quality_id`, `tech_comp`, `discipline_ids`, `applied_science_id`.
- The enums and constants in the contract, with a test that pins each value to `entities/defs/enumerations.xml`.
- Serializers for 112, 137, 138, 139 and 140 in `crates/wire`, byte-exact tested; `map_loaded.rs` calls the new 139 serializer. Record the `CraftingOptions` layout in `docs/protocol/`.
- Convert `player/crafting.rs` into `player/crafting/`, parse every argument of 95-100 per `SGWPlayer.def:894-949` (including the `ARRAY<ItemID>`s), and forward a `CellToBaseMsg::Crafting(CraftRequest)` with `allowed = 0`. The base arm logs the request at `crafting` and sends the D-CR14 "not available yet" text line, so a press is never silent.
- `base/crafting/feedback.rs`: `reject(entity_id, reason)` sends the text line and, where a code is mapped, `onErrorCode`. One enum of reasons; later packets add variants.
- Move the crafting live-DB sentinels to `0x7000_Cxxx` (#800).

**Acceptance:** catalog loaded from a fresh `db/database.sql` in a live-DB test with the counts in audit C-20 to C-23 (78 / 498 / 2,556, blueprint 21 has no components); round-trip parse tests for every verb's arguments; the forward carries every argument.

### CR-E1

**Status:** Ready. **Scope title:** Client evidence for the crafting UI. **Depends:** none. **Writer:** game-archaeology-specialist (Ghidra, read-only on the client; the client Lua under `..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI`). Documentation only, plus Ghidra comment fixes.

**Questions, each answered with an address or file:line:**

1. Respec: what `Event_SlashCmd_RespecCraft` (`0x018417f0`) sends, and whether a full respec sends method 100 once or twice (feeds D-CR16).
2. `CraftingOptions`: confirm the nested array encoding from the unpacker (`0x00e49180` → `0x00e47250`), and whether the client itself checks distance to the machine entity or trusts the server.
3. Which feedback reaches the player: does `onErrorCode` with `ERRORCODE_SYSTEM_Ability` and codes 213/214 print text? Where does `Mercury__unknown_00ceae50` write (log or chat)? Which server text path shows in chat?
4. The client clock `FUN_00c6e220`: which message sets it (`SET_GAME_TIME`, `TICK_SYNC`, `UPDATE_FREQUENCY_NOTIFICATION`) and in what unit (feeds CR-02).
5. Alloy: confirm the `0x00e46990` counts, stack-quantity counting, and what the client sends as `aCurrentTierItemId` and `aLowerTierItems` (C-38).
6. Research kickers: confirm the one-per-science rule and whether the kicker's "+1 Expertise" text maps to any client-side number (C-39).

**Output:** `docs/reverse-engineering/findings/crafting-client-ui.md`, indexed in the findings README; corrections for audit C-61 and C-62 in the existing crafting findings and in the Ghidra PRE_COMMENT at `0x00e465d0`; a short `worknotes/cr-e1.md` that feeds D-CR16 and D-CR20.

### CR-02

**Status:** Ready. **Scope title:** One consistent game clock for timer expiries. **Depends:** none. **Advisor:** bigworld-engine-advisor, combat-systems-advisor.

**Scope:**

- Make `SET_GAME_TIME`, `TICK_SYNC` and the ongoing tick sync agree on one clock (audit C-13): the declared tick rate matches the rate the counter actually advances, and login sends the current time, not 0.
- One `game_time_secs()` that every `onTimerUpdate` sender can use for `BigWorldTimeComplete`.
- Use it for the ability warmup and cooldown timers that send `0.0` today only if CR-E1 Q4 confirms the domain; otherwise leave them and record why.

**Acceptance:** a unit test that the declared tick rate and the advance per loop agree; a test that a timer built with `game_time_secs() + d` decodes to `now + d` on the same clock. CR-14 checks the bar in game. If the evidence says the client's clock cannot be matched without a client patch, stop and report; D-CR20's fallback then stands.

## Wave 1 (after CR-01 merges)

### CR-03

**Status:** BlockedDependency (CR-01); paradigm defaults BlockedDecision (D-CR03). **Scope title:** Login sync, ASP display and paradigm defaults. **Advisor:** aoi-witness-broadcast.

**Scope:**

- Call `load_crafting_state` at player load. Send, owner-only: 136 per known discipline, 138 per paradigm, 139, 140 (empty unless CR-05's gate says otherwise), and the ASP property.
- Every ASP change (GM grant, spend, respec, earning) pushes `onEntityProperty(2, total)`, the **total** (audit C-06, C-57). Fix the doc comment (C-63).
- Paradigm defaults per D-CR03, applied when a character has no stored levels, so existing characters are covered without a migration.

**Acceptance:** a byte-exact test of the login crafting bundle for a fixture state; a live-DB test that a relog restores disciplines, expertise, paradigms and blueprints; a guard that the GM ASP grant pushes the property.

### CR-04

**Status:** BlockedDependency (CR-01); the blueprint grant BlockedDecision (D-CR04). **Scope title:** `spendAppliedSciencePoints` (95). **Advisor:** server-authority-enforcer, database-persistence.

**Scope:**

- `base/crafting/spend.rs`: one transaction that checks ASP ≥ 1, the discipline exists, it is not already known, the paradigm level, and every prerequisite known at expertise ≥ 50; learns it at expertise 1, spends 1 ASP, and grants blueprints per D-CR04. It is replay-safe: a second identical request finds the discipline known and changes nothing (CAT-F F-02).
- Then 136, the ASP property and (if blueprints changed) 139.
- Every rejection sends feedback: 214 `NotEnoughAppliedSciencePoints` where it fits, text otherwise (D-CR14).

**Acceptance:** a live-DB test per rejection reason and for the success path; a replay test; a guard that fails when the prerequisite check is removed.

### CR-05

**Status:** BlockedDependency (CR-01); stations vs tools BlockedDecision (D-CR05) for tools only. **Scope title:** Station gate, crafting options and "craft anywhere". **Advisor:** aoi-witness-broadcast, server-authority-enforcer.

**Scope:**

- A station is any entity whose template `entity_flags` carries `ENTITYFLAG_Craft_*` bits. For each player the cell tracks the nearest station per verb within interaction range (`MAX_INTERACT_DISTANCE`), and sends 140 when that set changes: entering or leaving range, the station despawning, or a world change. The state lives in a new file (the entity struct is over its cap).
- The forward to the base carries `allowed`; the base rejects a verb whose bit is missing, with the text "No crafting station for …" (D-CR14).
- `.allcraft` and "craft anywhere" per D-CR17, GM-gated.
- Spawning a flagged entity needs no new AoI path. If it does, message cimmeria-b5 first.

**Acceptance:** a unit test of the options set against positions; a byte-exact 140 test; a guard that a forged request with no station in range is rejected; a test that "craft anywhere" is refused for a non-GM.

### CR-06

**Status:** BlockedDependency (CR-01). CR-02 is soft: until it lands, the timer uses the best available clock. **Scope title:** Induction engine and the consume-and-grant transaction. **Advisor:** items-systems-advisor, testing-validation-engineer, server-authority-enforcer.

**Scope:**

- `session.rs`: a per-player `CraftingSession` with one active induction and a FIFO queue of at most 10 (D-CR13). Starting an induction sends `onTimerUpdate(type 16, SourceID = entity, TotalTime = 3.0, expiry)`. Dropped on logout or world change without consuming. A test clock drives expiry.
- `transaction.rs`: one transaction that locks the player and the named items, checks that each is owned and sits in bag 1 or 15, consumes by design across bags 1 and 15 (C-31), grants outputs by stack merge or free slot, and adjusts expertise. After commit: `onRemoveItem` for drained stacks, `onUpdateItem` for changed ones, and a full resync as the fallback. A failure rolls back and sends feedback.
- An injectable RNG for the verbs that roll.
- No verb uses it yet; CR-07 to CR-09 plug in.

**Acceptance:** live-DB tests for a partial stack, a drained stack, a full inventory (rollback plus feedback), an item moved to the bank mid-induction (rollback), and a logout mid-induction (nothing consumed); a queue test for the eleventh request.

## Wave 2

### CR-07

**Status:** BlockedDependency (CR-04 for known blueprints, CR-05, CR-06). **Scope title:** `craft` (96). **Advisor:** items-systems-advisor, server-authority-enforcer.

**Scope:** `craft.rs`. Blueprint known, discipline known (D-CR15), not an alloy, `quantity` in 1..=100, the submitted ids pick the component set whose component types they cover, and the bags hold `quantity × component.quantity` of each. At completion: consume, grant `blueprint.quantity × quantity` of the product, +1 expertise, send 136. Blueprint 21 (no components) is rejected.

**Acceptance:** live-DB end to end with blueprint 412 set 1 (14× 5254) and set 2; a multi-stack requirement (C-31); each rejection with feedback; a guard for the discipline-known check.

### CR-08

**Status:** BlockedDependency (CR-05, CR-06); reverse engineering BlockedDecision (D-CR06). **Scope title:** `research` (97) and `reverseEngineer` (98).

**Scope:**

- `research.rs`: item researchable (`Craft_Research`), kickers flagged `Kicker`, at most one per applied science and none from the item's own (D-CR11). At completion: consume the item and kickers, roll per D-CR15, +5 on success, and a text line either way.
- `reverse_engineer.rs`: item reverse-engineerable (`Craft_RevEng`) and produced by at least one blueprint. At completion: consume the item, pick a blueprint and a component set uniformly (no C-50), recover per D-CR06, grant. Up to 10 queued (C-34).

**Acceptance:** seeded-RNG tests for success and failure; a burst of 10 reverse-engineer requests completes 10 times; guards for the kicker rules and the zero-expertise case.

### CR-09

**Status:** BlockedDependency (CR-04, CR-05, CR-06). **Scope title:** `alloying` (99). **Advisor:** items-systems-advisor.

**Scope:** `alloy.rs`. An alloy blueprint known with its discipline; the current-tier item is the blueprint's component (C-53 and C-54 fixed); the elementary items are exactly one tier lower, and their summed stack quantity meets the client's count for one quality (Normal 10, Good 5, Great 2, Fantastic 1; D-CR11). At completion: consume, grant 2× product, +1 expertise.

**Acceptance:** live-DB end to end with blueprint 42; one test per count rule; a guard for the tier rule.

### CR-10

**Status:** BlockedDecision (D-CR02) and BlockedDependency (CR-E1 Q1, CR-04). **Scope title:** `respecCrafting` (100, 112, 137). **Advisor:** server-authority-enforcer, database-persistence.

**Scope:** `respec.rs`, per D-CR16 as corrected by CR-E1: the prompt, the pending window, then one transaction that charges the cost, clears disciplines, expertise and discipline-granted blueprints, and refunds ASP per D-CR02. Then 137, the ASP property and 139. Not enough naquadah or nothing to reset gets feedback. Replay-safe.

**Acceptance:** live-DB tests for the charge, the refund and the clear; a guard that a single send never wipes; a replay test.

### CR-11

**Status:** BlockedDependency (#846 merged, CR-05); BlockedDecision (D-CR05) for station placement beyond the hub. **Scope title:** Crafting stations and supplies in the stasis-room debug hub. **Advisor:** items-systems-advisor.

**Scope:**

- Templates 310-313: the four "<Science> Crafting Station" entities, named from the existing texts (audit C-25), with all four `ENTITYFLAG_Craft_*` bits. Spawns 410-413 in the stasis room, placed per `docs/content/debug-hub.md` (read its placement warning and the hub worker's authoring traps in `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`).
- Template 314 / spawn 414: a "crafting supplies" vendor selling the components of the UAT recipes (audit §2) and the four kickers at 1 naquadah.
- `.craftkit <blueprint> [count]` (D-CR17).
- Add the stations to `docs/content/debug-hub.md`.

**Acceptance:** a live-DB seed guard that the templates carry the flags and the spawns sit in world 12; the vendor list resolves; a `.craftkit` test.

### CR-12

**Status:** BlockedDecision (D-CR01). **Scope title:** Earning ASP. **Advisor:** combat-systems-advisor (the `grant_xp` path).

**Scope:** per D-CR01. With the recommendation: `grant_xp` adds the levels gained to `applied_science_points` in the same statement that raises the level, and pushes the property; new characters start with 1.

**Acceptance:** a live-DB test that a level-up grants ASP atomically with the level; a guard that fails when the grant is removed.

## Wave 3

### CR-13

**Status:** BlockedDependency (every packet above that the owner has not deferred). **Scope title:** Close-out, UAT checklist and release.

**Scope:** `docs/gameplay/crafting-system.md`, `docs/gap-analysis.md` §19, `docs/project-status.md`, the crafting findings (C-60), CAT-F paths (C-64); close or update #567, #723 and #465; write the CR-14 checklist into `handoffs/session-resume.md`; `/release` on the last merged PR.

### CR-14: owner UAT (colo, after the release)

Run as GM in the stasis-room debug hub, and use `.bug <note>` at each oddity.

1. Log in with a new character. Open Ctrl+J: the ASP count shows, and the tree is drawn (green where learnable, per D-CR03).
2. `/gmgiveappliedsciencepoints 5` (or the native GM console). The count updates without a relog.
3. Learn Biomedical Engineering (21). Its expertise reads 1 and the ASP count drops by 1. Click it again: a message says it is already known.
4. Relog. Disciplines, expertise, ASP and blueprints are all still there.
5. Open J away from the stations: every tab says "Disabled". Walk to the Materials Crafting Station: the tabs enable. Walk away: they disable again.
6. Buy 14× Steel Core (5254) from the supplies vendor, or `.craftkit 412`. Craft Titanium Plating (blueprint 412): the 3 s induction bar shows, the components go, the plating arrives, and expertise rises by 1.
7. Craft with too few components: a message explains why and nothing is consumed.
8. Research Crafted Pistol of the Whale (5481) with one kicker: a message reports the result; on success expertise rises by 5.
9. Put 10 items in reverse engineering and confirm: all 10 complete in turn, and components arrive.
10. Alloy with blueprint 42 and 10 Normal tier-1 elementary components: 2× Blend (Bio-Medical Alloy) arrives.
11. Log out during an induction and log back in: nothing was consumed.
12. `/respeccraft` (or the respec path CR-E1 finds): the cost prompt shows; confirm; disciplines clear, ASP is refunded, naquadah is charged.
13. `.allcraft`: every tab enables anywhere, and every discipline shows 100.
