# UO-35 ruling: eight guide tags on four two-trigger chains

> **Type:** Ruling (architect) · **Packet:** [UO-35](../UO-35.md) · **Date:** 2026-10-19
> **Supersedes:** the packet's tag column, its § Allowlist and its § Steps 3, 5 and 6 where they differ. Everything else in the packet stands.

## Ruling

**Re-spec.** The packet put two spawns on each of four tags. `spawnlist.tag` is `UNIQUE` (`spawnlist_tag_key`, `db/resources/_primary_keys.sql:643`), and `interact_tag` triggers match the tag, not the spawn. So each of the eight spawns gets its own tag. The four chains 63852-63855 stay, one per specialty, and each chain gets **two `interact_tag` triggers**: the plain spot's tag (`sort_order` 0) and the Adv. spot's tag (`sort_order` 1). Each spawn pair already splits into one plain spot and one Adv. spot.

This works with no schema change, no new ids and no code change:

- The loader builds one in-memory `Chain` per `content_triggers` row, with shared conditions and actions. Any one trigger fires the chain (OR semantics): `crates/content-engine/src/loader/mod.rs:236-302`, pinned by `loader/tests/chain_loading.rs:103`. Precedent: chain 1103 fires on three `Barracks_Guard` tags (`chain_replay_tests/mission_687.rs:81-105`).
- The `interact_tag` linter matches on `(file, chain_id)`. Chains 63852-63855 are already in `interact_tag_linter/union_omega.rs`, so **that file does not change**. `scan_chains` keeps only the first tag of each chain, so the second trigger cannot cause a violation.
- `content_triggers` has a serial primary key and no unique key on `chain_id`.

Options rejected:

| Option | Why not |
|---|---|
| Eight tags, eight chains (63852-63855 plus 63858-63861) | Also works. The spare ids 63858-63869 are free, and 63858-63861 fall in the services range the guard allows. But it costs four extra chains, four linter entries and an ID-map edit, and gains no behavior. Rejected as the larger change. |
| Drop `spawnlist_tag_key` | This is a schema change. The repo rule is no migrations, and other zones rely on unique tags. |
| One spawn per specialty | Breaks D-UO31, which puts guides on all eight art spots. |

No owner decision is needed. D-UO31 is unchanged.

## Tags, chains and screens

| Spawn | Spot | Tag (`spawnlist.tag` = trigger `event_key`) | Chain | Trigger `sort_order` | Screen |
|---|---|---|---|---|---|
| 63810 | `ca.guide.biology` | `UnionOmega_Guide_Biology` | 63852 | 0 | 63901 |
| 63817 | `sgc.guide.adv_biology` | `UnionOmega_Guide_AdvBiology` | 63852 | 1 | 63901 |
| 63811 | `ca.guide.electronics` | `UnionOmega_Guide_Electronics` | 63853 | 0 | 63902 |
| 63812 | `ca.guide.adv_electronics` | `UnionOmega_Guide_AdvElectronics` | 63853 | 1 | 63902 |
| 63813 | `ca.guide.engineering` | `UnionOmega_Guide_Engineering` | 63854 | 0 | 63903 |
| 63814 | `ca.guide.adv_engineering` | `UnionOmega_Guide_AdvEngineering` | 63854 | 1 | 63903 |
| 63815 | `ca.guide.physics` | `UnionOmega_Guide_Physics` | 63855 | 0 | 63904 |
| 63816 | `sgc.guide.adv_physics` | `UnionOmega_Guide_AdvPhysics` | 63855 | 1 | 63904 |

The four plain tags are unchanged, so the UO-24 linter strings and the existing `SERVICE_TAGS` entries stay valid.

## Allowlist (replaces the packet's)

| File | Change |
|---|---|
| `db/resources/Entities/Seed/entity_templates_union_omega.sql` | As the packet says; the draft patch's row is correct. |
| `db/resources/Worlds/Seed/spawnlist_union_omega.sql` | 8 rows inside `[UO-35]`. Copy them verbatim from the worknotes, **except** the `tag` value of 63812, 63814, 63816 and 63817, which comes from the table above. This ruling overrides packet rule 12 for those four values only. Keep the trailing `-- ca.guide.*` comments. |
| `db/resources/Dialogs/Seed/dialogs_union_omega.sql` | As the packet says; the draft patch's screens are correct. |
| `db/resources/Content/Seed/union_omega_services_chains.sql` | Chains 63852-63855 inside `[UO-35]`, using the SQL below. |
| `crates/cell-content/src/cell/content/chain_replay_tests/union_omega/guides.rs` | Replace the stub; see § Tests. |
| `crates/cell-content/src/cell/content/chain_replay_tests/union_omega/arrival.rs` | **Added.** Change only the `SERVICE_TAGS` constant and its doc comment (lines 211-222). |

