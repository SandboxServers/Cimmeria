# Cooked-data presync telemetry

> **Type:** Reference
> **Audience:** Operators and engineers reading the launcher presync's server rows in SigNoz
> **Last updated:** 2026-10-26
> **Companions:** [target catalog](observability-target-catalog.md#cooked_presync), [cooked-data pipeline § Launcher presync](../engine/cooked-data-pipeline.md#launcher-presync), [telemetry operations § Cooked-data presync](../operations/telemetry.md#cooked-data-presync)

**Server rows only.** The launcher's own rows (`cooked_presync_category`, `cooked_presync_finished`) are described with the launcher's telemetry, not here.

**Emitted from:** `crates/base/src/base/presync_catalog.rs` (the build and its outcome) and `crates/admin-api/src/routes/cooked/handlers.rs` (the two routes), for the launcher presync ([cooked-data-pipeline.md § Launcher presync](../engine/cooked-data-pipeline.md#launcher-presync)). The routes are anonymous and no row or label carries an address, a path or a name the client sent.

| Name | Kind | Level | Fields or labels |
|---|---|---|---|
| `cooked_presync.catalog_ready` | log row | INFO | `categories`, `body_bytes`, `duration_ms`. Once per process, when the catalog is built and published. |
| `cooked_presync.catalog_unavailable` | log row | WARN | `reason` (`raw_pak_unreadable`, `category_too_large`, `build_failed`), `category_id` when one category failed the build. Once per process. Every launcher on the server now gets `catalog_failed`. |
| `cooked_presync.served` | log row | INFO | `category_id`, `category_name`, `version`, `bytes`, `changed`. One per category body served. |
| `cooked_presync.index_served` | log row | DEBUG | `categories`. One per index served. |
| `cooked_presync.refused` | log row | INFO | `route` (`index` or `category`), `reason` (the JSON error code), `status`, `category_id` (0 on the index route and when the id did not parse). |
| `cooked_presync_requests_total` | counter | | `route` = `index` or `category`; `outcome` = `served` or the refusal code (`rate_limited`, `disabled`, `catalog_unavailable`, `catalog_failed`, `unknown_category`, `version_changed`, `bad_request`). |
| `cooked_data_version_replies_total` | counter | | `outcome` (`no_server_data`, `up_to_date`, `full_resync`), `reason` (`category_not_served`, `versions_match`, `version_mismatch`), `category` (the category name, or `unknown`). One per `versionInfoRequest`, next to the existing `cooked_data.version_reply` row. |

The rows have no explicit `target:`; filter on the `event` field. Alert on `cooked_presync_requests_total{outcome="catalog_failed"}` above 0 or on the `catalog_unavailable` WARN: either means every launcher on that server is blocked. Query recipes: [telemetry.md § Cooked-data presync](../operations/telemetry.md#cooked-data-presync).
