---
name: ability-grant-provenance-seams
description: CS-01a grant provenance (sgw_player_ability_grants) - which writers must record a row, conversion of bought nodes, starters, where branch credit is computed, the InitPlayerState race, and the two-stage grant_ability load check
metadata:
  type: project
---

Learned implementing Class Start v6 CS-01a and its review fixes (2026-10-05).

- **Every writer that appends to `sgw_player.abilities` without a trainer
  purchase must decide its provenance row.** `.giveability` and
  `persist_bulk(GrantAll)` write `gm` rows (`grant_provenance::record_gm_grants`,
  `ON CONFLICT DO NOTHING`); `content_grant_write::persist_content_grant`
  writes its kind, *promotes* a `gm` row, *converts* a trained id (out of
  `trained_abilities`, node cost refunded, spend reduced) and writes **no**
  row for the archetype's char-creation starters (no free credit). A new
  grant path that forgets a row makes the GM reset drop the ability.
- **Lock order:** `sgw_player ... FOR UPDATE` first, then
  `sgw_player_ability_grants`, in one transaction.
- **Branch credit lives on the cell only.** `TreeProgress::credited_grants`
  is hydrated by `player_init_row`'s ARRAY subquery (filters `gm` and
  starters; guarded by `live_db_player_init_row_credits_only_non_gm_non_starter_grants`)
  and grown by `ContentAbilitiesGranted`. The base's purchase UPDATE never
  re-checks spend.
- **Base handles client packets and cell messages on separate tasks**
  (`crates/base/src/base/service.rs`), so an `InitPlayerState` built from an
  older read can reach the cell after a grant reply. `init_grant_merge`
  merges the entity's credited ids back in (safe because non-gm rows are
  append-only). Any other cell mirror that `InitPlayerState` overwrites has
  the same race.
- **`grant_ability` validation is two-stage.** Shape (kind, list, no `gm`,
  `archetypes` required for signature/racial_core) refuses the whole chain
  inside `build_chains_from_rows`; ability-id existence is
  `refuse_chains_with_unknown_abilities`, called only from
  `engine_loader::load_chains_from_db`. Test loaders skip the id check.
- **Trigger `archetype` coverage:** after CS-01b every player dispatcher sets
  it; `world_context_contract_tests.rs` covers stargate, mission, dialog and
  flank, but not region enter/exit/teleport_in (those set it in code only).
- **Adding a field to `TreeProgress` breaks ~7 struct literals** in
  `crates/cell/src/cell/service/base_messages/tests/` plus
  `client_ready/mod.rs`; adding one to `TrainContext` breaks the five
  `cell-catalog` test contexts and the two callers (trainer.rs, train.rs).

Related: [[cell-mirrors-of-base-owned-counters]], [[log-filter-parity-traps]].
