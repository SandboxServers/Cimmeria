# The stargate address book has three copies, and they drift differently

Read before touching `known_stargates`, the DHD dial UI, or any "grant the
player a persistent list entry" feature.

## Three copies, one per layer

| Copy | Written by | Read by |
|---|---|---|
| `sgw_player.known_stargates` (`integer[]`) | `base/world_entry/gate_travel/persist_arrival.rs` (on arrival) and `address_grant.rs` (content grant) | `world_entry_appearance/client_ready.rs`'s `PlayerInitRow` SELECT, at every world entry |
| `CellEntity::known_stargates` | `InitPlayerState` at `onClientReady`; `executor/stargate.rs`; `gm/travel.rs` (`gmDHD`, in-memory only) | `cell/gate_travel/address_book.rs::player_knows_stargate` — the dial gate |
| The client's list | `setupStargateInfo` at map load (whole list, **once**) and `updateStargateAddress` (client method 66, one entry) | the DHD dial UI |

**The client is handed the whole book exactly once per map load.** Any
mid-session grant is invisible without method 66. This is the trap: a grant
that writes the DB and the cell entity looks correct in every server-side
test and does nothing the player can see until they relog.

## Ordering rule for a mid-session grant

Write the cell entity and send method 66 **before** the DB write is
confirmed, not after. The cell's copy is what the dial gate enforces, so
gating the client's copy on a database round trip produces the worse
divergence: the server accepts a dial the client's UI never offered. A lost
DB write costs the address at next login and can warn; a lost method-66 send
is silent and makes a granted address undialable.

Precedent for a cell-side client method in this path: the dial refusal in
`address_book.rs` sends `onErrorCode` as a `CellToBaseMsg::EntityMethodCall`.
`handle_grant_cash`'s "base sends `onCashChanged` after the write" is the
opposite pattern and is right for *cash*, where the client needs the
authoritative total, not for a set membership the cell already knows.

## Wire and id facts

- `updateStargateAddress` = client method 66, `INT32 addressId` (LE) +
  `UINT8 hasAddress` + `UINT8 hidden`. Six bytes. 2009 sent `(id, 1, hidden)`
  on add and `(id, 0, 0)` on remove (`deprecated/python/cell/SGWPlayer.py:626`).
  The constant lives at `cell/client_methods/gate_travel.rs`, not in
  `mercury::method_idx` (which has 65 and not 66 — the drifted partial copy).
- **`address_origin` is a DHD glyph (1-38), not an identifier.** It repeats
  across rows. The key into `SpaceManager::stargates` is `stargate_id`.
  Harset is `stargate_id = 3`, `address_origin = 6`, world 57.
- There is **no hidden list** anywhere in Cimmeria: no column, no wire slot,
  and `mercury::world_data::map_loaded` hardcodes an empty hidden array.
  2009 accepted `known ∪ hidden`; here "known" is the whole book.

## Idempotency has to be at both ends

`known_stargates` is a bare `integer[]` with no uniqueness constraint and
`resources.stargates.stargate_id` has none either, so a double-append is
silent, permanent, and shows as a duplicated DHD row. The cell's
`contains()` early-return and the SQL set-difference append are **not** the
same lock (the cell's copy is a snapshot taken at `onClientReady`), so both
are needed. Use `persist_arrival.rs`'s statement shape rather than writing a
`CASE WHEN … = ANY(…)`; the sub-SELECT must stay inline and correlated, not a
CTE, or an EvalPlanQual recheck can double-append.

Related: [[content-chain-condition-context-gaps]],
[[chain-replay-executor-guards]], [[db-test-revert-verification]].
