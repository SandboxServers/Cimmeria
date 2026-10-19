# UB-01b: Beta way-back chains, shared Beta test helpers, linter slice

> **Type:** How-to (work packet) · **Index:** [beta-work-packets.md](../beta-work-packets.md) · **Wave:** B1 · **Common:** [chain-packet-common.md](chain-packet-common.md)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 38k |
| Branch / worktree | `union-beta-e1/ub-01b-way-back` / `ub-01b` |
| Commit subject | `feat(beta-e1): UB-01b Union arrivals on Beta learn the Omega Site address` |
| Depends on | UB-01a merged |
| Ship order | First in wave B1. UB-02a to UB-02d need nothing from it, but every chain packet does |

## Why

A Union player who gates to Beta must be able to dial home. The Dakara precedent (chain 8001) grants Omega Site's address on `player_loaded`. Beta needs the same grant for the three Union races and nobody else: Goa'uld (6) and Jaffa (8) are Praxis and must not learn Omega. This packet also lays the shared ground that every later Beta chain packet edits one line of:

- the `beta_e1` replay-test module and its helpers;
- the Beta block in `db/database.sql`;
- the slice form of the dialog-button linter's `CHAIN_FILES`;
- the Beta range table in `content-chains.instructions.md`.

## Contract excerpt

- Omega Site = `stargates.stargate_id` **5** (world 18). The Beta world name is `Beta_Site_Evo_1`.
- `grant_stargate_address` does nothing for an address the player already holds, so a `player_loaded` chain without a mission gate is allowed (`content-chains.instructions.md` "Dakara_E1" note).
- Archetypes (F4): Human 1-4, Asgard 5, Goa'uld 6, Free Jaffa 7, Jaffa 8, missing -1.
- The existing guard `live_db_every_address_a_player_loaded_chain_grants_has_a_way_back` (`crates/cell-world/src/cell/gate_dakara_e1_tests.rs:217`) walks **every** `player_loaded` grant. It will cover 23001-23003 without edits, so run it.

## Rules (binding)

