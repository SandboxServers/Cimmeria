# Client camera telemetry at interaction

> Type: reference, with the design reasons (explanation) kept to the scope section. Audience: engineers reading camera rows or joining them to server poses.
> Updated: 2026-10-18 (PC-18). Companions: [client-telemetry.md](client-telemetry.md) (the DLL), [observability-target-catalog.md](observability-target-catalog.md#interactionpose-and-interactionapproach) (the server pose row), [interaction-pose-views.md](../operations/signoz/interaction-pose-views.md) (queries and exports), [telemetry operations](../operations/telemetry.md) (the opt-in), [avatar-update-rotation-source.md](../reverse-engineering/findings/avatar-update-rotation-source.md) (the S-CAM finding).

**Tested versus untested.** The field names, offsets and join rules below are read from the merged code and covered by its unit tests: the pure decoders and field builder in `crates/client-telemetry/src/hooks/camera/`, the sequence-range rule in `hooks/ability_trace/seq_join.rs`, and the replay shape in `crates/admin-api/src/routes/telemetry/replay_camera.rs`. **Not yet verified live:** the DLL camera read in a running client, and the join of a `Channel::send` / `Nub::send` sequence range to the server's `mercury_seq`. PC-19 is the first live proof. Until it passes, read "exact" below as "exact by design".

## What is read, and when

The DLL reads the camera only when the client is about to send an `interact` or a `useItem` call. The read happens in the router detour, **before** the original router runs, so the numbers are the camera as the player clicked, not a frame later. Any other call reads nothing.

One `client.camera.at_interact` row carries:

| Part | Fields | Source |
|---|---|---|
| Call | `method` (`interact` \| `useItem`), `entity_id` (the local player), `send_id` | the routed call; `send_id` is the DLL's send counter |
| Camera block | `zoom`, `yaw_offset_deg`, `pitch_offset_deg`, `camera_flags`, `camera_gain` | the player camera's fields at `+0x348` through `+0x36c` |
| Camera actor pose | `camera_x`, `camera_y`, `camera_z`, `camera_yaw_deg` | the camera actor's location and rotator at `+0xdc` |
| Pawn pose | `pawn_x`, `pawn_y`, `pawn_z`, `pawn_yaw_deg`, `pawn_pitch_deg` | the local entity's pawn, under S-CAM A (below) |

Details that trip readers:

- `zoom` is the third-person distance in client units. `yaw_offset_deg` and `pitch_offset_deg` are the camera's offsets **from the pawn**, converted from rotator units (`units * 360 / 65536`) and not wrapped. They are what a lab spec calls `yaw_offset_deg` and `pitch_deg` ([automated-uat.md](../guides/automated-uat.md#targets-poses-and-waits)).
- `camera_flags` holds the invert bits (bit 2 inverts yaw, bit 3 inverts pitch). `camera_gain` is rotator units per mouse count.
- Positions are in **server units**: the client's `[x, y, z]` becomes `[y, z, x] / 100`. Yaws are degrees wrapped to `[0, 360)`.
- `camera_pose_zero` is `true` when the camera actor's location read is missing or all zero. Then no `camera_*` position keys are written, because a zero is not a position. A reader must not treat the absent keys as `(0, 0, 0)`.
- The offsets come from the lab's readers, proven live in [cegui-mouse-input-feed.md](../reverse-engineering/findings/cegui-mouse-input-feed.md) (section 10), and were copied unchanged. The camera pointer is found by scanning the level's actors for the player controller and is re-checked against the camera's vtable on every use, so a stale pointer reads as "no camera" and no row is written.
- The row is throttled per method and entity by the DLL's shared limiter. A throttled call writes no `at_interact` row.

## Scope and consent

The camera read is part of the existing launcher opt-in, **Send game diagnostics while playing** (see [telemetry operations](../operations/telemetry.md)). The owner ruled on 2026-10-18 that this opt-in covers the camera, so the launcher wording is unchanged and this page adds no new consent.

- **Off means off.** The launcher injects the DLL only while the opt-in is on, so with it off nothing here runs.
- **Ids only (D-PC2, D-PC14).** A camera row carries the entity id and no character name. The replay builds its own row shape for `client.camera.*` targets for this reason; the usual game shape would add names.
- **Memory reads only.** The DLL reads the client's memory and writes nothing to it.
- **Release gating.** The launcher refuses to inject a DLL whose SHA-256 differs from the one compiled into it, so a camera-capable DLL reaches player data only after a launcher release carries its hash.

## The rows

Both rows are DLL events (`client.camera.*`) that admin-api replays under the tracing target `client.native`, with `client_target` naming the DLL target. Both are always INFO, whatever level the DLL sent, and both land in the `cimmeria-client` index. Each carries `session_id`, `install_id`, `cimmeria.session_kind`, `ts_ms`, `seq`, `client_pid` and the raw `fields` bag besides the keys below.

**`client_target = 'client.camera.at_interact'`.** The keys in the table above, lifted out of the bag as numbers (a numeric string is accepted; `NaN` and infinities read as absent), plus:

- `server_ts_ms`: the row's time on the server clock (below).
- `clock_offset_ms`: the offset used for it.

**`client_target = 'client.camera.sent_seq'`.** Written by the network thread once the packets that carried a send are stamped. It maps one camera send to its Mercury sequence range:

- `method`, `send_id` (the same counter as the camera row);
- `mercury_seq_first`, `mercury_seq_last`, `packets`;
- `server_ts_ms`, `clock_offset_ms`.

It has no pose keys.

**The clock.** The DLL stamps every upload with an `X-Cimmeria-Client-Sent-Ms` header, its wall clock at send. For each chunk, admin-api computes the offset `server receive time - client sent time`, keeps the smallest offset seen in the last five minutes, and sets `server_ts_ms = ts_ms + offset`, capped at the chunk's receive time. The smallest offset is the least-delayed upload, so the error that remains is that upload's network delay, plus drift inside the window. A session with no observed offset (an upload without the header) has neither field.

## Joining the camera to `interaction.pose`

Three joins, in this order (D-PC20). Take the first that matches.

**1. Sequence (exact for `interact`).**

1. Match the camera row to its `sent_seq` row on `session_id` and `send_id`.
2. Find the pose row with the same `entity_id`, a matching kind (`interact` to `interact`, `useItem` to `use_item`), and `mercury_seq` inside `mercury_seq_first..=mercury_seq_last`.
3. Compare the range **modulo 2^28**: Mercury sequences wrap at 28 bits, so a range can cross the wrap. The rule is `seq_in_range` in `crates/client-telemetry/src/hooks/ability_trace/seq_join.rs`; do not write `first <= seq <= last` by hand.
4. Require the two times within 60 s, because sequences are reused across logins.

For `interact` the server records the sequence of the call exactly. For `useItem` the pose row's `mercury_seq` is **best effort** (the newest call within 5 s), so a sequence match on `use_item` can name the wrong call when two arrive close together. Prefer join 2 there, or check both.

**2. Server time.** The nearest pose row with the same `entity_id` and kind whose `timestamp` is within 500 ms of the camera row's `server_ts_ms`. The bound on the error is the upload's network delay (see the clock above), so a slow upload widens the real error beyond the typical case.

**3. Fallback**, for rows from before the clock fields existed (no `server_ts_ms`): the client `ts_ms` within 1 s of the pose row's time. The two clocks are unsynchronised, so use this only on old data and expect false matches.

A sketch of join 1, with the wrap rule left to the caller (ClickHouse does not have `seq_in_range`; export both sets and join in the tool, or add the modulo yourself):

```sql
SELECT c.attributes_number['pitch_offset_deg'] AS pitch_offset_deg, p.attributes_string['target_tag'] AS target_tag
FROM signoz_logs.distributed_logs_v2 AS c
INNER JOIN signoz_logs.distributed_logs_v2 AS s
  ON s.attributes_string['session_id'] = c.attributes_string['session_id']
 AND toInt64(s.attributes_number['send_id']) = toInt64(c.attributes_number['send_id'])
 AND s.attributes_string['client_target'] = 'client.camera.sent_seq'
INNER JOIN signoz_logs.distributed_logs_v2 AS p
  ON toInt64(p.attributes_number['entity_id']) = toInt64(c.attributes_number['entity_id'])
 AND p.scope_name = 'interaction.pose'
 AND toInt64(p.attributes_number['mercury_seq']) BETWEEN toInt64(s.attributes_number['mercury_seq_first']) AND toInt64(s.attributes_number['mercury_seq_last'])
 AND abs(p.timestamp - c.timestamp) < 60000000000
WHERE c.attributes_string['client_target'] = 'client.camera.at_interact'
  AND c.attributes_string['cimmeria.session_kind'] = 'player'
LIMIT 20
```

This sketch is untested and ignores the wrap; use it to see the shape. The exports that feed `lab pose-library --camera` are in [interaction-pose-views.md](../operations/signoz/interaction-pose-views.md#4-camera-rows).

**Mapping to the lab pose keys.**

| Lab key | From |
|---|---|
| `pitch_deg` | `pitch_offset_deg` |
| `zoom` | `zoom` |
| `yaw_offset_deg` | `atan2(tx - cx, tz - cz)` in degrees, minus `camera_yaw_deg` |

The third needs the camera position `(cx, cz)` and `camera_yaw_deg`, which are absent when `camera_pose_zero` is true, and the target position `(tx, tz)`, which the pose row does not log; derive it as the catalog entry says (D-PC9). Wrap the result to `[-180, 180)`.

## What the S-CAM verdict means

S-CAM asked which rotation the avatar update carries to the server: the camera's or the pawn's. The finding, [avatar-update-rotation-source.md](../reverse-engineering/findings/avatar-update-rotation-source.md), is **verdict A: the wire carries the pawn's rotation, not the camera's.** Three consequences:

- The server's `player_yaw_deg` on a pose row is the pawn's body facing. It says nothing about where the player was looking, which is why the camera is read at all.
- `wire_pitch_deg` is the pawn's pitch: 0 while the avatar walks or falls, and double the true pitch because the server unpacks pitch at the yaw scale.
- The DLL reads the pawn pose (`pawn_*`) beside the camera so the two can be compared on the same click. Had the verdict been B, the pawn would be passed as absent and no `pawn_*` key would ever be written; the code keeps that path.

## Known gaps

- Live verification of the camera read and the sequence join: PC-19.
- `useItem` sequence exactness is best effort, as above.
- No saved view exists for camera rows; the [queries](../operations/signoz/interaction-pose-views.md) are recipes.
