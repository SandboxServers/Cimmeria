# NPC AI saved views and dashboard (SigNoz export)

> Type: reference. Audience: operators and packet workers who need to rebuild or audit the NPC AI SigNoz objects.
> Updated: 2026-09-25 (NA03). Companions: [NPC AI telemetry runbook](../npc-ai-telemetry-runbook.md) (how to use these), [observability ADR](../../architecture/observability.md) (target catalog and metric names), [SigNoz deployment](../signoz-deployment.md), [NPC AI telemetry plan](../../analysis/npc-ai-restoration/telemetry.md).

This page is the source of truth for the NPC AI objects in the colo SigNoz. If the SigNoz volume is lost or you stand up a second SigNoz, recreate them from here.

## Dashboard

| Field | Value |
|---|---|
| Title | `Cimmeria — NPC AI health` |
| Id on the colo SigNoz | `01a0d755-9145-7eed-aedf-29d0efa28e1e` |
| Export | [npc-ai-health.dashboard.json](npc-ai-health.dashboard.json) (SigNoz dashboard `v5` JSON) |
| Tags | `cimmeria`, `npc-ai`, `na03` |

To re-import, open **Dashboards → New dashboard → Import JSON** in the SigNoz UI and paste the file, or pass its `title`, `description`, `tags`, `layout` and `widgets` to the SigNoz MCP tool `signoz_create_dashboard`. SigNoz assigns new query ids on import. That is expected.

### Panels

Every metric panel filters on `service.name = 'cimmeria-server'`. Counters use `increase` over a 60 s step, summed across series.

| Panel id | Title | Source | Group by |
|---|---|---|---|
| `aggro-by-cause` | Aggro by cause | metric `npc_ai_aggro_total` | `cause`, `world` |
| `transitions-by-reason` | AI state transitions by reason | metric `npc_ai_transitions_total` | `reason` |
| `detectors-per-world` | Stuck / float / leash detectors per world | metrics `npc_stale_velocity_total`, `npc_ground_deviation_total`, `npc_stuck_total`, `npc_off_mesh_total`, `npc_idle_parked_total`, `npc_leash_loop_total`, `npc_threat_cleared_without_exit_total`, `npc_spawn_off_mesh_total` | `world` |
| `detector-totals` | Detector totals over the range, per world (table) | the first seven metrics above, reduced to `sum` | `world` |
| `path-requests-by-status` | Path request status mix | metric `npc_path_requests_total` | `status` |
| `path-fail-and-partial` | Path failures by reason + partial corridors | metrics `npc_path_fail_total` (by `reason`) and `npc_path_partial_total` (by `state`) | as named |
| `idle-unticked` | Idle NPCs the AI tick skips | up/down gauge `npc_ai_idle_unticked`, time aggregation `avg` | `world` |
| `decisions-by-outcome` | AI tick decisions by outcome | metric `npc_ai_decisions_total` | `decision_outcome` |
| `cover-coverage` | Cover coverage per space (log list) | logs, `scope_name = 'cover.coverage'` | none; columns `space_id`, `world_id`, `nodes_in_world`, `nodes_on_mesh`, `sets_in_world`, `cover_npcs`, `reason` |
| `no-cover-reasons` | no_cover reasons (log count table) | logs, `scope_name = 'npc_ai' AND decision_outcome = 'no_cover'` | `reason`, `world` |
| `aggro-scan-rejections` | Aggro-scan rejections by reason (log count table) | logs, `scope_name = 'npc_ai.aggro_scan' AND event = 'candidate_rejected'` | `reason`, `world` |

Cover has no metric, so its two panels are log-based. The `no_cover` and `aggro_scan` rows are sampled (at most one per NPC per 10 s, or one per NPC and player per 10 s), so read their counts as "NPC-windows that hit this reason", not as ticks.

### Which metrics had data when the dashboard was built

