---
name: reference-auth-handshake-layers
description: Where Cimmeria's auth/session/character-lifecycle handlers live and what each layer is authoritative about
metadata:
  type: reference
---

# SGW auth handshake — layer map

The handshake has three layers; each owns a distinct server-authority property.

## Phase 1 + 2 — SOAP/HTTP (TCP)
- `crates/services/src/auth/handlers.rs`
  - `handle_user_auth` (POST `/SGWLogin/UserAuth`) — credential check, SID issue.
  - `handle_server_selection` (POST `/SGWLogin/ServerSelection`) — shard pick,
    ticket+session_key issue.
- `crates/services/src/auth/mod.rs` — `PendingLogin`, TTL constants, ShardInfo.
- `crates/services/src/auth/service.rs` — listener bind, reaper loop.
- Authoritative for: account_id, access_level (from `account.accesslevel`),
  AES session key (random 32B), ticket (random 10B → 20 hex chars).

## Phase 3 — UDP baseAppLogin (msg 0x00, flags 0x41)
- `crates/services/src/base/login/mod.rs`
  - `parse_baseapp_login` — wire decode; `_account_id` from body[9..13] is
    DISCARDED — trusted account_id comes from the ticket map, not the wire.
  - `handle_login` — consume ticket, evict duplicate-account session, register
    encrypted channel + spawn tick loop.
- Encrypted channel registration creates `ConnectedClientState` with the
  account_id + access_level carried from the ticket.

## Phase 4 — encrypted game packets
- `crates/services/src/base/connect_loop/encrypted/mod.rs::handle_encrypted_datagram`
  - decrypt → parse → dispatch bundle.
- `crates/services/src/base/connect_loop/account_arms.rs::dispatch_base_method`
  - 0xC2 logOff, 0xC3 createCharacter, 0xC4 playCharacter, 0xC5 deleteCharacter,
    0xC6 requestCharacterVisuals, 0xC7 onClientVersion.
- Once `player_entity_id` is Some on ConnectedClientState, the 0xC2..0xC7 range
  flips to SGWPlayer base-method dispatch instead (handled by
  `dispatch_sgw_player_base_method`).

## Where authorization-affecting state actually lives
- `account_id`, `access_level` — `ConnectedClientState` (sourced from the
  Phase 1/2 SOAP credential check + ticket). This is the source of truth for
  any "is this a GM?" decision in dispatch.
- `player_entity_id` — set by `play_character.rs:131-133` after a successful
  `query_world_entry`. Handlers gate on `Some(_)` to detect "in-world".
- DB row `sgw_player.access_level` — only read by `query_player_load_data` and
  fed back to the client as a property; NOT used for authorization. Two sources
  of truth (account vs sgw_player), authorization uses account.

## Static / server-only derivations from CharDefId on createCharacter
- `crates/services/src/base/chardef.rs::chardef_lookup` — (alignment, archetype,
  gender, bodyset, starting_world, pos_x/y/z). Client cannot spoof these.
- `resources.char_creation_visgroups` + `_choices` — visual options, validated
  per group (VIS_Optional vs VIS_Forced).
- `resources.char_creation_abilities` — starting ability list per CharDefId.
- `resources.items.container_sets` + `BAG_FILL_ORDER` — starting inventory bag
  placement.

Related: [[reference-cell-method-entity-id-authority]] for the analogous
"client says X, server overwrites with session truth" pattern at cell layer.
