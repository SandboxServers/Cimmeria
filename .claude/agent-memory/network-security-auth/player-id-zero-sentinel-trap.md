---
name: player-id-zero-sentinel-trap
description: player_id 0 is a fallback player-load sentinel that propagates into PendingClientReadyInfo — fail-closed checks keyed on it can deny capability after player loading falls back
metadata:
  type: project
---

`query_player_load_data` returns `default_player_load_data()` when the DB pool
is unavailable, a row is missing or account-mismatched, or the query errors; that
default carries `player_id: 0`
(`crates/services/src/base/world_entry/methods/player_load/meta.rs`).
`map_loaded.rs` then stores that value into `PendingClientReadyInfo.player_id`,
which `world_entry_appearance/client_ready.rs` uses as the key for every
per-player DB read it issues (abilities, archetype, bandolier, and anything
added later).

**Why:** a transient DB error during `playCharacter` player loading can turn into
`WHERE player_id = 0` for ready-time reads. For cosmetic reads
(abilities → empty hotbar) that is merely ugly. For a **server-authoritative
capability check** that fails closed on an empty list, it silently revokes
the capability until a later successful world-entry initialization replaces that
state. The original player-load query logs its error, but the abilities read
silently defaults to an empty list.

**How to apply:** when reviewing or writing any security-relevant read keyed
on `pending.player_id`:

- Do not add a fresh `SELECT ... WHERE player_id = $1` + `unwrap_or_default()`.
  The existing `abilities` query in `client_ready.rs` uses that shape; it is a
  precedent for cosmetic state only, not for authorization inputs.
- Prefer carrying the already-loaded value forward on `PendingClientReadyInfo`
  (it is populated from the same `PlayerLoadData` that fed the client's wire
  payload, so server and client agree by construction).
- If you must query, use `fetch_optional` on the whole column so
  `Err` / `Ok(None)` / `Ok(Some(empty))` stay distinguishable, log the first
  two at ERROR per `docs/architecture/negative-logging-convention.md`, and
  add `AND account_id = $2` for an account-owned player row. The existing
  player-load query already carries that guard.

Related: [[known-stargates-write-path]].
