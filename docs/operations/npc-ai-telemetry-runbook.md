# NPC AI telemetry runbook

> Type: how-to. Audience: whoever plays a session and then has to explain what the NPCs did (the owner, packet workers, reviewers).
> Updated: 2026-09-25 (NA25). Companions: [NPC AI views and dashboard export](signoz/npc-ai-views.md) (reference for every object used here), [observability ADR](../architecture/observability.md) (target catalog, field meanings, metric names), [SigNoz deployment](signoz-deployment.md), [SigNoz remote access](signoz-remote-access.md), [NPC AI telemetry plan](../analysis/npc-ai-restoration/telemetry.md) (why each event exists), [work packets](../analysis/npc-ai-restoration/work-packets.md).

You use this after a play session to answer "what did that NPC do, and why?" from SigNoz alone, with no debugger and no code reading. Every question has one saved view or one dashboard panel that answers it.

## Before you start

- You can reach the SigNoz UI (see [signoz-remote-access.md](signoz-remote-access.md)) or the SigNoz MCP from Claude Code.
- The server that ran the session is a build with NA00 (#776) and NA02 (#781) or later. On an older build most panels read "No data" and several views return nothing. Check `service.version` on any row: it is the git SHA the server was built from.
- You know roughly when the session happened. Set the time range to cover it before you open anything.

## After a play session

1. **Open the dashboard.** In SigNoz, open **Dashboards → Cimmeria — NPC AI health** and set the time range to the session.
2. **Read the detector table first.** The `Detector totals over the range, per world` panel sums every stuck, float and leash detector. After NA10-NA12 every cell should be 0. A non-zero cell tells you which view to open next (see the table below).
3. **Scan the rest of the dashboard.** Aggro by cause, transitions by reason, path status mix and idle-unticked tell you whether the session was normal. A path mix that is not almost all `ok`, or a rising `idle_unticked` line, is worth a note even if no detector fired.
4. **Check cover.** The `Cover coverage per space` panel shows one row per space. A WARN row with `reason = no_usable_cover` means that space's cover-seeking NPCs cannot take cover at all.
5. **Drill into one NPC.** Get an NPC id from a detector row or from a `.bug` bookmark (next section), then open the matching view and add `AND npc_id = N` to its search bar.
6. **Scope to the colo if the dev box also ships.** Add `AND cimmeria.deploy_env = 'colo'` to any view, or to a panel's filter, when local runs share the same SigNoz.

## Start from a `.bug` bookmark

A `.bug <note>` typed by a GM in game is the best anchor you have. It writes one `playtest.bookmark` row and one `playtest.bookmark.entity` row per entity within 60 u of you (the nearest 32, your target always included), all sharing a `bookmark_id`.

1. Open the **Playtest: bookmarks (.bug notes)** view and find your note. Copy its `bookmark_id` and timestamp.
2. In the Logs Explorer, run `scope_name = 'playtest.bookmark.entity' AND bookmark_id = <id>` (a number, no quotes). Each row names an entity near you by `entity_id`, with its AI state, nav path, threat, velocity and `y_above_ground` at that instant. Pick the NPC you were complaining about and note its `entity_id`. For an NPC that is the same number the NPC AI rows carry as `npc_id`.
3. Narrow the time range to about ±30 s around the bookmark and open **NPC AI — Timeline for one NPC** with `AND npc_id = N`. You now see that NPC's state changes, aggro, leash and path requests in order.
4. If the complaint was about how the NPC looked (running in place, facing the wrong way), open **NPC AI — What did the client see?** with `AND entity_id = N`. The avatar sample keys on `entity_id`, not `npc_id`.

Say what you were looking at in the `.bug` note ("guard by the cell door ignored me"). The note is the only record of what you saw; the telemetry records what the server did.

## Which view answers which question

Every view lives in the Logs Explorer under the category `npc-ai`. The exact filters and columns are in [signoz/npc-ai-views.md](signoz/npc-ai-views.md#saved-logs-explorer-views).

| Question | Open | What to look for |
|---|---|---|
| Which NPCs aggroed, and how? | View **NPC AI — Which NPCs aggroed, and how?**; dashboard panel *Aggro by cause* | Group by `cause` and `tag`. `proximity` is aggro on sight; `damage` means the player hit first; `content_threat` came from a content chain; `assist` means a same-faction neighbour engaged and this NPC joined (NA14) |
| Why did this guard ignore me? | View **NPC AI — Why did this guard ignore me?** plus `AND npc_id = N`; dashboard panels *Aggro-scan rejections* and *Idle NPCs the AI tick skips* | Each `candidate_rejected` row names a `reason`. **No rows at all** means the NPC was never ticked: it is counted in `npc_ai_idle_unticked` |
| What happened to this one NPC, in order? | View **NPC AI — Timeline for one NPC** plus `AND npc_id = N` | Read top to bottom: `from` → `to` with `reason`, then leash, path and movement rows |
| Who is running in place? | View **NPC AI — Who is running in place?**; counter `npc_stale_velocity_total` | `path_state = empty` is a path cleared mid-leg; `stalled` is a path the NPC is not walking |
| Who is floating or sunk into the floor? | View **NPC AI — Who is floating?**; counter `npc_ground_deviation_total` | Sort by `dy`; `dir` is `up`, `down` or `unknown` (no floor within jump height) |
| Who is stuck? | View **NPC AI — Who is stuck?**; counters `npc_stuck_total`, `npc_off_mesh_total`, `npc_idle_parked_total`, `npc_leash_loop_total` | `stuck` = chasing without closing; `npc_off_mesh` = standing off the navmesh; `npc_ai.idle_parked` = gone Idle away from spawn and never ticked again; leash `loop` = three leashes in 60 s |
| Is a player left in combat after the NPC reset? | Dashboard panel *Detector totals*, column `cleared_without_exit` | Any non-zero value is a player whose `threatened_mobs` still lists an NPC that cleared its threat |
| Does this map have usable cover? | View **NPC AI — Does this map have usable cover?**; dashboard panel *Cover coverage per space* | `nodes_on_mesh` against `nodes_in_world`; WARN `no_usable_cover` is a map where cover cannot work |
| Why no cover in this fight? | View **NPC AI — Why no cover in this fight?** plus `AND npc_id = N`; dashboard panel *no_cover reasons* | `reason` on the `no_cover` rows; `cover.selection` rows show the node that won and the top three losers |
| Why did the pathfinder fail? | Dashboard panels *Path request status mix* and *Path failures by reason*; then the Timeline view filtered to `scope_name = 'npc_ai.path'` | `no_start_poly` / `no_end_poly` usually mean the NPC or its target is off the mesh; `target_is_gm = true` is a GM standing somewhere the mesh does not cover |
| Why is this NPC holding fire, or firing with `los=blocked`? | View **NPC AI — Why is this NPC holding fire?** plus `AND npc_id = N` and a short time range | On `npc_ai.tick` rows, `los` is the navmesh verdict and `los_policy` is the rule that acted on it. `los=blocked los_policy=stationary_relaxed` is a stationary NPC firing across furniture the navmesh cuts out, which is expected (D-NA11). `stationary_other_storey` is a stationary NPC holding because the target is more than 4 u above or below it. `in_cover_slot` is a mobile NPC at its cover slot (it holds Cover Stance) firing over the cover, which is expected (NA22). `strict` with `blocked` is a mobile NPC walking toward its target. The sampled `npc_ai.los` rows give the ray endpoints and `hit_xyz` |
| What did the client see? | View **NPC AI — What did the client see?** plus `AND entity_id = N` | `npc_moved_since_last = false` beside a non-zero `vx` / `vz` is running in place as the client saw it. It is a 1-in-101 sample (`sampled_1_in`, with `suppressed` sends between samples), so absence proves nothing. Every send is in `logs/world_entry.log` as `AoI: entity position update` if you have the server's disk |

## Mining a large pull with the helper scripts

A SigNoz MCP result over about 25k tokens lands in a tool-results file instead of the conversation. Do not read that file raw. The scripts in [docs/analysis/npc-ai-restoration/evidence/signoz/](../analysis/npc-ai-restoration/evidence/signoz/) condense a raw logs pull into one line per event.

1. Save the raw logs JSON (the tool-results file, or an API export) under `signoz-raw/` next to the scripts. The analysis scripts expect these names: `ai_main.json` (NPC AI rows), `ticks.json` (`npc_ai.tick`), `npc_movement.json` (`movement.npc`) and `spawns.json` (`spawner.npc_behaviour`, used for id-to-name lookups).
2. From that directory, run `python cond2.py signoz-raw/<file>.json` to print every row as `time severity target | body | key=value ...`. It skips `playtest.bookmark.entity` rows. This works for any pull.
3. For a first look at a specific pattern, run the analysis scripts: `ticks.py` (stuck Fighting runs), `inplace.py` (running in place), `aggro2.py` (proximity or damage aggro per NPC name).

`aggro.py` and `aggro2.py` were written for the pre-NA00 session in the audit. They match the old unstructured bodies (`auto-aggro`, `preempt`). On an NA00 or later build, use the **Which NPCs aggroed** view or `npc_ai.aggro` rows grouped by `cause` instead. `cond2.py` does not depend on message text and stays current.

When you query through the MCP, always filter on `service.name = 'cimmeria-server'` and never call `signoz_get_field_keys` without a filter.

## Rows that were only in the log files

Since NA25 every row the server writes to `logs/*.log` also reaches SigNoz, so you should not need the server's disk. TRACE-level rows go to their own service: query `service.name = 'cimmeria-trace'`, never the default `cimmeria-server`. That is where to look for:

- `movement.navmesh` rows with `reason = 'advisory_off_mesh_accepted'`: where players walked off the navmesh in an advisory world (Castle, Harset). At most one row per player per 500 ms; `suppressed` counts the packets in between. This is the input for a mesh rebake.
- The pre-world-entry and dispatch drops: `Cell method before world entry -- ignored`, `Ignoring cell method until mapLoaded arrives`, `Unhandled client message`, `Unhandled Account base method`, `Bundle truncated`, `Dropping EntityMoved while witness is pre-onClientReady`.
- The wire sends and ACK traffic: the `UDP_OUT ...` rows, `AVATAR_UPDATE_EXPLICIT -> CellService`, `Queueing ACK for client reliable message`, `Piggybacking ACKs on tick_sync`.

Two per-packet rows are sampled, not copied. `DECRYPT_OK` arrives as `scope_name = 'wire.sampled.decrypt'` (1 in 53, with the hex) and `UDP_IN` as `wire.sampled.udp_in` (1 in 53, length only). Each sample carries `sampled_1_in` and `suppressed`; the true count over a window is the sum of `1 + suppressed`. The full stream is still in `logs/base.log`.

Keep the time range short on `cimmeria-trace`: it receives roughly one row per datagram.

## When a panel or view looks wrong

- **"No data" on a metric panel.** The metric has not been emitted in this time range. On 2026-09-25 only `npc_ai_decisions_total`, `npc_path_fail_total` and `npc_respawns_total` existed; the rest arrive with the first session on an NA00/NA02 build. Check `signoz_list_metrics` with `searchText = npc`.
- **`key ... not found` in the search bar.** SigNoz refuses a filter on a field it has never ingested. The field arrives with the build that emits it. Drop the clause for now.
- **A view shows rows from the wrong server.** Add `AND cimmeria.deploy_env = 'colo'` (or `'dev'`) and, to pin one build, `AND service.version = '<sha>'`.
- **The dashboard or a view is gone.** Recreate it from [signoz/npc-ai-views.md](signoz/npc-ai-views.md) and [signoz/npc-ai-health.dashboard.json](signoz/npc-ai-health.dashboard.json).

## Keeping this current

When a packet adds or renames an NPC AI target, field or metric, update the [observability ADR](../architecture/observability.md) catalog first, then the affected view or panel in SigNoz, then [signoz/npc-ai-views.md](signoz/npc-ai-views.md) and the dashboard JSON export, then the question table above.
