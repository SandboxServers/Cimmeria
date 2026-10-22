# Interaction pose queries and exports (SigNoz / ClickHouse)

> Type: how-to with reference tables. Audience: engineers and packet workers who turn human playtest poses into lab spec lines.
> Updated: 2026-10-18 (PC-05). Companions: [target catalog entry](../../architecture/observability-target-catalog.md#interactionpose-and-interactionapproach) (the fields), [client camera telemetry](../../architecture/client-camera-telemetry.md) (the camera join), [SigNoz deployment](../signoz-deployment.md) (access), [automated UAT guide](../../guides/automated-uat.md) (the spec line).

**Tested versus untested.** The field names below are read from the merged emitter (`crates/cell-world/src/cell/interaction_pose/emit.rs`) and from the lab-cli reader. No query on this page has run against a live SigNoz yet, and no `interaction.pose` row from a human session has been seen. The first live proof is PC-19. Treat every query as a draft until PC-19 passes, and fix the page, not the data, when one fails.

This page is a recipe book, not a saved-view export: nothing here is imported into a SigNoz. Run the queries in the SigNoz logs explorer or through ClickHouse.

## Ground rules

- Pose rows are server logs: `resources_string['service.name'] = 'cimmeria-server'`, `scope_name = 'interaction.pose'` or `'interaction.approach'`.
- **Humans only** means `attributes_string['cimmeria.session_kind'] = 'player'`. Lab sessions write `lab`. A row with no kind is neither; leave it out.
- Numbers sit in `attributes_number`, text in `attributes_string`. Wrap an id read in `toInt64()`, or ClickHouse refuses the read.
- A pose row carries no character or account name (D-PC2). Group by `player_id` when you need "per player".
- `bearing_deg` is circular: 359 and 1 are 2 degrees apart. **Never take a median or mean of bearings across the 0/360 line.** Use the circular mean (the angle of the summed unit vectors); `lab pose-library` (PC-20) does it for you. The other fields are plain numbers.

## Human poses per target

Median and spread of where humans stand, per world and spawn tag.

```sql
SELECT
  attributes_string['world']      AS world,
  attributes_string['target_tag'] AS target_tag,
  count()                         AS n,
  count(DISTINCT toInt64(attributes_number['player_id'])) AS players,
  quantiles(0.1, 0.5, 0.9)(attributes_number['radius_m'])          AS radius_p10_p50_p90,
  quantiles(0.1, 0.5, 0.9)(attributes_number['dy_m'])              AS dy_p10_p50_p90,
  quantiles(0.1, 0.5, 0.9)(attributes_number['facing_error_deg'])  AS facing_err_p10_p50_p90
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'cimmeria-server'
  AND scope_name = 'interaction.pose'
  AND attributes_string['cimmeria.session_kind'] = 'player'
  AND timestamp > toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000000000
GROUP BY world, target_tag
ORDER BY n DESC
LIMIT 50
```

Read it with care:

- A tag with `players = 1` is one person's habit, not a distribution.
- `facing_error_deg` is the angle between the pawn's body facing and the target. The pawn is not the camera ([client-camera-telemetry.md](../../architecture/client-camera-telemetry.md#what-the-s-cam-verdict-means)), so a large error is normal when a player looks at the target while strafing.
- Rows with `route = 'fallback'` are interactions no handler claimed. Add `AND attributes_string['route'] != 'fallback'` to drop them.

## Lab against human for one target

Same shape, grouped by session kind, for one tag. It shows whether the lab's hand-measured pose sits inside the human spread.

```sql
SELECT
  attributes_string['cimmeria.session_kind'] AS kind,
  count() AS n,
  quantiles(0.1, 0.5, 0.9)(attributes_number['radius_m']) AS radius_p10_p50_p90,
  quantiles(0.1, 0.5, 0.9)(attributes_number['dy_m'])     AS dy_p10_p50_p90
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'cimmeria-server'
  AND scope_name = 'interaction.pose'
  AND attributes_string['target_tag'] = 'ArmYourself_FrostBody'
GROUP BY kind
ORDER BY kind
```

## Hostile clicks and nearby hostiles

A right-click on a living hostile NPC is an interaction too. Its pose row carries `route = 'hostile_attack'` (the click is rerouted to combat, so the other routes never apply to it). The same pose row also gets one `interaction_hostiles` row when a living hostile stands within 20 m of the player, listing the three nearest, nearest first. It carries the pose row's `pose_seq` and is written by the same actor's pose throttle, so it is never written without its pose row. The target itself, dead NPCs and non-hostile NPCs are left out. A pose with no hostile nearby has no `interaction_hostiles` row.

| Field | Meaning |
|---|---|
| `event` | `interaction_hostiles` |
| `pose_seq` | The pose row's `pose_seq`, the join key |
| `entity_id` | The acting player's entity id |
| `count` | How many hostiles are listed, 1 to 3 |
| `hostile_1_id`, `hostile_2_id`, `hostile_3_id` | The hostile's entity id, absent past `count` |
| `hostile_N_dist_m` | Horizontal distance from the player, in metres |
| `hostile_N_bearing_deg` | `atan2(pz - tz, px - tx)` from the hostile to the player, in `[0, 360)` |

```sql
SELECT
  p.timestamp                                   AS t,
  toInt64(p.attributes_number['pose_seq'])      AS pose_seq,
  p.attributes_number['radius_m']               AS radius_m,
  h.attributes_number['count']                  AS hostile_count,
  h.attributes_number['hostile_1_id']           AS hostile_1_id,
  h.attributes_number['hostile_1_dist_m']       AS hostile_1_dist_m,
  h.attributes_number['hostile_1_bearing_deg']  AS hostile_1_bearing_deg
FROM signoz_logs.distributed_logs_v2 AS p
INNER JOIN signoz_logs.distributed_logs_v2 AS h
  ON toInt64(h.attributes_number['pose_seq']) = toInt64(p.attributes_number['pose_seq'])
 AND h.resources_string['service.name'] = 'cimmeria-server'
 AND h.scope_name = 'interaction.pose'
 AND h.attributes_string['event'] = 'interaction_hostiles'
WHERE p.resources_string['service.name'] = 'cimmeria-server'
  AND p.scope_name = 'interaction.pose'
  AND p.attributes_string['route'] = 'hostile_attack'
  AND p.timestamp > toUnixTimestamp(now() - INTERVAL 1 DAY) * 1000000000
  AND h.timestamp > toUnixTimestamp(now() - INTERVAL 1 DAY) * 1000000000
  AND toInt64(h.timestamp) - toInt64(p.timestamp) BETWEEN 0 AND 5000000000
ORDER BY t DESC
LIMIT 50
```

Each `pose_seq` restarts at 1 with the server, so narrow the window to one server run if the join matches across a restart.

## Approach trails

The approach row holds the last few positions before the interaction. Join it to its pose row on `pose_seq`, which is process-global and restarts at 1 with the server, so also bound the time (the approach row is written right after its pose row; the bound sits in `WHERE` because a ClickHouse join condition between the two sides is not available on every version):

```sql
SELECT
  p.timestamp                                   AS t,
  toInt64(p.attributes_number['pose_seq'])      AS pose_seq,
  p.attributes_string['target_tag']             AS target_tag,
  p.attributes_number['radius_m']               AS radius_m,
  a.attributes_number['n_points']               AS n_points,
  a.attributes_number['path_len_m']             AS path_len_m,
  a.attributes_string['points']                 AS points
FROM signoz_logs.distributed_logs_v2 AS p
INNER JOIN signoz_logs.distributed_logs_v2 AS a
  ON toInt64(a.attributes_number['pose_seq']) = toInt64(p.attributes_number['pose_seq'])
 AND a.resources_string['service.name'] = 'cimmeria-server'
 AND a.scope_name = 'interaction.approach'
WHERE p.resources_string['service.name'] = 'cimmeria-server'
  AND p.scope_name = 'interaction.pose'
  AND p.attributes_string['cimmeria.session_kind'] = 'player'
  AND p.timestamp > toUnixTimestamp(now() - INTERVAL 1 DAY) * 1000000000
  AND a.timestamp > toUnixTimestamp(now() - INTERVAL 1 DAY) * 1000000000
  AND toInt64(a.timestamp) - toInt64(p.timestamp) BETWEEN 0 AND 5000000000
ORDER BY t DESC
LIMIT 50
```

`points` is `x,y,z,yaw_deg,age_s;...`, oldest first, `age_s` being seconds before the interaction. A pose row with no approach row simply had a trail shorter than two points. If the join returns nothing across a restart boundary, narrow the window to one server run.

## From a pose to a spec line

A spec `pose` line in [automated-uat.md](../../guides/automated-uat.md#targets-poses-and-waits) names a target by spawn tag and gives a stand-and-aim recipe. Take the Frost line from `first-session.toml` (FS-P3):

```toml
{ id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0, calibrated = "2026-10-10 hand" }
```

Where each key would come from, once human rows exist:

| Spec key | Source | Source row |
|---|---|---|
| `tag` | `target_tag` | `interaction.pose` |
| `radius_m`, `dy_m` | median of the field | `interaction.pose`, humans only |
| `bearing_deg` | circular mean of the field | `interaction.pose`, humans only |
| `pitch_deg` | `pitch_offset_deg` | camera row, joined by [the pose join](../../architecture/client-camera-telemetry.md#joining-the-camera-to-interactionpose) |
| `zoom` | `zoom` | camera row |
| `yaw_offset_deg` | `atan2(tx - cx, tz - cz)` in degrees, minus `camera_yaw_deg` | camera row plus the derived target position |

`lab pose-library` (wave 4, not yet shipped when this page was written) reads the exports below and does the aggregation and the join. Until it ships, the table is the manual route. The spec line's `calibrated` stamp should then say where the numbers came from.

## Export for the lab tools (D-PC15)

The lab tools read flat JSONL: one object per line, keyed by the tracing field name plus `timestamp`. They accept numbers as numbers or as strings, and treat `null` as absent. ClickHouse writes that with `FORMAT JSONEachRow`. Name the columns, so an absent value stays `null` instead of `0` or `''`. Two helpers, written inline:

- a number: `if(mapContains(attributes_number, 'k'), attributes_number['k'], NULL) AS k`
- a text: `nullIf(attributes_string['k'], '') AS k`

**The `timestamp` key is integer milliseconds.** The tools read `timestamp` as integer milliseconds or RFC 3339, never as nanoseconds, and the column is nanoseconds. Each query below therefore reads the raw column as `t` in an inner query and writes `intDiv(t, 1000000) AS timestamp` outside, because an alias named `timestamp` would shadow the column in the time filter. `SETTINGS output_format_json_quote_64bit_integers = 0` keeps that integer unquoted.

For access to the ClickHouse container, use the recipe in the [telemetry-triage skill](../../../.claude/skills/telemetry-triage/SKILL.md) and [signoz-deployment.md](../signoz-deployment.md); put your own SSH alias in, and commit no host. Pipe the result to a `.jsonl` file. In PowerShell, write it as UTF-8 (a leading BOM is tolerated).

### 1. Pose rows

```sql
SELECT
  intDiv(t, 1000000) AS timestamp,
  nullIf(attributes_string['kind'], '') AS kind,
  nullIf(attributes_string['route'], '') AS route,
  nullIf(attributes_string['cimmeria.session_kind'], '') AS `cimmeria.session_kind`,
  nullIf(attributes_string['world'], '') AS world,
  nullIf(attributes_string['target_tag'], '') AS target_tag,
  nullIf(attributes_string['target_template_name'], '') AS target_template_name,
  if(mapContains(attributes_number, 'entity_id'), attributes_number['entity_id'], NULL) AS entity_id,
  if(mapContains(attributes_number, 'player_id'), attributes_number['player_id'], NULL) AS player_id,
  if(mapContains(attributes_number, 'target_entity_id'), attributes_number['target_entity_id'], NULL) AS target_entity_id,
  if(mapContains(attributes_number, 'target_template_id'), attributes_number['target_template_id'], NULL) AS target_template_id,
  if(mapContains(attributes_number, 'player_x'), attributes_number['player_x'], NULL) AS player_x,
  if(mapContains(attributes_number, 'player_y'), attributes_number['player_y'], NULL) AS player_y,
  if(mapContains(attributes_number, 'player_z'), attributes_number['player_z'], NULL) AS player_z,
  if(mapContains(attributes_number, 'player_yaw_deg'), attributes_number['player_yaw_deg'], NULL) AS player_yaw_deg,
  if(mapContains(attributes_number, 'wire_pitch_deg'), attributes_number['wire_pitch_deg'], NULL) AS wire_pitch_deg,
  if(mapContains(attributes_number, 'radius_m'), attributes_number['radius_m'], NULL) AS radius_m,
  if(mapContains(attributes_number, 'bearing_deg'), attributes_number['bearing_deg'], NULL) AS bearing_deg,
  if(mapContains(attributes_number, 'dy_m'), attributes_number['dy_m'], NULL) AS dy_m,
  if(mapContains(attributes_number, 'facing_error_deg'), attributes_number['facing_error_deg'], NULL) AS facing_error_deg,
  if(mapContains(attributes_number, 'pose_seq'), attributes_number['pose_seq'], NULL) AS pose_seq,
  if(mapContains(attributes_number, 'mercury_seq'), attributes_number['mercury_seq'], NULL) AS mercury_seq
FROM (
  SELECT timestamp AS t, attributes_string, attributes_number
  FROM signoz_logs.distributed_logs_v2
  WHERE resources_string['service.name'] = 'cimmeria-server'
    AND scope_name = 'interaction.pose'
    AND timestamp > toUnixTimestamp(now() - INTERVAL 14 DAY) * 1000000000
)
ORDER BY t
SETTINGS output_format_json_quote_64bit_integers = 0
FORMAT JSONEachRow
```

Keep the column list in step with the [field list](../../architecture/observability-target-catalog.md#interactionpose-and-interactionapproach); add `space_id`, `world_id`, `dialog_*` and `item_*` columns the same way if a tool needs them.

### 2. Approach rows

```sql
SELECT
  intDiv(t, 1000000) AS timestamp,
  nullIf(attributes_string['cimmeria.session_kind'], '') AS `cimmeria.session_kind`,
  nullIf(attributes_string['world'], '') AS world,
  nullIf(attributes_string['target_tag'], '') AS target_tag,
  nullIf(attributes_string['points'], '') AS points,
  if(mapContains(attributes_number, 'pose_seq'), attributes_number['pose_seq'], NULL) AS pose_seq,
  if(mapContains(attributes_number, 'entity_id'), attributes_number['entity_id'], NULL) AS entity_id,
  if(mapContains(attributes_number, 'player_id'), attributes_number['player_id'], NULL) AS player_id,
  if(mapContains(attributes_number, 'target_entity_id'), attributes_number['target_entity_id'], NULL) AS target_entity_id,
  if(mapContains(attributes_number, 'n_points'), attributes_number['n_points'], NULL) AS n_points,
  if(mapContains(attributes_number, 'span_s'), attributes_number['span_s'], NULL) AS span_s,
  if(mapContains(attributes_number, 'path_len_m'), attributes_number['path_len_m'], NULL) AS path_len_m
FROM (
  SELECT timestamp AS t, attributes_string, attributes_number
  FROM signoz_logs.distributed_logs_v2
  WHERE resources_string['service.name'] = 'cimmeria-server'
    AND scope_name = 'interaction.approach'
    AND timestamp > toUnixTimestamp(now() - INTERVAL 14 DAY) * 1000000000
)
ORDER BY t
SETTINGS output_format_json_quote_64bit_integers = 0
FORMAT JSONEachRow
```

### 3. Friction rows

For `lab friction-drafts`. The reader needs `signal`, `reason`, `world`, `timestamp`, `player_id` and `cimmeria.session_kind`, and drops every row whose kind is not `player` (D-PC1). It groups rows by signal, reason, world and a subject: the first present of `tag`, `template_name`, `item_id`, `step_id`, `mission_id`, `region_tag` and `dialog_id`. Leave the subject columns out and every row of a signal falls into one cluster, so the query carries all seven. The friction rows also carry account and character names; this query does not select them, and the reader drops them again from its samples.

```sql
SELECT
  intDiv(t, 1000000) AS timestamp,
  nullIf(attributes_string['signal'], '') AS signal,
  nullIf(attributes_string['reason'], '') AS reason,
  nullIf(attributes_string['world'], '') AS world,
  nullIf(attributes_string['cimmeria.session_kind'], '') AS `cimmeria.session_kind`,
  nullIf(attributes_string['tag'], '') AS tag,
  nullIf(attributes_string['template_name'], '') AS template_name,
  nullIf(attributes_string['region_tag'], '') AS region_tag,
  if(mapContains(attributes_number, 'player_id'), attributes_number['player_id'], NULL) AS player_id,
  if(mapContains(attributes_number, 'item_id'), attributes_number['item_id'], NULL) AS item_id,
  if(mapContains(attributes_number, 'step_id'), attributes_number['step_id'], NULL) AS step_id,
  if(mapContains(attributes_number, 'mission_id'), attributes_number['mission_id'], NULL) AS mission_id,
  if(mapContains(attributes_number, 'dialog_id'), attributes_number['dialog_id'], NULL) AS dialog_id
FROM (
  SELECT timestamp AS t, attributes_string, attributes_number
  FROM signoz_logs.distributed_logs_v2
  WHERE resources_string['service.name'] = 'cimmeria-server'
    AND scope_name = 'playtest.friction'
    AND timestamp > toUnixTimestamp(now() - INTERVAL 14 DAY) * 1000000000
)
ORDER BY t
SETTINGS output_format_json_quote_64bit_integers = 0
FORMAT JSONEachRow
```

### 4. Camera rows

For `lab pose-library --camera`: both `client.camera.at_interact` and `client.camera.sent_seq`, which carry the keys the sequence join needs. These are client telemetry, replayed by admin-api under the `client.native` target and indexed under the `cimmeria-client` service. The client's own target is the attribute `client_target`. A camera row's `timestamp` is when admin-api replayed the upload, not when the player clicked; the join keys are `server_ts_ms` and `ts_ms`, so they are exported as they are.

```sql
SELECT
  intDiv(t, 1000000) AS timestamp,
  nullIf(attributes_string['client_target'], '') AS client_target,
  nullIf(attributes_string['session_id'], '') AS session_id,
  nullIf(attributes_string['cimmeria.session_kind'], '') AS `cimmeria.session_kind`,
  nullIf(attributes_string['method'], '') AS method,
  if(mapContains(attributes_number, 'ts_ms'), attributes_number['ts_ms'], NULL) AS ts_ms,
  if(mapContains(attributes_number, 'server_ts_ms'), attributes_number['server_ts_ms'], NULL) AS server_ts_ms,
  if(mapContains(attributes_number, 'clock_offset_ms'), attributes_number['clock_offset_ms'], NULL) AS clock_offset_ms,
  if(mapContains(attributes_number, 'entity_id'), attributes_number['entity_id'], NULL) AS entity_id,
  if(mapContains(attributes_number, 'send_id'), attributes_number['send_id'], NULL) AS send_id,
  if(mapContains(attributes_number, 'zoom'), attributes_number['zoom'], NULL) AS zoom,
  if(mapContains(attributes_number, 'yaw_offset_deg'), attributes_number['yaw_offset_deg'], NULL) AS yaw_offset_deg,
  if(mapContains(attributes_number, 'pitch_offset_deg'), attributes_number['pitch_offset_deg'], NULL) AS pitch_offset_deg,
  if(mapContains(attributes_number, 'camera_x'), attributes_number['camera_x'], NULL) AS camera_x,
  if(mapContains(attributes_number, 'camera_y'), attributes_number['camera_y'], NULL) AS camera_y,
  if(mapContains(attributes_number, 'camera_z'), attributes_number['camera_z'], NULL) AS camera_z,
  if(mapContains(attributes_number, 'camera_yaw_deg'), attributes_number['camera_yaw_deg'], NULL) AS camera_yaw_deg,
  if(mapContains(attributes_number, 'mercury_seq_first'), attributes_number['mercury_seq_first'], NULL) AS mercury_seq_first,
  if(mapContains(attributes_number, 'mercury_seq_last'), attributes_number['mercury_seq_last'], NULL) AS mercury_seq_last
FROM (
  SELECT timestamp AS t, attributes_string, attributes_number
  FROM signoz_logs.distributed_logs_v2
  WHERE resources_string['service.name'] = 'cimmeria-client'
    AND scope_name = 'client.native'
    AND attributes_string['client_target'] LIKE 'client.camera.%'
    AND timestamp > toUnixTimestamp(now() - INTERVAL 14 DAY) * 1000000000
)
ORDER BY t
SETTINGS output_format_json_quote_64bit_integers = 0
FORMAT JSONEachRow
```

Check before you trust it: run `SELECT DISTINCT scope_name, resources_string['service.name'] ... LIMIT 20` on a day with a camera upload. If the rows sit under another service name, change the `WHERE`, not the column list. The `pawn_*` columns are left out on purpose; add them the same way if you want the pawn pose beside the camera.

## Heading age at interact

The `interact` INFO row carries `heading_age_ms`: milliseconds since the cell processed the player's last position update, so it says how old the server's heading is at the click. `-1` means no position update has been seen yet. Standing players send a position about every 3 s, so an age above 1,000 means the server heading may lag the client's turn.

## Friction is DEBUG now (D-PC11)

`playtest.friction` is exported at DEBUG through its own `OTEL_FILTER` row (`crates/server/src/logging/filters.rs`). A saved view or alert that filters it on WARN severity shows nothing. Filter on `scope_name = 'playtest.friction'` and drop the severity condition. The owner updates the live "Playtest: friction" saved view; this repository does not hold its export, and no worker edits it.

## Mouse navigation

The nav poll's rows (`client.input.button`, `client.input.mode`, `client.nav.pose`) are client telemetry replayed by admin-api. Each row has `resources_string['service.name'] = 'cimmeria-client'`, `scope_name = 'client.native'`, the event name in `attributes_string['client_target']` and the session in `attributes_string['session_id']`. Only a few correlation keys are lifted to attributes (`entity_id` is one, on `client.input.hover`). Every other field, `button`, `edge`, `class`, `hover_id`, `mode`, `ui_owned` and the rest, is in the JSON text of `attributes_string['fields']`, so the queries below read it with `JSONExtract*`. What each field means is in [client-telemetry.md](../../architecture/client-telemetry.md#mouse-input-clientinput).

Not run against a live SigNoz yet. The first live proof is LN-13 (live confirmation in the lab), and the owner's mouse session is listed in [unified-uat.md](../../guides/unified-uat.md) once it lands.

### One session in time order

Replace `<session_id>` with the session's id. The row timestamp is the replay time; the order is the order the poll saw the rows within one session.

```sql
WITH attributes_string['fields'] AS f
SELECT
  timestamp                                     AS t,
  attributes_string['client_target']            AS target,
  JSONExtractString(f, 'button')                AS button,
  JSONExtractString(f, 'edge')                  AS edge,
  JSONExtractString(f, 'class')                 AS class,
  JSONExtractRaw(f, 'hover_id')                 AS hover_id,
  JSONExtractRaw(f, 'mode')                     AS mode,
  JSONExtractRaw(f, 'buttons')                  AS buttons,
  JSONExtractRaw(f, 'ui_owned')                 AS ui_owned
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'cimmeria-client'
  AND scope_name = 'client.native'
  AND attributes_string['session_id'] = '<session_id>'
  AND attributes_string['client_target'] IN ('client.input.button', 'client.input.mode', 'client.nav.pose')
  AND timestamp > toUnixTimestamp(now() - INTERVAL 1 DAY) * 1000000000
ORDER BY t
LIMIT 500
```

`JSONExtractRaw` returns an empty string for an absent field, so an empty `ui_owned` means unknown, not `false`. `hover_id` is only on select and interact rows, so an empty value there is normal.

### Clicks by class and hover id

Counts releases (`edge = 'up'`) by `class` and `hover_id`, for human sessions. A `hover_id` of `0` covers both "absent" (drag and chord releases) and "zero". The id is the hover id at release and approximates the picked target (see the architecture page).

```sql
WITH attributes_string['fields'] AS f
SELECT
  JSONExtractString(f, 'class') AS class,
  JSONExtractInt(f, 'hover_id') AS hover_id,
  count()                       AS clicks
FROM signoz_logs.distributed_logs_v2
WHERE resources_string['service.name'] = 'cimmeria-client'
  AND scope_name = 'client.native'
  AND attributes_string['client_target'] = 'client.input.button'
  AND attributes_string['cimmeria.session_kind'] = 'player'
  AND JSONExtractString(f, 'edge') = 'up'
  AND timestamp > toUnixTimestamp(now() - INTERVAL 7 DAY) * 1000000000
GROUP BY class, hover_id
ORDER BY clicks DESC
LIMIT 50
```

Read it with care: `hover_id` is approximate, so a count per id is a hint about where the player clicked, not proof of the picked entity. Drop the `class` column to see the totals per id alone.

## Documentation debt

- No saved-view export exists for these queries. Add one only after PC-19 shows the queries work.
- The `lab pose-library` section above describes tools that wave 4 builds; re-check it when they merge.