1. PowerShell only; start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ub-01b;`.
2. Never run `cargo` directly; use `pwsh tools/build-lane/lane.ps1` and `pwsh tools/build-lane/live-db-test.ps1`. Run in the foreground and check test counts are above 0.
3. Edit only the allowlist.
4. The seed and docs are CRLF (normalise after writing). In `db/database.sql`, `seed_model.rs`, `interact_tag_linter.rs` and `.github/instructions/*.md`, keep each file's existing line endings.
5. Seeds in `db/resources/` only; no migrations.
6. Under 500 lines per file.
7. Never touch `sgw`; no lab.
8. No `git stash`, prune or push; one local commit.
9. All three chains use `delay_ms` 0, `scope_type` `'space'` and `scope_id` 23.
10. Provenance comment on each chain: CIMMERIA_RECONSTRUCTION, precedent DK-01 chain 8001.

## Allowlist

| File | Change |
|---|---|
| `db/resources/Content/Seed/beta_e1_travel_chains.sql` | new, chains 23001-23003 |
| `db/database.sql` | one `\ir` line after `\ir resources/Content/Seed/dakara_e1_space_chains.sql` (line 407), under a comment `-- Beta Site E1 (world 23), Union first visit: 23000-23499` |
| `crates/content-engine/tests/it/dialog_button_linter/seed_model.rs` | `CHAIN_FILES` becomes `pub(crate) const CHAIN_FILES: &[&str] = &[ ... ];`, plus `"beta_e1_travel_chains.sql",` |
| `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` | `mod beta_e1;` before `mod black_market_auctioneer;` |
| `crates/cell-content/src/cell/content/chain_replay_tests/beta_e1/mod.rs` | new: helpers, plus `mod travel;` |
| `crates/cell-content/src/cell/content/chain_replay_tests/beta_e1/travel.rs` | new: tests |
| `.github/instructions/content-chains.instructions.md` | new subsection after "Dakara_E1" |
| `crates/content-engine/tests/it/interact_tag_linter.rs` | `git mv` to `crates/content-engine/tests/it/interact_tag_linter/mod.rs` (no content change except the two lines in step 6b) |
| `crates/content-engine/tests/it/interact_tag_linter/allowlist.rs` | new: `fn allowlist` and its doc comment, moved out of `mod.rs` unchanged, plus one `||` line |
| `crates/content-engine/tests/it/interact_tag_linter/beta_e1.rs` | new: the Beta allowlist slice (empty) |

## Steps

1. **Seed file** `beta_e1_travel_chains.sql`. Use the header from chain-packet-common, then:

   | Chain | Description | Conditions (sort 0, 1) | Action |
   |---|---|---|---|
   | 23001 | `Beta_Site_Evo_1 - Human arrival: learn the Omega Site address (UB-01b)` | `archetype gte 1`, `archetype lte 4` | `grant_stargate_address` target 5 |
   | 23002 | `... Asgard arrival ...` | `archetype eq 5` | same |
   | 23003 | `... Free Jaffa arrival ...` | `archetype eq 7` | same |

   Each chain has one trigger: `player_loaded`, `event_key` `'Beta_Site_Evo_1'`, scope `'player'`, `once` false.

2. **database.sql**:

   ```text
   -- Beta Site E1 (world 23), Union first visit: 23000-23499 (docs/analysis/union-zone-planning/beta-work-packets.md)
   \ir resources/Content/Seed/beta_e1_travel_chains.sql
   ```

3. **CHAIN_FILES.** Change the type to a slice and append the travel file. Fix any call site the compiler flags; `for file in CHAIN_FILES` now yields `&&str`, and `dir.join(file)` still compiles. Run the linter test to prove it still passes. The travel file has no `dialog_choice`, so this only proves the slice works.

4. **Helpers** (`beta_e1/mod.rs`). Start with a `//!` header naming the campaign and the seed files. Copy `engine_for`, `fire`, `summarized` and `interact_ctx` from `chain_replay_tests/mission_701/mod.rs`, and change `engine_for`'s panic text to name `beta_e1_*_chains.sql`. Then add:

   ```rust
   pub(super) const BETA: &str = "Beta_Site_Evo_1";
   pub(super) const HUMANS: [i64; 4] = [1, 2, 3, 4];
   pub(super) const ASGARD: i64 = 5;
   pub(super) const GOAULD: i64 = 6;
   pub(super) const FREE_JAFFA: i64 = 7;
   pub(super) const JAFFA: i64 = 8;
   /// Every archetype outside a gate, plus 0 (Any): use to prove a race gate.
   pub(super) const ALL_ARCHETYPES: [i64; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];

   pub(super) async fn whole_seed(pool: &PgPool) -> ChainEngine {
       super::super::engine_loader::build_engine(Some(pool)).await
   }
   pub(super) fn chains_matched(resolved: &ResolvedActions) -> std::collections::BTreeSet<i64> {
       resolved.actions.iter().map(|(id, _)| *id).collect()
   }
   pub(super) fn login_ctx(world: &str, archetype: Option<i64>) -> ExecutionContext { /* world_name; archetype if Some */ }
   pub(super) fn choice_ctx(dialog_id: i64, archetype: i64) -> ExecutionContext { /* dialog_id + archetype (CS-01b sets both) */ }
   pub(super) fn item_use_ctx(item_id: i64, archetype: i64) -> ExecutionContext { /* item_id + archetype */ }
   pub(super) fn region_ctx(region_key: &str, archetype: i64) -> ExecutionContext { /* region_key + archetype */ }
   pub(super) fn completed_ctx(mission_id: i64, archetype: i64) -> ExecutionContext { /* mission_id + archetype */ }
   pub(super) fn with_mission(ctx: &mut ExecutionContext, mission: i32, status: &str) { /* mission_{m}_status */ }
   pub(super) fn with_step(ctx: &mut ExecutionContext, mission: i32, step: i32, status: &str) { /* mission_{m}_step_{s}_status */ }
   pub(super) fn with_obj(ctx: &mut ExecutionContext, mission: i32, obj: i32, status: &str) { /* mission_{m}_obj_{o}_status */ }
   pub(super) fn with_counter(ctx: &mut ExecutionContext, name: &str, value: i64) { /* counter_{name} */ }
   ```

   `summarize` covers every verb the Beta packets use (the full list is in chain-packet-common). Copy the 701 arms and add `CompleteObjective`, `AddItem`, `RemoveItem`, `IncrementCounter`, `GrantStargateAddress` and `MoveEntity`. **Read the exact field names from `crates/content-engine/src/actions/`** before writing the arms. Keep the `other => format!("UNEXPECTED({other:?})")` arm. Mark helpers that no test uses yet `#[allow(dead_code)]` with the comment `// used by later Beta packets (UB-10+)`, so clippy `-D warnings` passes.

