# Protocol Documentation

> **Last updated**: 2026-03-01

Wire formats, Mercury messaging, and client-server protocol documentation.

## Documents

| Document | Description | Status |
|----------|-------------|--------|
| [message-catalog.md](message-catalog.md) | All 420 Event_NetOut/NetIn with handler addresses | HUB - initial |
| [mercury-wire-format.md](mercury-wire-format.md) | Mercury packet structure, reliability, channels | Stub |
| [entity-property-sync.md](entity-property-sync.md) | Property flag-based synchronization protocol | Stub |
| [login-handshake.md](login-handshake.md) | Auth flow: challenge, shard key, server select | Stub |
| [position-updates.md](position-updates.md) | Entity movement and volatile property updates | Stub |

## Key References

- **BigWorld source**: `external/engines/BigWorld-Engine-2.0.1/src/lib/network/` and `src/lib/connection/`
- **Cimmeria implementation**: `src/lib/` (UnifiedKernel), `src/server/*/`
- **Entity definitions**: `entities/defs/` — define the property/method contract
- **Existing docs**: `docs/connection-flow.md`, `docs/network-messages.md`

## Message Count Summary

| Direction | Count | Implemented | Coverage |
|-----------|-------|-------------|----------|
| NetOut (client → server) | 253 | ~65 | ~26% |
| NetIn (server → client) | 167 | ~45 | ~27% |
| **Total** | **420** | **~110** | **~26%** |
