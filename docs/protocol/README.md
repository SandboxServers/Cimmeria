# Protocol Documentation

> **Last updated**: 2026-06-20

Wire formats, Mercury messaging, and client-server protocol documentation.

## Documents

| Document | Description | Status |
|----------|-------------|--------|
| [message-catalog.md](message-catalog.md) | All 420 Event_NetOut/NetIn with handler addresses; cooked-data wire path (`versionInfoRequest`, `onVersionInfo` with `InvalidKeys`, `resourceFragment`) | HUB - initial |
| [mercury-wire-format.md](mercury-wire-format.md) | Mercury packet structure, reliability, channels, AES-256 encryption | Complete |
| [entity-property-sync.md](entity-property-sync.md) | Property flag-based synchronization protocol; delta encoding, create/update formats | Complete |
| [login-handshake.md](login-handshake.md) | Auth flow: challenge, shard key, server select, baseAppLogin binary format | Complete |
| [position-updates.md](position-updates.md) | Entity movement and volatile property updates; avatarUpdate variants, packed formats | Complete |
| [message-dispatch-table.md](message-dispatch-table.md) | Mercury message dispatch table: message id → handler mapping | Complete |
| [client-method-dispatch-table.md](client-method-dispatch-table.md) | SGWPlayer client-method dispatch table (server → client), by method index | Complete |
| [cell-method-dispatch-table.md](cell-method-dispatch-table.md) | SGWPlayer exposed CellMethod dispatch table (client → cell), by method index | Complete |
| [sgwplayer-base-method-dispatch-table.md](sgwplayer-base-method-dispatch-table.md) | SGWPlayer exposed BaseMethod dispatch table (client → base), by method index | Complete |
| [client-verified-wire-formats.md](client-verified-wire-formats.md) | Wire formats verified byte-exact against the live client | Complete |
| [world-entry-phases.md](world-entry-phases.md) | World entry phase sequence: the ordered steps from baseAppLogin to in-world | Complete |
| [auto-cycle-button.md](auto-cycle-button.md) | Auto-cycle (auto-fire) button: protocol and behavior reference | Complete |
| [item-sequence-lookup.md](item-sequence-lookup.md) | Item sequence lookup: `items_event_sets` archaeology, item-id → sequence resolution | Complete |

See also: [../architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md) — the cooked-data wire path is what powers Cimmeria's per-mission PAK overrides.

## Key References

- **BigWorld source**: `external/engines/BigWorld-Engine-2.0.1/` (if present)
- **Cimmeria Rust implementation** (active): `crates/mercury/`, `crates/services/src/auth/`, `crates/services/src/base/`, `crates/services/src/cell/`
- **Legacy C++ implementation** (historical, not extended): `deprecated/cpp/src/` (`mercury/`, `authentication/`, `baseapp/`, `cellapp/`)
- **Entity definitions**: `entities/defs/` — define the property/method contract
- **Existing docs**: `docs/connection-flow.md`, `docs/network-messages.md`

## Message Count Summary

Of the 420 cataloged messages (253 NetOut client → server, 167 NetIn server → client), roughly a quarter are implemented end-to-end on the Rust server — concentrated in the world-entry, movement, combat, inventory, and mission paths. The figures below are an approximate, last-measured snapshot rather than a live count; see [message-catalog.md](message-catalog.md) and [../gap-analysis.md](../gap-analysis.md) for per-message status.

| Direction | Count | Implemented (approx.) | Coverage (approx.) |
|-----------|-------|-----------------------|--------------------|
| NetOut (client → server) | 253 | ~65 | ~26% |
| NetIn (server → client) | 167 | ~45 | ~27% |
| **Total** | **420** | **~110** | **~26%** |