5. **Tests** (`beta_e1/travel.rs`). Each is `#[tokio::test]`, starts `require_db_or_skip!()`, and has a `///` comment naming its revert:
   - `human_arrival_learns_omega`: for each of `HUMANS`, `engine_for(23001)` plus `login_ctx(BETA, Some(a))` gives `["grant_stargate_address(5)"]`. Then call `super::super::assert_no_deferred_actions(&resolved, 23001)`.
   - `asgard_arrival_learns_omega` (23002, archetype 5) and `free_jaffa_arrival_learns_omega` (23003, archetype 7), with the same shape.
   - `praxis_and_unknown_arrivals_learn_nothing`: for archetypes 0, 6, 8 and `None`, run `whole_seed` with `login_ctx(BETA, ..)`. `chains_matched` must not contain 23001, 23002 or 23003.
   - `each_union_race_matches_exactly_one_travel_chain`: run `whole_seed` with `login_ctx(BETA, Some(a))` for every archetype 1-5 and 7. The intersection of `chains_matched` with {23001, 23002, 23003} must be exactly one chain: the race's own.
   - `other_worlds_do_not_grant`: run `login_ctx("Omega_Site", Some(1))` against `engine_for(23001)` and expect it to be empty.

6. **content-chains.instructions.md.** After the Dakara subsection, add:

   ````markdown
   ### Beta_Site_Evo_1 (world 23) - 23000-23499, one file per mission

   The Union Beta E1 campaign ([docs/analysis/union-zone-planning/beta-work-packets.md](../../docs/analysis/union-zone-planning/beta-work-packets.md)) reserves `23000-23499`. Each mission packet owns one file and its sub-block. Race gates are Human `archetype gte 1` and `lte 4`, Asgard `eq 5`, Free Jaffa `eq 7`. Beta chains bind dialogs per player only (`add_dialog_set` with `slot` = template id) and never call `set_interaction_type` on a shared NPC.

   ```text
   beta_e1_travel_chains.sql   travel, way back: 23000-23019 (23001-23003 used)
   ```
   ````

6b. **Split the interact-tag linter.** `interact_tag_linter.rs` is 713 lines, over the 700 hard cap, and every Beta chain packet must add allowlist rows (Beta never calls `set_interaction_type`, D-UB10). Split it along its natural seam before anyone adds to it:
   - `git mv crates/content-engine/tests/it/interact_tag_linter.rs crates/content-engine/tests/it/interact_tag_linter/mod.rs`. `main.rs` keeps `mod interact_tag_linter;` unchanged.
   - Move `fn allowlist` (with its whole doc comment, lines about 100-200) into `interact_tag_linter/allowlist.rs` as `pub(super) fn allowlist(...)`. In `mod.rs`, add `mod allowlist;`, `mod beta_e1;` and `use allowlist::allowlist;`. Move no other code.
   - At the end of the `matches!(...)` expression in `allowlist`, add `|| super::beta_e1::allowed(filename, chain_id)`.
   - `interact_tag_linter/beta_e1.rs`:

     ```rust
     //! Beta Site E1 (world 23) interact_tag allowlist (Union Beta E1 campaign,
     //! docs/analysis/union-zone-planning/beta-work-packets.md). Beta chains bind
     //! dialogs per player (`add_dialog_set`, slot = template) and Beta objects carry
     //! INT_MissionWorldObject from their template (UB-02a), so no Beta chain calls
     //! `set_interaction_type`. Each chain packet appends its own rows.

     /// (seed file, chain id, where the interaction bit comes from)
     const BETA_E1: &[(&str, i32, &str)] = &[];

     pub(super) fn allowed(filename: &str, chain_id: i32) -> bool {
         BETA_E1.iter().any(|(f, c, _)| *f == filename && *c == chain_id)
     }
     ```
   - Check: `mod.rs` and `allowlist.rs` together hold the same lines as the old file plus the new `mod`/`use`/`||` lines. Both are under 700 and `mod.rs` is about 600. The linter's test count is unchanged before and after.

7. **Revert proof.** Delete chain 23002's rows and rerun `chain_replay_tests::beta_e1`; `asgard_arrival_learns_omega` fails in `engine_for`. Change 23001's `lte 4` to `lte 5` and rerun; `each_union_race_matches_exactly_one_travel_chain` fails. Restore and rerun green. Quote all of it in the report.

## Lane checks (in order)

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content -p cimmeria-content-engine
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-content-engine --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine --test it dialog_button_linter interact_tag_linter
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::beta_e1
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 gate_dakara_e1
```

## Acceptance

- Six travel tests are green. The shared way-back guard is green and now sees 23001-23003. The linter suite is green with the slice.
- SigNoz (`cimmeria-cell`, UB-90 row UB-U1): a Human, Asgard or Free Jaffa entering Beta logs `resolve_event: chain matched` with `chain_id` 23001, 23002 or 23003, then `Content: granting stargate address` (`executor/stargate.rs`) for stargate 5. A second login logs the grant as a no-op, not an error. Grep the exact text before quoting it.

## Docs rows

- Row 41 (content chains): the new `content-chains.instructions.md` subsection.

## Return

`UB-01b <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ub-01b\.claude\reports\union-beta-e1\UB-01b-coder.md`