On 2026-09-25 the colo SigNoz knew three `npc_*` metrics: `npc_ai_decisions_total`, `npc_path_fail_total` (reasons `no_path` and `degenerate_path` only, the pre-NA02 set) and `npc_respawns_total`. Every other metric above ships in NA00 (#776) and NA02 (#781), which were not deployed yet. Those panels read "No data" until the first session on a build that carries them. That is not a broken panel.

## Saved Logs Explorer views

All ten live in the Logs Explorer under the category `npc-ai`, tagged `npc-ai`. Each one maps to one question in the [runbook](../npc-ai-telemetry-runbook.md#which-view-answers-which-question). In SigNoz the Rust log `target` is the `scope_name` column.

Every filter below starts with `service.name = 'cimmeria-server' AND`, which is omitted from the table for width.

| Name | Id | Filter (after the service clause) | Columns |
|---|---|---|---|
| NPC AI — Which NPCs aggroed, and how? | `01a0d755-e0f3-7c32-aa8d-a96864ca4a8d` | `scope_name = 'npc_ai.aggro'` | `cause, tag, npc_id, world, from, target_id, player_id, npc_to_target, dy, has_los, aggression, body` |
| NPC AI — Why did this guard ignore me? | `01a0d755-eed1-78d2-878b-aa989046cb97` | `scope_name IN ('npc_ai.aggro_scan', 'npc_ai.idle')` | `scope_name, event, reason, npc_id, tag, world, player_id, npc_to_target, dy, aggro_radius, witness_count, rejected, body` |
| NPC AI — Timeline for one NPC (add npc_id = N) | `01a0d755-fea5-7b8c-b597-2f58bd73800f` | `scope_name IN ('npc_ai.transition', 'npc_ai.leash', 'npc_ai.aggro', 'npc_ai.path', 'npc_ai.path_fail', 'movement.npc')` | `scope_name, event, npc_id, tag, from, to, reason, status, cause, npc_to_spawn, npc_to_target, nav_path_len, threat_count, body` |
| NPC AI — Who is running in place? (stale_velocity) | `01a0d756-0d1b-798a-a742-5776c1788614` | `scope_name = 'movement.npc' AND event = 'stale_velocity'` | `npc_id, tag, world, path_state, ai_state, speed, vx, vy, vz, still_ticks, nav_path_len, movement_type, suppressed, body` |
| NPC AI — Who is floating? (ground_deviation) | `01a0d756-1b99-7381-a274-7732e7255b02` | `scope_name = 'movement.npc' AND event = 'ground_deviation'` | `npc_id, tag, world, dir, dy, ground_y, y_source, leg_len, leg_dy, wp_x, wp_y, wp_z, suppressed, body` |
| NPC AI — Who is stuck? (stuck, off-mesh, idle-parked, leash loop) | `01a0d756-2c2b-76c9-9d0c-162706f5b49c` | `((scope_name = 'npc_ai' AND event IN ('stuck', 'npc_off_mesh')) OR scope_name = 'npc_ai.idle_parked' OR (scope_name = 'npc_ai.leash' AND event = 'loop'))` | `scope_name, event, npc_id, tag, world, reason, gate, last_move_source, npc_to_target_history, npc_to_spawn, leash_count, nav_path_len, los, body` |
| NPC AI — Does this map have usable cover? (cover.coverage) | `01a0d756-387d-7b4a-96cd-91d63adaba7a` | `scope_name = 'cover.coverage'` | `severity_text, space_id, world_id, nodes_in_world, nodes_on_mesh, sets_in_world, cover_npcs, reason, body` |
| NPC AI — Why no cover in this fight? (no_cover + cover.selection) | `01a0d756-47c5-774a-bebf-28718c22e672` | `((scope_name = 'npc_ai' AND decision_outcome = 'no_cover') OR scope_name = 'cover.selection')` | `scope_name, event, npc_id, tag, world, reason, candidates_scanned, reserved_skipped, search_radius, cover_nodes_loaded, node_id, score, rank, body` |
| NPC AI — What did the client see? (add entity_id = N) | `01a0d756-54d3-7435-af97-955f8306f0e4` | `scope_name = 'wire.out.avatar_update'` | `entity_id, witness_id, npc_moved_since_last, x, y, z, vx, vy, vz, yaw_byte, pos_variant, body` |
| NPC AI — Why is this NPC holding fire? (add npc_id = N) | `01a0d77a-8c08-7d17-9df4-7c950fdeb9bd` | `scope_name IN ('npc_ai.tick', 'npc_ai.los')` | `scope_name, npc_id, tag, ai_state, decision_outcome, los, los_policy, result, dist_to_target, dy, hit_xyz, eye_height_used, body` |

**Why is this NPC holding fire?** was added by NA16 (2026-09-25), which is not in the telemetry plan. It is the only view that carries `npc_ai.tick`, which is one DEBUG row per NPC per AI tick, so always add `AND npc_id = N` and a short time range. `los` is the navmesh verdict, and `los_policy` is the attack rule that acted on it (`strict`, `stationary`, `stationary_relaxed`, `stationary_other_storey`; decision D-NA11). The Timeline view does not list these columns because none of its scopes emit them.

Two of these differ slightly from the query in the telemetry plan:

- **Timeline** adds `npc_ai.path_fail`, because a failed route is part of an NPC's story and the plan's list predates that target's NA02 reasons.
- **What did the client see?** does not filter on `npc_moved_since_last EXISTS`. SigNoz rejects a filter on a key it has never ingested (`key ... not found`), and no deployed build emitted that field when the view was made. Once an NA02 build has run a session, you can add `AND npc_moved_since_last EXISTS` to drop the player samples.

### Recreating a view

Each view is a Logs Explorer `list` query. The SigNoz MCP `signoz_create_view` call takes this shape (fill `expression` and the columns from the table):

```json
{
  "name": "NPC AI — Who is running in place? (stale_velocity)",
  "sourcePage": "logs",
  "category": "npc-ai",
  "tags": ["npc-ai", "movement"],
  "compositeQuery": {
    "queryType": "builder",
    "panelType": "list",
    "queries": [
      {
        "type": "builder_query",
        "spec": {
          "name": "A",
          "signal": "logs",
          "source": "",
          "stepInterval": 0,
          "filter": { "expression": "service.name = 'cimmeria-server' AND scope_name = 'movement.npc' AND event = 'stale_velocity'" },
          "having": { "expression": "" }
        }
      }
    ]
  },
  "extraData": "{\"selectColumns\":[{\"name\":\"npc_id\",\"signal\":\"logs\"},{\"name\":\"body\",\"signal\":\"logs\"}]}"
}
```

In the UI, paste the filter into the Logs Explorer search bar, add the columns, and use **Save view** with the name and category from the table.
