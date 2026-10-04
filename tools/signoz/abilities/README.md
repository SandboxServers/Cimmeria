# Ability SigNoz views and dashboard

> Type: reference. Audience: the coordinator importing these into the colo SigNoz, and anyone rebuilding them.
> Added by AB-T7 of the [ability-mechanics telemetry plan](../../../docs/analysis/ability-mechanics/lab-uat-and-telemetry.md). How to read one cast with them: [ability-system.md, "Reading one cast"](../../../docs/gameplay/ability-system.md#reading-one-cast).

These files are the reviewable source of the ability objects in SigNoz. They are not created automatically: someone imports them once, and re-imports after a change here. The older SigNoz exports (NPC AI, Black Market) live in [docs/operations/signoz/](../../../docs/operations/signoz/) and use the same JSON shapes.

| File | Kind | Name in SigNoz |
|---|---|---|
| [cast-forensics.view.json](cast-forensics.view.json) | Logs Explorer saved view | Abilities — One cast, in order (add cast_id = N AND player_id = P) |
| [refusals.view.json](refusals.view.json) | Logs Explorer saved view | Abilities — Refusals by reason |
| [wire-sends.view.json](wire-sends.view.json) | Logs Explorer saved view | Abilities — Wire sends for a player (add player_id = P) |
| [ability-metrics.dashboard.json](ability-metrics.dashboard.json) | Dashboard (`v5`) | Cimmeria — Ability metrics |

## Importing

**Views.** Each view file is the body of one SigNoz MCP `signoz_create_view` call (`name`, `sourcePage`, `category`, `tags`, `compositeQuery`, `extraData`); pass it as is. In the UI: open the Logs Explorer, paste the `filter.expression` into the search bar, pick the columns listed in `extraData`, set the order (the forensics and wire views sort by `timestamp` ascending, oldest first), then **Save view** under the category `abilities`.

**Dashboard.** **Dashboards → New dashboard → Import JSON** and paste the file, or pass its `title`, `description`, `tags`, `layout` and `widgets` to `signoz_create_dashboard`. SigNoz assigns new query ids on import; that is expected.

After an import, record the object ids SigNoz returned in this table's PR or in the ledger row, as the NPC AI export does.

## What to check on the first import

- **`key not found`.** SigNoz refuses a filter on an attribute it has never ingested. `cast_id` (AB-T1), `mercury_seq` (AB-T2) and the `abilities.wire` fields (AB-T4) exist only after a build carrying them has run a session on that SigNoz. If a view's filter is refused, run one ability session on the new build first.
- **Histogram panels.** The four latency and amount panels (`press-to-fire-p50/p95`, `damage-p95`, `heal-p95`) use `spaceAggregation` `p50` / `p95` on the OTel explicit-bucket histograms. If SigNoz shows them empty while the counters have data, open each panel and re-pick the metric: SigNoz sometimes stores a histogram's type only once the first sample arrives.
- **Metric names.** Every metric is listed in `crates/cell-combat/src/cell/abilities/metrics/mod.rs` with its labels. A metric whose code has not run yet shows "No data", not an error.

## The views

Every filter starts with `service.name = 'cimmeria-server'`. The Rust log `target` is SigNoz's `scope_name`.

- **One cast, in order.** Every server row of every ability scope (`abilities`, `abilities.*`, `base.entity_method`) that carries a `cast_id`, oldest first. Add `cast_id = N AND player_id = P`. `cast_id` is the caster's own counter, not global, so always pair it with the caster (`player_id`, or `entity_id` for an NPC caster) and a time range around the cast. A wire row names the entity the method is about: a target's `onStatUpdate` carries the cast's `cast_id` but the target's `player_id`, so add `OR entity_id = <target>` to see it.
- **Refusals by reason.** The launch's refusal rows: the `use_ability_*` gate rows (AB-T2), every `*_refused` row and the deployables' `deploy_refused`. The `reason` column is the same value as the `reason` label of `abilities_refused_total`, so the dashboard's refusal panel and this list agree.
- **Wire sends for a player.** `abilities.wire` (`wire_sent`, `wire_send_failed`, the base's `client_sent`) and `base.entity_method` (`client_send_dropped`). Add `player_id = P` for sends about that player, or `witness_player_ids CONTAINS 'P'` for fan-outs they watched.

The client's side of a cast is in another service: `service.name = 'cimmeria-client'`, `client_target LIKE 'client.ability.%'`. Its fields are in the `fields` JSON attribute, so filter it with `fields CONTAINS '"cast_id":N'` (receive rows) or join the press through `mercury_seq` as [Reading one cast](../../../docs/gameplay/ability-system.md#reading-one-cast) describes.