Do not touch: `interact_tag_linter/union_omega.rs`, `services_common.rs`, any `mod.rs`, the linter `allowlist.rs`. All six files are CRLF; write CRLF only.

## Chain SQL (replaces the draft's `[UO-35]` block body)

```sql
-- CIMMERIA_RECONSTRUCTION: crafting guides from shipped verbs (D-UO31); no reward (D-UO37)
-- Two interact_tag triggers per chain (plain spot, Adv. spot): spawnlist.tag is UNIQUE (UO-35 ruling)
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63852, 'Union Omega: the Biology guides name the Biomedical vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63852, 'interact_tag', 'UnionOmega_Guide_Biology', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63852, 'interact_tag', 'UnionOmega_Guide_AdvBiology', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63852, 'npc_bark', NULL, NULL, '{"screen_id": 63901, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63853, 'Union Omega: the Electronics guides name the Electronics vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63853, 'interact_tag', 'UnionOmega_Guide_Electronics', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63853, 'interact_tag', 'UnionOmega_Guide_AdvElectronics', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63853, 'npc_bark', NULL, NULL, '{"screen_id": 63902, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63854, 'Union Omega: the Engineering guides name the Power Systems vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63854, 'interact_tag', 'UnionOmega_Guide_Engineering', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63854, 'interact_tag', 'UnionOmega_Guide_AdvEngineering', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63854, 'npc_bark', NULL, NULL, '{"screen_id": 63903, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);

INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (63855, 'Union Omega: the Physics guides name the Materials vendors', 'space', 18, true, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63855, 'interact_tag', 'UnionOmega_Guide_Physics', 'player', false, 0);
INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (63855, 'interact_tag', 'UnionOmega_Guide_AdvPhysics', 'player', false, 1);
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (63855, 'npc_bark', NULL, NULL, '{"screen_id": 63904, "speaker": "SGC Scientist", "channel": "say"}', 0, 0);
```

## Tests

### `guides.rs`

Start from the draft patch and make these changes:

1. **`GUIDES`.** Take the tags from the table above, keeping spawn order 63810-63817. The template test's `ORDER BY spawn_id` depends on that order.
   ```rust
   const GUIDES: [(i32, &str, i32, i32); 8] = [
       (63810, "UnionOmega_Guide_Biology", 63852, 63901),
       (63811, "UnionOmega_Guide_Electronics", 63853, 63902),
       (63812, "UnionOmega_Guide_AdvElectronics", 63853, 63902),
       (63813, "UnionOmega_Guide_Engineering", 63854, 63903),
       (63814, "UnionOmega_Guide_AdvEngineering", 63854, 63903),
       (63815, "UnionOmega_Guide_Physics", 63855, 63904),
       (63816, "UnionOmega_Guide_AdvPhysics", 63855, 63904),
       (63817, "UnionOmega_Guide_AdvBiology", 63852, 63901),
   ];
   ```
2. **Engine: every expansion.** `services_common::engine_with` uses `load_single_chain_for_test`, which returns only the **first** expansion. With it, every Adv. click would come back unhandled. Do not use `engine_with` in this file, and do not edit `services_common.rs` (its `#![allow(dead_code)]` covers the unused helper). Add a private helper in `guides.rs`, following the precedent in `mission_687.rs:86-105`:
   ```rust
   use super::super::super::engine_loader::load_chain_expansions_for_test;
   use cimmeria_content_engine::chain::ChainEngine;

   /// The four guide chains with every trigger expansion registered: each
   /// chain has a plain-spot and an Adv.-spot `interact_tag` trigger.
   async fn guide_engine(pool: &sqlx::PgPool) -> ChainEngine {
       let mut engine = ChainEngine::new();
       for chain_id in [63852, 63853, 63854, 63855] {
           let expansions = load_chain_expansions_for_test(pool, chain_id)
               .await
               .unwrap_or_else(|e| panic!("DB query for chain {chain_id}: {e}"));
           assert_eq!(
               expansions.len(),
               2,
               "chain {chain_id} must hold two interact_tag triggers (plain and Adv. guide)"
           );
           for chain in expansions {
               engine.register_chain(chain);
           }
       }
       engine
   }
   ```
   Put the helper and its imports where they compile cleanly under clippy `-D warnings`. If the helper is only used in `mod tests`, put it inside `mod tests`. In `live_db_each_omega_guide_speaks_its_specialty`, replace `engine_with(&pool, &[63852, 63853, 63854, 63855]).await` with `guide_engine(&pool).await`. Change nothing else in that test.
