# Verifying server clauses on a local lab drive

Verified on 2026-11-03 against the SP-DCB1-p1 drive (local lab server, lab run `dr-lab`).
Read-only: none of these steps touches the client or the lease.

## Why `server_log_tail` came back empty

`server_log_tail` reads an in-memory ring of only 500 entries (`BUFFER_CAPACITY` in
`crates/admin-api/src/ws/broadcast_layer/mod.rs`). The ring takes DEBUG events, and every
lab-server MCP call writes dozens of `h2::codec` and `rmcp::service` DEBUG lines into it. On
the local server, 500 entries covered about 4 seconds. A bookmark from a minute earlier has
already been pushed out by the time the verifier asks. Filters (`contains`, `since_ms`) run
after the ring has dropped those rows, so no filter gets them back. Use `server_log_tail` only
to wait for a line that is about to happen. To grade anything in the past, use one of the two
sources below.

## Source 1: SigNoz (preferred, one call per question)

The local server exports to SigNoz as `service.name = cimmeria-server` with the resource
attribute `cimmeria.lab_run` set to the name of the worktree that runs the server (it was
`dr-lab` here). Find it in the `resources_string` of any row from
`signoz_search_logs {service: "cimmeria-server", timeRange: "1h", limit: 1}`.

1. Bookmarks. The note text is in the `note` attribute, not in the body, so a `searchText`
   search finds nothing. Use a filter:

   ```
   signoz_search_logs {
     filter: "resource.cimmeria.lab_run = 'dr-lab' AND attribute.note CONTAINS 'SPU SP-DCB1-p1'",
     timeRange: "6h", limit: 20 }
   ```

   Each row has `attributes_number.bookmark_id`. It is the bookmark's time in Unix ms, so it
   gives `start`/`end` for the next query. The row also carries the player snapshot:
   position, missions with step ids, regions and `recent_events`.

2. Server events between two bookmarks. Pass the two `bookmark_id` values as `start`/`end`:

   ```
   signoz_search_logs {
     filter: "resource.cimmeria.lab_run = 'dr-lab' AND attribute.event = 'movement.hold_snap'",
     start: 1793685462448, end: 1793685533178, limit: 50 }
   ```

   Other useful filters: `scope_name = 'playtest.bookmark'`, `attribute.player_name = '<char>'`,
   `body CONTAINS '<text>'`, `severity_text = 'WARN'`. To count instead of list, use
   `signoz_aggregate_logs` with the same filter. Convert ms to clock time with a date
   function, never in your head.

## Source 2: the server's own log file (fallback when SigNoz is down)

The server writes JSON lines, timestamped in UTC, to `<server worktree>\logs\server.log`.
Here that was `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log`;
`logs\` in the main checkout holds older runs. Explorer's LastWriteTime lags because the
server keeps the file open, so do not go by it. The file is current.

```powershell
$f = 'C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log'
Select-String -Path $f -Pattern 'SPU SP-DCB1-p1' | % Line
Select-String -Path $f -Pattern 'movement.hold_snap' |
  ? { $_.Line -match '"timestamp":"2026-11-03T05:5[78]' } | % Line
```

Topic logs (`aoi.log`, `combat.log`, `world_entry.log`, ...) live next to it. `protocol.log`
and `world_entry.log` are over 1 GB, so always pass `-Pattern` and never `Get-Content` them.

## SP-DCB1-p1 result from this check

Bookmarks: `CB-U01a start` at 05:57:42Z (entity 36) and `CB-U01a end` at 05:58:53Z
(entity 37, after the relog). `movement.hold_snap` (reason `content_hold`) fired from
05:57:46Z, with rows still arriving at 05:58:50Z, after the relog. The server did hold and snap
the player back on both sides of the relog. The driver saw no snap-back on screen, and that is
a client-display question, not proof that no hold applied.
