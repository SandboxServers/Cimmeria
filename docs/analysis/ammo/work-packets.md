# Ammo Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-28. Companions: [launch prompt and decisions](README.md), [audit](audit.md), [testing playbook](../../../TESTING.md).

## Why this shape

The issue's suggested packet order (AM-01 RE, then AM-02..AM-09 mostly serial) would leave most agents idle most of the time: AM-03 (validation) doesn't need AM-02 (reserve), AM-05 (loot) doesn't need either, and none of the four remaining ammo families need each other. This ledger restructures around one rule: **freeze the contracts once, in one small packet, then fan out as wide as the file graph allows.**

Changes from the issue's suggested sequence, and why:

- **A new packet, AM-F, is inserted before everything else.** It is the only serial gate. Every later packet codes against its item ids, its `AmmoReserve`/`AmmoModifier` Rust contracts, and its seed-file skeleton, instead of each packet inventing its own and colliding at merge time.
- **AM-01 (reverse engineering) does not gate AM-02 through AM-06, and in fact ran ahead of AM-F and finished first** (PR #1040). D-AM01 already decided "reserve lives in the bags" regardless of what AM-01 would find, and every Wave-1 packet looks ammo items up by `EAmmoType` through `ammo_item_types`, never a hardcoded id or a hardcoded reserve model. AM-01's one load-bearing finding — new-item-id feasibility is architecturally sound but unproven — reshaped AM-07 into a spike-then-batch packet rather than gating it on a separate RE packet; see [§ What AM-01 found](#what-am-01-found-and-what-it-changed-in-this-plan).
- **Validation (AM-03) is pulled out from under "close the TODO" and given its own packet up front**, because #602 (open) is the same hazard class and the same file — merging them separately would race two PRs on `ammo_change.rs`.
- **The damage framework is split from "Hollow Point and Armor Piercing content."** AM-04 builds the mechanism (read `cur_ammo_type`, apply an `AmmoModifier`) using HP/AP as its first two data rows, since D-AM04 orders HP/AP first anyway. Every later family (AM-08 through AM-11c) is then a small, symmetric "add a row + an effect file" packet gated only on AM-04, not on each other.
- **GM tooling (AM-06) is pulled forward into Wave 1** instead of sitting near the end, because every other Wave-1 and Wave-2 packet's own testing benefits from `/gmgiveammo` existing early.
- **Darts are split into three packets by effect kind** (crowd-control, tech-disable, buff/heal) instead of one "darts" packet, matching how differently `Dart_Tranquilizer`/`Dart_Poison`/`Dart_Disease` (debuff-shaped), `Dart_EMP`/`Dart_Radioactive` (tech-disable-shaped) and `Dart_Stim`/`Dart_Coagulant`/`Dart_Nanites`/`Dart_Antidote`/`Dart_Adrenaline` (buff/heal-shaped, closer to `StatBuff`) actually behave.

## Dispatch rules

- One worktree per worker, made with `bash tools/build-lane/mk-worktree.sh ammo/<packet>-<slug> ammo-<packet>`. Each worker has its own `sgw_<worktree>` test database.
- Every cargo call goes through `bash tools/build-lane/lane.sh cargo <cmd> -p <crate>`. Live-DB tests run through `bash tools/build-lane/live-db-test.sh <filter>`. Never `--workspace`, never `--exclusive`, per packet.
- **The coordinator is the single writer of this ledger's README and work-packets.md.** Packets write only their own `worknotes/AM-xx.md`; a packet that needs a contract change (a new owned file, a renamed struct field) raises it with the coordinator instead of editing this file directly.
- **Merge train, in dependency order, under the merge-on-minimum-CI rule** (fmt/clippy/build/change-specific lanes green — do not wait on coverage or live-DB before merging, per the owner's 2026-09-28 rule). Whoever merges second rebases onto whoever merged first, for any two packets that touch a shared file (see the matrix below).
- Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

`rust-gameserver-dev` is the default writer. The advisors:

- `items-systems-advisor`: AM-F's `AmmoReserve` contract, AM-02, AM-03, AM-05, reviews the bandolier and loot touch points in every packet;
- `combat-systems-advisor`: AM-04 and every Wave-2 family packet (the damage modifier and toggle-ability effects);
- `server-authority-enforcer`: reviews AM-02 (reserve draw, a TOCTOU-shaped stack decrement), AM-03 (the fail-closed fix), AM-06 (a new privileged native opcode);
- `database-persistence`: the `ammo_item_types` and `ammo_modifiers` table shapes (AM-F), and any live-DB test design;
- `game-archaeology-specialist`: AM-01;
- `documentation-writer`: the doc updates each packet owes, and AM-12's close-out.

## What AM-01 found, and what it changed in this plan

AM-01 ran ahead of AM-F and shipped as PR [#1040](https://github.com/SandboxServers/Cimmeria/pull/1040) ([findings](../../reverse-engineering/findings/ammo-system.md)). Its answers are folded into this ledger already (see the audit's §3a, §7 and §8, and [README.md § Open questions](README.md#open-questions)). Net effect on the plan:

- **Items-vs-pool**: confirmed no reserve model exists anywhere in the client schema — D-AM01 is a restoration design choice, not a recovery. No plan change; recorded for the historical record.
- **Widening a weapon's `ammo_types` needs no client push.** This *removed* scope from AM-07: it no longer covers the widened pistol/SMG families at all, only the 15 new ammo item definitions.
- **New-item-id feasibility is de-risked but not proven** (MEDIUM-HIGH confidence, architecturally sound, only *existing*-id overrides are proven in production). This did not gate AM-07 on AM-01 the way originally planned, because AM-01 already ran — instead it changed AM-07's *internal* structure: it now spikes one item first (push, then UAT in a live client) before batch-authoring the rest. The contract that contains a spike failure is unchanged from the original design: AM-02 through AM-06 never reference a hardcoded item id, only `ammo_item_types::item_id_for(ammo_type)`, so a failed spike means re-seeding two files (`ammo_items.sql`, `ammo_item_types.sql`) to repurpose existing, already-cooked placeholder ids instead of the reserved block — no other packet's code changes.
- **Toggle abilities 715/719 are independent of `requestAmmoChange`** — this is a real design decision AM-04 needs the owner's sign-off on (README open question 1); it does not change AM-04's file ownership, only its internal logic (apply the modifier directly, per the recommendation, rather than casting an ability).
- **`/gmgiveammo`/`/gmsetinfiniteammo` have no recovered native receiver.** This changed AM-06's design from "wire the native opcode" to "build a `.`-console command," and its owned-files list moved from `gm_gate.rs`/`grant_item.rs`-adjacent to `crates/cell-console/`.

## Contract fixed by AM-F

Parallel packets build against these names. A worker who needs to change one raises it with the coordinator instead of renaming it locally.

**Reserved item ids and the ammo-type mapping (AM-F).**

- Item ids `9000`-`9014` reserved for special-ammo items: `9000` Bullet_Armor_Piercing, `9001` Bullet_Hollow_Point, `9002` Bullet_Incendiary, `9003` Bullet_EMP, `9004` Bullet_Explosive, `9005`-`9014` the ten `Dart_*` specials (`Dart_Poison, Dart_Disease, Dart_Tranquilizer, Dart_EMP, Dart_Radioactive, Dart_Stim, Dart_Coagulant, Dart_Nanites, Dart_Antidote, Dart_Adrenaline`, in that enum order). Confirmed unclaimed against every `Items/Seed/*.sql` file (audit A-15). `9015`-`9099` held in reserve for anything this plan missed.
- New file `db/resources/Items/Seed/ammo_items.sql`: one `INSERT INTO items (...)` row per id above. Stackable (`max_stack_size` a real cap, not 1), `container_sets = '{1,15,17}'` (matches D-AM01: carried bags, the crafting bag is a valid mail/carry source per the Bank ledger, and the personal vault), `clip_size = 0` (these are reserve stacks, not weapons — `load_item_defs`'s `WHERE clip_size > 0` correctly excludes them from the `WeaponDef` cache), `ammo_types = '{}'`.
- New table `resources.ammo_item_types` (`db/resources/Items/Tables/ammo_item_types.sql`): `ammo_type "EAmmoType" PRIMARY KEY, item_id integer NOT NULL REFERENCES resources.items(item_id)`, seeded in `db/resources/Items/Seed/ammo_item_types.sql`. This is the one indirection every packet uses instead of a hardcoded item id (see above).
- One new Rust module, `crates/entity/src/ammo_type.rs`, with named `i32` constants matching `EAmmoType`'s enum ordinal exactly (`AMMO_NONE = 0, BULLET_DEFAULT = 1, BULLET_ARMOR_PIERCING = 2, BULLET_HOLLOW_POINT = 3, BULLET_INCENDIARY = 4, BULLET_EMP = 5, BULLET_EXPLOSIVE = 6, DAGGER_DEFAULT = 7 .. DART_DEFAULT = 13, DART_POISON = 14 .. DART_ADRENALINE = 23`, per the `CREATE TYPE` order in `db/resources/Abilities/Types/EAmmoType.sql`) plus `pub const fn is_special(ammo_type: i32) -> bool` (D-AM02: true for anything except `AMMO_NONE`, `BULLET_DEFAULT`, `DART_DEFAULT`). A live-DB test pins the ordinals against `SELECT enumlabel, enumsortorder FROM pg_enum WHERE enumtypid = 'resources."EAmmoType"'::regtype` so a future enum edit fails loudly instead of silently drifting (audit A-14 flagged nothing pins this today).
- New file `db/resources/Items/Seed/ammo_weapon_widening.sql`: `UPDATE resources.items SET ammo_types = ammo_types || ARRAY['Bullet_Armor_Piercing','Bullet_Hollow_Point','Bullet_Incendiary','Bullet_EMP','Bullet_Explosive']::resources."EAmmoType"[] WHERE description IN ('Standard Pistol', 'Standard SMG');` (pending the owner's confirmation of the family choice, README open question 5) — a mutating `UPDATE`, not a pure `INSERT`, but it lives in its own new file so it never conflicts with `items.sql` itself; the real goal is zero merge conflicts in the giant shared file, which an own-file `UPDATE` satisfies exactly as well as a pure `INSERT` would.
- Four new `\ir` lines added to `db/database.sql`, once, by AM-F: the three files above plus the `ammo_item_types` table file. This is AM-F's one shared-file touch.

**The `AmmoReserve` API (AM-F, real implementation, not a stub).**

```rust
// crates/base-methods/src/base/world_entry/methods/inventory/ammo_reserve.rs
pub struct AmmoDraw {
    pub drawn: i32,       // rounds actually removed from the stack; may be < requested
}
pub struct AmmoReturn {
    pub returned: i32,    // rounds actually added back to the stack; < requested if bag-capped
    pub remainder: i32,   // rounds that did not fit and must stay in the clip (D-AM05: never deleted)
}

/// Count of `ammo_type` rounds in the player's carried bags (1, 15), summed
/// across every matching stack. Read-only, no lock — callers that need a
/// consistent read-then-write take the lock themselves (see `draw`).
pub async fn count(tx: &mut Transaction<'_, Postgres>, player_id: i32, ammo_type: i32) -> Result<i32, sqlx::Error>;

/// Remove up to `n` rounds of `ammo_type` from the player's carried bags,
/// across as many stacks as needed, oldest slot first. Takes `FOR UPDATE`
/// on every touched `sgw_inventory` row before decrementing, deletes a
/// stack that hits zero, and never removes more than is present — the
/// caller reads `drawn` to know how many rounds actually loaded (D-AM05:
/// "if the stack is short, the reload loads what is there").
pub async fn draw(tx: &mut Transaction<'_, Postgres>, player_id: i32, ammo_type: i32, n: i32) -> Result<AmmoDraw, sqlx::Error>;

/// Add up to `n` rounds of `ammo_type` back to the player's carried bags:
/// merge into an existing stack first (room under `max_stack_size`), else
/// open a new stack in a free slot. `remainder` is what didn't fit — the
/// caller keeps those rounds in the clip rather than delete them (D-AM05).
pub async fn return_rounds(tx: &mut Transaction<'_, Postgres>, player_id: i32, ammo_type: i32, n: i32) -> Result<AmmoReturn, sqlx::Error>;
```

Built as a real, tested module (not a stub) in AM-F rather than left for AM-02, because the stack find/lock/decrement pattern already has a proven shape in this codebase (the Bank campaign's `D-BV25` stack-merge rules, and the vault move path's row-locking discipline) — shipping it once, real and tested, in the foundation packet means AM-02 only has to wire it into reload, not invent bag-stack locking from scratch under Wave-1 time pressure.

**The `AmmoModifier` shape (AM-F creates the table and the Rust loader; Wave-1/Wave-2 packets seed their own rows in their own files).**

- New table `resources.ammo_modifiers` (`db/resources/Abilities/Tables/ammo_modifiers.sql`): `ammo_type "EAmmoType" PRIMARY KEY, damage_mult real NOT NULL DEFAULT 1.0, penetration_mult real NOT NULL DEFAULT 1.0, on_hit_effect_id integer, toggle_ability_id integer NOT NULL`. Empty at AM-F time — no seed file yet. Per AM-04's design decision (README open question 1, recommendation option (a)), `toggle_ability_id` is **provenance-only**: it records which cooked ability's description/effect text the reconstructed `damage_mult`/`penetration_mult` numbers came from. It is never cast or engaged by the server — the modifier applies directly whenever the ammo type is loaded.
- Rust loader (`crates/cell-catalog/src/cell/spawner/ammo_modifiers.rs` or beside `load_item_defs` in `loot.rs`, coordinator's call at AM-F time): `load_ammo_modifiers(pool) -> HashMap<i32, AmmoModifier>`, cached the same way `WeaponDef` is.
- Each family packet (AM-04 for HP/AP, AM-08 Incendiary, AM-09 EMP, AM-10 Explosive, AM-11a/b/c darts) ships its **own** seed file (`ammo_modifiers_hp_ap.sql`, `ammo_modifiers_incendiary.sql`, …) with its own `\ir` line in `db/database.sql` — each packet's one shared-file touch, sequenced by the merge train.

**Feature flag (AM-F).** `ammo.finite_special` (a bool in the existing config/feature-flag surface — coordinator confirms the exact mechanism used elsewhere in the repo before AM-F ships), default **off**. While off: `requestAmmoChange` and the reload path behave exactly as they do on `main` today (special types validate the same as AM-03 makes them, but reload still refills for free — the flag gates only the reserve **draw**, not the whitelist fix, since the whitelist fix is a security fix that should ship regardless). AM-12 flips it on after every packet through AM-11c is merged and the debug-hub UAT (D-AM06) has passed.

**Telemetry contract.** Every packet satisfies all of the following; a worker who needs a new event adds a row here through the coordinator.

- **Target:** `ammo`. Every event is structured: an `event="…"` discriminator plus fields, never free text alone.
- **Correlators on every event:** `account_id`, `player_id`, `entity_id`, `item_id` (the ammo item's type id, from `ammo_item_types`), `ammo_type`.
- **Refusals:** a stable `reason=` string from the packet's reject enum.
- **Before and after:** every event that changes state records the prior and new values (`clip_before`/`after`, `stack_before`/`after`).
- **Guards:** every event row has a `LogCapture` test (TESTING.md type 12).
- **Filter:** `ammo` at `debug` needs an `OTEL_FILTER` row plus its pinning assertion in `crates/server/src/logging/`, added by whichever packet first emits a debug `ammo` event (AM-02).
- **Spans:** an info span per dispatch entrypoint (`ammo.reload_draw`, `ammo.ammo_change`, `ammo.gm_give`, …), named by each owning packet in its worknote and recorded here by the coordinator once merged.

| Event | Level | Packet | Fields beyond the correlators |
|---|---|---|---|
| `reload_draw` | debug | AM-02 | `requested`, `drawn`, `clip_before`, `clip_after`, `stack_before`, `stack_after` |
| `reload_refused` | warn | AM-02 | `reason` (`stack_empty`) |
| `ammo_switch_return` | debug | AM-02 | `returned`, `remainder`, `stack_before`, `stack_after` |
| `ammo_type_change_rejected` | warn | AM-03 | `reason` (`weapon_def_cache_miss`, `not_in_allowed_types`, `ambiguous_slot`, `item_not_in_bandolier`, `non_positive_ammo_type`) |
| `ammo_damage_applied` | debug | AM-04+ | `damage_mult`, `penetration_mult`, `toggle_ability_id`, `on_hit_effect_id` (when present) |
| `ammo_loot_dropped` | debug | AM-05 | `loot_table_id`, `quantity` |
| `gm_give_ammo` | info; warn on refusal | AM-06 | `quantity`, `reason` on refusal |
| `gm_infinite_ammo_toggled` | info | AM-06 | `on` |

## File-ownership matrix

Every packet lists the files it may edit. A file not listed for a packet must not be touched by it. Files marked **(new)** are created by that packet; everything else is an existing file being edited.

| Packet | Owned files | Must not touch |
|---|---|---|
| AM-F | `db/resources/Items/Seed/ammo_items.sql` (new), `db/resources/Items/Tables/ammo_item_types.sql` (new), `db/resources/Items/Seed/ammo_item_types.sql` (new), `db/resources/Items/Seed/ammo_weapon_widening.sql` (new), `db/resources/Abilities/Tables/ammo_modifiers.sql` (new), `db/database.sql` (4 new `\ir` lines), `crates/entity/src/ammo_type.rs` (new), `crates/base-methods/.../inventory/ammo_reserve.rs` (new), `crates/cell-catalog/.../ammo_modifiers.rs` (new), the feature-flag surface (wherever the repo's existing flag mechanism lives) | `ammo_change.rs`, `reload.rs`, `active_slot.rs`, `scripts.rs`, `registry.rs`, `items.sql`, `loot.sql` |
| AM-01 | `docs/reverse-engineering/findings/ammo-model.md` (new) | Everything else — read-only research packet |
| AM-02 Reserve | `crates/cell-combat/.../player/world/reload.rs` (split: reserve-draw logic into a new sibling `reload_reserve.rs`), `crates/cell-combat/.../bandolier/active_slot.rs` (split: switch-return logic into a new sibling `switch_return.rs`), `crates/wire/src/cell/messages/mod.rs` (new `CellToBaseMsg` variant if the reserve draw needs its own base round trip beyond `BandolierAmmoUpdate`) | `ammo_change.rs` (AM-03), `scripts.rs`/`registry.rs` (AM-04) |
| AM-03 Validation | `crates/cell-combat/.../bandolier/ammo_change.rs` | `reload.rs`, `active_slot.rs`, `scripts.rs`/`registry.rs` |
| AM-04 Damage framework | `crates/cell-world/src/cell/effects/ammo_damage.rs` (new), one match arm in `crates/cell-world/src/cell/effects/registry.rs`, `crates/cell-combat/src/cell/abilities/resolve.rs` (thread real ammo type through instead of the `0` placeholder), `db/resources/Abilities/Seed/ammo_modifiers_hp_ap.sql` (new), `db/database.sql` (1 new `\ir` line) | `scripts.rs` itself (new file only), `ammo_change.rs`, `reload.rs` |
| AM-05 Loot and crates | `db/resources/Loot/Seed/ammo_loot.sql` (new, covers table 3 and tables 8/9), `db/database.sql` (1 new `\ir` line) | Any Rust file — this packet should be pure data if `open_loot`/`roll_loot_entries` already handle arbitrary `design_id`s, which the loot-system doc suggests they do |
| AM-06 GM tooling | New files under `crates/cell-console/src/cell/console/gm/` (`give_ammo.rs`, `set_infinite_ammo.rs`, following the existing `give.rs`/`give_training_points.rs` pattern), `docs/commands.md` (flip the two rows to "Yes," and note they ship as `.`-console commands, not the native opcode) | `ammo_change.rs`, `reload.rs`, `registry.rs`, `gm_gate.rs` (native dispatch is not this packet's path — see the AM-06 scope note) |
| AM-07 Client push | `crates/resources/src/base/item_overrides.rs` (or a parallel new module if the spike finds new-id injection needs different plumbing than an attribute override) — scope is now only the 15 new ammo item definitions, never the widened weapons | Everything else, `ammo_items.sql`/`ammo_item_types.sql` (AM-F's; only touched if the spike fails and ids must be repointed, coordinated with AM-F's owner) |
| AM-08 Incendiary | `crates/cell-world/src/cell/effects/ammo_incendiary.rs` (new), one match arm in `registry.rs`, `db/resources/Abilities/Seed/ammo_modifiers_incendiary.sql` (new), `db/database.sql` (1 line) | `ammo_damage.rs`, every other family's file |
| AM-09 EMP | `crates/cell-world/src/cell/effects/ammo_emp.rs` (new), one match arm in `registry.rs`, `db/resources/Abilities/Seed/ammo_modifiers_emp.sql` (new), `db/database.sql` (1 line) | Same isolation as AM-08 |
| AM-10 Explosive | `crates/cell-world/src/cell/effects/ammo_explosive.rs` (new), one match arm in `registry.rs`, `db/resources/Abilities/Seed/ammo_modifiers_explosive.sql` (new), `db/database.sql` (1 line) | Same isolation as AM-08 |
| AM-11a Darts (crowd-control: Poison, Disease, Tranquilizer) | `crates/cell-world/src/cell/effects/ammo_dart_cc.rs` (new), one match arm in `registry.rs`, its own seed file, `db/database.sql` (1 line) | Same isolation |
| AM-11b Darts (tech-disable: EMP, Radioactive) | `crates/cell-world/src/cell/effects/ammo_dart_tech.rs` (new), one match arm in `registry.rs`, its own seed file, `db/database.sql` (1 line) | Same isolation |
| AM-11c Darts (buff/heal: Stim, Coagulant, Nanites, Antidote, Adrenaline) | `crates/cell-world/src/cell/effects/ammo_dart_support.rs` (new), one match arm in `registry.rs`, its own seed file, `db/database.sql` (1 line) | Same isolation |
| AM-12 Close-out | `docs/gameplay/weapon-ammo-reload.md`, `docs/gameplay/combat-system.md`, `docs/gameplay/loot-system.md`, `docs/commands.md` (if not already flipped by AM-06), `docs/architecture/abilities-and-effects-system.md`, `docs/gap-analysis.md`, `docs/project-status.md`, `docs/guides/unified-uat.md`, the feature-flag flip | Nothing else — pure doc + flag packet |

**Shared contended file: `registry.rs` (80 lines today).** Every Wave-2 family packet adds exactly one `match` arm. This cannot be fully parallelized away — it is the one place the effects system centralizes dispatch — but each edit is a single line, so the merge train's "whoever merges second rebases" rule keeps the cost to a one-line conflict, not a redesign. **`db/database.sql`** is the same shape: many packets each add one or two `\ir` lines; same mitigation.

## Dependency graph and waves

```text
AM-01 RE (done ahead of AM-F, PR #1040 — findings folded into every packet below)

AM-F (serial gate: item ids, ammo_item_types, AmmoReserve, AmmoModifier shape, pre-splits, flag, telemetry catalog)
  │
  ├─► AM-02 Reserve (reload draw, partial reload, switch-return)
  ├─► AM-03 Validation (requestAmmoChange vs ammo_types; absorbs #602)
  ├─► AM-04 Damage framework (HP/AP; the mechanism every later family reuses) ──┬─► AM-08 Incendiary
  ├─► AM-05 Loot and crates (NPC drops, debug crate table 3, Castle chest HP)   ├─► AM-09 EMP
  ├─► AM-06 GM tooling (.gmgiveammo, .gmsetinfiniteammo — console, not native) ├─► AM-10 Explosive
  └─► AM-07 Client push (spike one item, then batch the rest — item defs only)┴─► AM-11a/b/c Darts (3 parallel)
                                                                                         │
                                                                                         ▼
                                                                                     AM-12 Close-out
```

Wave 1 is 6 packets in parallel (AM-02 through AM-07); AM-01 already shipped and is not a Wave-1 occupant. Wave 2 is 6 packets in parallel (AM-08, AM-09, AM-10, AM-11a, AM-11b, AM-11c), gated only on AM-04. Wave 3 is AM-12 alone, gated on every packet merging.

**Maximum useful parallelism per wave:** AM-F = 1, Wave 1 = 6, Wave 2 = 6, AM-12 = 1.
**Critical path:** AM-F → AM-04 → (longest of AM-08/09/10/11a/11b/11c) → AM-12 = 4 packets deep.

## AM-F: foundation

**Status: Ready.** The only serial gate. Scope, contract and file list are fully specified above. No player-visible behavior changes — `ammo.finite_special` stays off, `requestAmmoChange`'s whitelist logic is untouched (AM-03's job), reload is untouched (AM-02's job).

Tests:

- **Unit.** `is_special()` for every `EAmmoType` ordinal. `AmmoReserve::draw`/`return_rounds` arithmetic (in-memory, no DB) for the boundary cases D-AM05 names: exact fit, short stack, over-capacity return.
- **Live-DB.** The ordinal-pinning test against `pg_enum` (audit A-14). `AmmoReserve::draw` under a real `sgw_inventory` fixture: draws across two stacks, deletes an emptied stack, never over-draws. `AmmoReserve::return_rounds` merges into an existing stack first, then opens a new slot, and reports `remainder` correctly when bags are full.
- **Seed guard.** `ammo_item_types` has exactly 15 rows, one per non-default special `EAmmoType`; every `item_id` it references exists in `resources.items`.

Docs: none yet — AM-F ships no player-visible behavior, so the gameplay docs wait for AM-02/03/04 to have something to describe. `docs/architecture/abilities-and-effects-system.md` gets a short forward-pointer to `ammo_modifiers` so nobody reading it after AM-04 wonders where the table came from.

## AM-01: reverse engineering

**Status: Done** (PR [#1040](https://github.com/SandboxServers/Cimmeria/pull/1040); ran ahead of AM-F, findings in [docs/reverse-engineering/findings/ammo-system.md](../../reverse-engineering/findings/ammo-system.md)). Writer: `game-archaeology-specialist`.

Answered:

1. **Items vs. pool** — no reserve model of any kind exists in the client schema (HIGH confidence in the absence). D-AM01 is a restoration design choice.
2. **The client's allowed-ammo-type source** — a live, server-populated container cache (`SGWPlayer+0x8c → +0x24`), not `CookedDataItems.pak` (HIGH). Widening `ammo_types` is DB-only.
3. **Toggle-ability linkage** — 715/719 and `requestAmmoChange` are independent wire paths; no auto-engagement found (MEDIUM). Acceptance criterion 4 is a design decision for AM-04 (README open question 1).
4. **New-item-id feasibility** — architecturally should work (no static id-range check), but only proven in production for *existing* ids (#405); recommend a spike before depending on it structurally (MEDIUM-HIGH).
5. **Client-side gates** — none on reload or ammo picking (HIGH); a different, unrelated gate exists on bandolier active-slot swapping, flagged as a UAT caution only.

Follow-up open items the finding itself could not close in its time budget (not blocking any packet, but worth a future session): the exact `Event_NetOut_GiveAmmo` byte layout, and whether `getAmmoTypes`/`getCurrentAmmoType`'s lookup key is a container id, an item id, or both.

## AM-02: reserve

**Status: BlockedDependency (AM-F).**

Scope:

- Reload draws from `AmmoReserve` when the active slot's `cur_ammo_type` is special (`is_special()`), for `clip_size - current_ammo` rounds, landing in the existing `reload_completion_tick` refill path but replacing "refill to `clip_size`" with "refill to `current_ammo + drawn`" — D-AM05's partial-reload arithmetic.
- An empty stack (`drawn == 0`) refuses the reload before the warmup timer even starts, with visible feedback (`onErrorCode`) and the clip left exactly as it was (acceptance criterion 2 in #1026).
- `requestAmmoChange`'s ammo-type swap (the AM-03 handler) triggers switch-return: the unfired rounds in the clip for the **previous** ammo type, if special, go back to that type's stack via `AmmoReserve::return_rounds`; `remainder` (bags full) stays in the clip rather than being deleted, per D-AM05. This needs a small hook in `ammo_change.rs` calling into AM-02's new file — coordinate the exact call site with AM-03 before either merges (they touch adjacent logic in the same handler even though the file-ownership matrix keeps them in separate files).
- Under the `ammo.finite_special` flag: off means the reload path behaves exactly as `main` today (free refill for every ammo type, special included) — the flag gates only the reserve draw, so a partial Wave-1 merge never changes live behavior.

Tests:

- **Unit.** The partial-reload arithmetic table from acceptance criterion 3a (18/30 with a 100-round stack → 30/30 clip, 88 stack; 18/30 with a 5-round stack → 23/30 clip, 0 stack).
- **Live-DB regression guards**, each proven to fail with the fix reverted: a reload with a special type and an empty stack is refused and the clip is unchanged; a reload that partially fills leaves the exact remainder in the stack; a double-fire of the reload-completion tick does not double-draw (idempotent on `reload_slot_id`, matching the existing pinned-slot guard).
- **Concurrency.** Two reload completions racing the same player's stack (implausible today given one tick loop, but the DB-level `FOR UPDATE` guard should be exercised directly) do not double-draw.
- **Wire-format.** The reload refusal (`onErrorCode`), byte-exact.

Telemetry: `reload_draw`, `reload_refused`, `ammo_switch_return`, per the catalog.

Review by `server-authority-enforcer` (a stack decrement under concurrent access is exactly its TOCTOU wheelhouse) and `items-systems-advisor`.

## AM-03: validation

**Status: BlockedDependency (AM-F).**

Scope:

- Absorb #602: the cache-miss fall-open becomes fail-closed (`weapon_def_cache_miss`), exactly as that PR's diff already does — the coordinator should pull that branch in as this packet's starting point rather than re-derive it, since it is already written, reviewed and tested. Remove the stale TODO comment (audit A-06).
- Add the acceptance-criterion-1 requirement: reject `ammo_type` values not in the weapon's `ammo_types` column even when they ARE in the `WeaponDef` cache's `allowed_ammo_types` — these should already agree, since `load_item_defs` derives `allowed_ammo_types` from `ammo_types` at load time (audit A-09), so this is confirmation via a live-DB test, not new logic, unless the audit's assumption is wrong (check first).
- Wire in AM-02's switch-return call (coordinate the exact call site, see AM-02 above).

Tests:

- **Live-DB regression guards**, each proven to fail with the fix reverted: a cache-miss item accepts no `ammo_type` (closes #602's hole); a weapon whose `ammo_types` excludes a value refuses it even if `WeaponDef` somehow disagreed; a same-item duplicate-slot request is still rejected as ambiguous (existing behavior, guard against regressing it).
- **Wire-format.** The rejection path sends no `BandolierAmmoUpdate` and no `onEntityProperty` — byte-level "nothing was sent" assertion, matching #602's existing test shape.

Docs: `docs/gameplay/weapon-ammo-reload.md` § `requestAmmoChange flow` (the TODO removal), `docs/security-audit/2026-05-31-server-authority/findings/CAT-D-inventory.md` (CAT-D-06 resolved, as #602 already stages).

Review by `items-systems-advisor` and `server-authority-enforcer`.

## AM-04: damage framework

**Status: BlockedDependency (AM-F, and the owner's sign-off on the design decision below).**

**Design decision (needs owner confirmation, README open question 1): option (a), direct modifier application.** AM-01 found 715/719 are architecturally independent of `requestAmmoChange` in the client — nothing auto-engages a toggle ability when the player picks an ammo type. Rather than build that missing link (option b: server-launches the toggle ability on ammo-type selection, which entangles ammo switching with ability cooldowns — 719's is 30 s — for no benefit anything in the issue or the client asks for), AM-04 applies the ammo type's damage/penetration modifier **directly and automatically** whenever a shot fires with that type loaded. The toggle abilities 715/719 stay exactly as seeded, untouched; `ammo_modifiers.toggle_ability_id` is provenance-only, recording which ability's description/effect text the reconstructed multiplier came from. Ability 715 has empty `effect_ids` (reconstruct from its description text); 719's effect 747 exists but is shaped for a flat ability-cast (`FocusDamage`/`HealthDamage`), not obviously a multiplier — treat its numbers as a starting point for the reconstructed multiplier, labelled as reconstruction per acceptance criterion 4, not a direct port.

Scope:

- New effect script(s) in `ammo_damage.rs` that read the attacker's active bandolier slot's `cur_ammo_type` (via `ctx.source_id` → `space_mgr.get_entity(source_id)` → `active_ammo_type()`), look up its `AmmoModifier`, and apply `damage_mult`/`penetration_mult` to the existing `RangedPhysicalDamage`/`MeleeDamage` calculation — either by wrapping/calling the existing script with a modifier argument, or by promoting the modifier lookup into `EffectContext` so every physical-damage script picks it up for free. Coordinator or `combat-systems-advisor` picks the exact seam; either way it is additive to `scripts.rs`'s existing logic, not a rewrite (`RangedPhysicalDamage` at `scripts.rs:453-482` should be touched only if the wrapping approach is rejected in favor of an in-place edit — the file-ownership matrix bars AM-04 from `scripts.rs` on the assumption wrapping wins; revisit with the coordinator if it doesn't).
- Ship the Hollow Point (715) and Armor Piercing (719, effect 747) modifier rows first, per D-AM04's ordering.
- `resolve.rs`'s `default_ammo_type: 0, cur_ammo_type: 0` placeholder (audit A-10) is replaced with the real value read from the acting entity.

Tests:

- **Unit.** The modifier lookup and multiply-through math.
- **Live-DB.** `ammo_modifiers` seed rows for 715/719 load correctly.
- **Chain-replay or smoke.** A Hollow Point shot does measurably more damage than a default shot against the same target under the same ability; Armor Piercing the reverse for penetration.

Telemetry: `ammo_damage_applied`.

Review by `combat-systems-advisor`.

## AM-05: loot and crates

**Status: BlockedDependency (AM-F).**

Scope:

- `db/resources/Loot/Seed/ammo_loot.sql`: table 3 gets one row per bullet special ammo item at probability 1, full stack (`min_quantity = max_quantity = ` the item's `max_stack_size`), plus rows adding the chosen widened pistol and SMG item ids (D-AM06). Tables 8 and 9 (Castle pre-Romney chest) get one additional Hollow Point row each, alongside the existing #1031 rows — additive, not a replacement.
- NPC loot tables: add ammo-item rows at low-to-moderate probability across a representative slice of existing mob loot tables, per the acceptance criterion "ammo items drop from NPC loot tables." Scope the exact table list with `items-systems-advisor` — this audit did not enumerate every mob loot table.

Tests:

- **Live-DB.** Table 3's new rows exist and reference valid item ids; a roll against table 3 with a fixed RNG seed (if the loot roller supports one for testing, per existing loot-system test patterns) or a statistical-repeat test yields the ammo items at probability 1 reliably.
- **Chain-replay.** The debug crate grant round-trips through `open_loot` → `onLootDisplay` → `lootItem`/Loot All, landing the ammo stack in the bags.

Telemetry: `ammo_loot_dropped`.

Review by `items-systems-advisor`.

## AM-06: GM tooling (revised: console-first)

**Status: BlockedDependency (AM-F).**

AM-01 found no recoverable server-side receiver for the native `GiveAmmo` opcode (audit A-22: the event is registered and client-emittable, but no `.def` entry exists, and the wire byte layout is unrecovered). Per the project's existing split — a command with a native index uses the client's `/` console, a command with no recovered native binding uses the GM-gated `.`-console — this packet ships as `.`-console commands, not a native-opcode handler.

Scope:

- `.gmgiveammo <ammoType> <quantity>` (GM-gated, `crates/cell-console/src/cell/console/gm/give_ammo.rs`, following `give.rs`'s pattern): grants `quantity` rounds of the ammo item mapped from `ammoType` (an `EAmmoType` ordinal, per `ammo_type.rs`) via `ammo_item_types::item_id_for`, clamped similarly to `gmGiveItem`.
- `.gmsetinfiniteammo <on>` (GM-gated): a per-player bool that `AmmoReserve::draw` (AM-02) checks first — when set, skip the reserve draw and refill from nothing (today's behavior), matching `bInfiniteAmmo`'s existing but unread property. Confirm the exact semantics with the owner first (README open question 3).
- Update `docs/commands.md`: flip both rows to "✅ Yes," and note in the "Parameters" or a footnote that they ship as `.`-console commands (`.gmgiveammo`/`.gmsetinfiniteammo`), not the native `/gmgiveammo`/`/gmsetinfiniteammo` slash commands, since the native receiver was never finished server-side (cite AM-01). If a future session recovers `Event_NetOut_GiveAmmo`'s byte layout, wiring the real native opcode alongside the console command is a clean, additive follow-up — not scoped here.

Tests:

- **Console/dispatch.** A GM's `.gmgiveammo` grants the item; a non-GM's is refused with the generic `.`-console non-GM message.
- **Live-DB.** The granted stack lands correctly, respecting `max_stack_size`.

Telemetry: `gm_give_ammo`, `gm_infinite_ammo_toggled`.

Review by `server-authority-enforcer` (a new privileged console command).

## AM-07: client push (narrowed scope, early spike)

**Status: BlockedDependency (AM-F).** No longer gated on AM-01 (it already shipped); AM-01's finding narrowed this packet's scope and added an internal ordering.

**Scope narrowed by AM-01 (audit A-22): this packet covers only the 15 new ammo item definitions (icon, name, stack cap). The widened pistol/SMG `ammo_types` need no client push at all** — `getAmmoTypes`/`requestAmmoChange` read the live server-populated container cache, not cooked data, so AM-F's DB-only widening is already sufficient.

**Internal ordering — spike first, per AM-01's recommendation (audit A-20):**

1. **Spike.** Push a cooked-data entry for exactly one of the 15 reserved ammo item ids (e.g. `9001`, Bullet_Hollow_Point) through the same `versionInfoRequest → onVersionInfo → resourceFragment` handshake `item_overrides.rs` already uses, and confirm in a live client (colo or local) that the item renders correctly (name, icon, stack behavior) the first time the client has ever seen that id. This is the empirical test AM-01 could not complete itself.
2. **If the spike succeeds:** author `ItemOverride`-equivalent entries (or genuinely new cooked-data authoring, if the mechanism for a never-shipped key differs from patching an existing one) for the remaining 14 ids.
3. **If the spike fails** (the client silently drops the fragment, or renders nothing usable): re-seed `ammo_items.sql`/`ammo_item_types.sql` (AM-F's files, coordinate the edit with whoever owns AM-F's branch at that point) to repurpose existing, already-cooked placeholder item ids instead of the reserved block, then this packet becomes ordinary `ItemOverride` entries for those ids — the Slappack (#405) pattern exactly, already proven in production.

Tests: a wire-format test for the resource-fragment push (byte-exact, per the existing `item_overrides` test pattern) either way. The spike step itself is a manual client UAT, not an automated test — record its result in `worknotes/am-07.md` before proceeding to step 2 or 3.

## AM-08 through AM-11c: the remaining families

**Status: BlockedDependency (AM-04).** Each is a small, symmetric packet: one new effect file, one `registry.rs` match arm, one `ammo_modifiers` seed file, one `db/database.sql` line, plus whatever on-hit effect the ability's cooked text (or reconstruction, labelled as such per acceptance criterion 4) calls for:

- **AM-08 Incendiary** (723): likely a DoT-shaped `on_hit_effect_id`, following the existing DoT effect-script pattern in `scripts.rs` if one exists, else a new minimal one.
- **AM-09 EMP** (1445): likely a tech/shield-disable-shaped effect against mechanical targets; check `resources.effects` for 1445 first (audit § toggle abilities).
- **AM-10 Explosive** (1446): likely an AoE-on-hit shape; check whether the existing cone/AoE machinery (`cell/abilities/cone_aoe/`) is the right seam before writing a new one.
- **AM-11a/b/c Darts**: three packets by effect kind (crowd-control: Poison/Disease/Tranquilizer; tech-disable: EMP/Radioactive; buff/heal: Stim/Coagulant/Nanites/Antidote/Adrenaline — the last group is close in shape to the existing `StatBuff` script and should reuse it as a base). Ability ids per the issue: 990-1228, 2874, 2876 — map each specific ability id to its dart type before scoping (not done in this audit).

Each packet's tests: unit (modifier math), live-DB (seed rows), and a chain-replay or smoke test demonstrating the on-hit effect actually fires. Docs: each packet appends its family's row to `docs/architecture/abilities-and-effects-system.md`'s script table.

Review by `combat-systems-advisor` for all six.

## AM-12: close-out

**Status: BlockedDependency (every packet above).**

Scope:

- Flip `ammo.finite_special` on.
- Update `docs/gameplay/weapon-ammo-reload.md`, `docs/gameplay/combat-system.md`, `docs/gameplay/loot-system.md`, `docs/architecture/abilities-and-effects-system.md`.
- `docs/gap-analysis.md` and `docs/project-status.md` rows, once, per the campaign-close-out rule.
- Extend `docs/guides/unified-uat.md` with the ammo section, using the outline in [README.md](README.md#uat-checklist-outline).
- Confirm every catalog telemetry event has its `LogCapture` guard on `main`; record any exempt reason with why, following the Bank campaign's "Known gaps" pattern.