3. Fix the `//!` header wording if needed: "eight guides, each with its own tag; four chains, two triggers each". The other two tests stay as in the draft.

### `arrival.rs`: `SERVICE_TAGS` only

Make the constant `[&str; 12]`, and put each Adv. tag right after its plain tag:

```rust
/// Service tags of the hub services (chains 63850-63869, `interact_tag`). Each
/// names spawns of a template that carries its interaction bit as a default.
/// Each crafting guide has its own tag (`spawnlist.tag` is unique), so guide
/// chains 63852-63855 carry two triggers each, plain and Adv. (UO-35).
const SERVICE_TAGS: [&str; 12] = [
    "UnionOmega_Auctioneer",
    "UnionOmega_Medic",
    "UnionOmega_Guide_Biology",
    "UnionOmega_Guide_AdvBiology",
    "UnionOmega_Guide_Electronics",
    "UnionOmega_Guide_AdvElectronics",
    "UnionOmega_Guide_Engineering",
    "UnionOmega_Guide_AdvEngineering",
    "UnionOmega_Guide_Physics",
    "UnionOmega_Guide_AdvPhysics",
    "UnionOmega_Inn",
    "UnionOmega_Battlemaster",
];
```

`live_db_omega_service_tags_name_npcs_with_no_dialog` then checks each new tag against its one spawn. That needs no edit.

## Revert proofs (replaces the packet's table)

| Mutation | Fails |
|---|---|
| Point chain 63854's bark at screen 63902 | `live_db_each_omega_guide_speaks_its_specialty`: spawns 63813 and 63814 speak the Electronics line |
| Delete chain 63854's `sort_order` 1 trigger | `live_db_each_omega_guide_speaks_its_specialty`: the expansion count is 1, not 2 |
| Set 63829's `trainer_ability_list_id` to 1 | `live_db_omega_guide_template_reaches_the_chain` |
| Remove `UnionOmega_Guide_AdvPhysics` from `SERVICE_TAGS` | `live_db_omega_interact_tag_chains_are_services_only` (and the array length fails to compile, so shrink it to 11) |
| Give screen 63903 `index` 1 | The seed load fails on `UNIQUE (dialog_id, index)`; quote the error |

## Lane checks

These are the packet's five checks, unchanged. Check 3 (`live-db-test.ps1 union_omega`) runs the `arrival.rs` guards. Check 4 (`interact_tag_linter`) must pass with `union_omega.rs` unchanged. The `git diff` touches nothing outside the five `[UO-35]` seed pairs, `guides.rs` and `arrival.rs` lines 211-228 (the new constant and its doc comment).

## Collisions checked

- **Chain ids:** 63852-63855 are unchanged, so the § ID map in `omega-work-packets.md` stays true. No new chain, template, spawn or screen ids.
- **`arrival.rs`:** among the W9 packets (UO-31 to UO-34, UO-36), none lists it, and UO-24 (W7, merged) is the only other writer. No merge conflict inside `uo-w9`. UO-35 merges fifth.
- **`union_omega_services_chains.sql`:** UO-32, UO-33 and UO-36 each write only their own marker pair. No overlap.
- **`services_common.rs`:** not edited. UO-32, UO-33 and UO-36 keep `engine_with`; their chains have one trigger each, so the first expansion is the whole chain.

## Follow-ups (coordinator, not blocking)

- **UO-98** (`docs/content/mission-chains.md`) lists the guide chains with **eight** keys, two per chain. Add this to UO-98's brief.
- **UO-U1 § R5:** both spawns of a specialty log the same `chain=6385N screen=6390N` on "Content: npc bark". The clicked tag tells the plain guide from the Adv. guide.
- `UO-35.md`, `UO-24.md`, `UO-17.md`, `UO-20.md` and the UO-17/UO-20 worknotes still show the old tag pairs. This ruling supersedes them for UO-35. Add a pointer to this note when the ledger is next touched.
