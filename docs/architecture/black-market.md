# Black Market / Auction House

> **Last updated**: 2026-07-25
> **Audience**: Engineers touching the auction house, escrow, or the client-side method-binding patch
> **Type**: ADR + reference
> **Owner**: Social systems
> **Status**: Implemented but **unmerged** — Phases 1–3 live on `feat/571-black-market-phase1` (PR #586, issue #571). `crates/services/src/base/black_market/` does not exist on `main`. Client-side window restoration is tracked separately by issue #587.
> **Confidence**: High for the server state machine (code + tests in-branch); Medium for the wire contract (three constants are admitted guesses pending x64dbg capture); High for the client-binding diagnosis (owner-confirmed live, 2026-06-21)

## Context

The Black Market is SGW's player-to-player auction house. The client half
is *complete* — CEGUI layouts (`UIAuctionView`, `UIAuctionTime`),
`BlackMarket.lua`, a C++ auction store with three views (SearchResults /
MyAuctions / MyBids), and Lua read bindings. The **server** half never
existed: both `deprecated/python/{base,cell}/SGWBlackMarket.py` files are
`__init__`-only stubs with every method `pass`, and no auction tables
shipped in `db/sgw/`. See
[black-market-restoration.md](../reverse-engineering/findings/black-market-restoration.md).

That left the surface in the worst possible state for a server emulator:
the client emits six well-formed cell methods that a naive dispatcher
would decode and act on, but nothing on the server enforced ownership,
funds, or authorisation. The security audit catalogued this as **CAT-I**
([CAT-I-black-market.md](../security-audit/2026-05-31-server-authority/findings/CAT-I-black-market.md))
— six findings covering the whole interface being stubbed, plus create,
bid, cancel, search, and the absent expiry sweep.

There is a second, unusual constraint. The Black Market was **shelved by
its original developers before the client-side wiring was finished**. The
six server→client methods (indices 90–95) are parsed, named, and flagged
`Exposed` in the client's entity description — byte-identical to the
working `onDialogDisplay` at index 105 — but they were never bound into
the entity's method-handler map, so every one of them lands on the
dispatcher's silent-drop path. A correct server cannot make the window
appear on a stock client. That is the subject of
[black-market-client-window-patch.md](../reverse-engineering/findings/black-market-client-window-patch.md)
and the reason this ADR has a client-side section at all.

## Decision

Implement the auction house **server-authoritatively and fully**, in the
existing cell→base split, and treat the client-side gap as a separate
binary-patch problem rather than as a reason to reshape the server.

### 1. Surface: cell methods 61–66 in, client methods 90–95 out

`SGWBlackMarketManager` is the 10th `<Implements>` interface on
`SGWPlayer`, which fixes both index ranges. The inbound half is decoded
in the cell and forwarded to the base; nothing about an auction is
decided cell-side.

| Cell method | Name | Payload | Handling |
|---|---|---|---|
| 61 | `BMSearch` | `BMSearchOptions` (11 fields, variable) | `CellToBaseMsg::BMSearch` |
| 62 | `BMCreateAuction` | `INT32 itemInstanceId, INT32 startingPrice, INT32 buyoutPrice, UINT8 auctionLength` — **13 bytes** | `CellToBaseMsg::BMCreateAuction` |
| 63 | `BMPlaceBid` | `INT32 sequenceId, INT32 bidAmount` — 8 bytes | `CellToBaseMsg::BMPlaceBid` |
| 64 | `BMCancelAuction` | `INT32 sequenceId` — 4 bytes | `CellToBaseMsg::BMCancelAuction` |
| 65 | `BMStartWatchingItem` | `INT32 itemDefId` | logged `UNIMPLEMENTED`, no state |
| 66 | `BMStopWatchingItem` | `INT32 itemDefId` | logged `UNIMPLEMENTED`, no state |

Decoders live in
`cell/cell_methods/black_market/mod.rs`;
the base-side routing arms in
`base/world_entry/cell_dispatch/black_market_dispatch.rs`.

| Client method | Name | Args | Sent by |
|---|---|---|---|
| 90 | `onBMOpen` | `INT32 entityId` (the auctioneer NPC) | content-engine `Action::OpenBlackMarket` |
| 91 | `onBMError` | `INT32 errorId` | every rejection branch of create / bid / cancel |
| 92 | `onBMAuctions` | `UINT32 count`, `count ×AuctionItem`, `INT32 view`, `INT32 total` | search |
| 93 | `onBMAuctionRemove` | `INT32 sequenceId` | cancel, expiry sweep |
| 94 | `onBMAuctionUpdate` | one `AuctionItem` | create, bid |
| 95 | `onBMWatchedItemsUpdate` | `ARRAY<INT32>` | never (watch list unimplemented) |

Serializers are in
`base/black_market/wire.rs`;
the send wrappers in
`base/black_market/send.rs`.
Indices are pinned in `crates/services/src/mercury/mod.rs` (`method_idx`)
and `crates/services/src/cell/client_methods/black_market.rs`.

**Names are narrow `STRING`** (4-byte LE length prefix + UTF-8 body), not
`WSTRING`/UTF-16 as most other SGW social systems use. This is
deliberate and load-bearing — see the open item on `sellerName` below.

### 2. `player_id` resolution fails closed

Every inbound method resolves the caller's `player_id` through
`resolve_player_id`, which returns `None` rather than defaulting to `0`.
An auction op keyed on `player_id = 0` would target a sentinel row, so
the dispatcher logs a warn and drops the action instead. This is the one
piece of authorisation the cell does; everything else is base-side.

### 3. Lifecycle: four states, one terminal transition each

`sgw_auction.status` is the whole state machine: `0 = ACTIVE`,
`1 = SOLD`, `2 = CANCELLED`, `3 = EXPIRED`. There is no intermediate
"settling" state — every transition out of `ACTIVE` happens inside one
transaction that also moves the item and the cash.

```text
                     createAuction
                          │  (escrow item out of inventory)
                          ▼
   placeBid ────────►  ACTIVE  ────────► CANCELLED   (seller reclaim:
   (refund prior,        │  │             item returned, bidder refunded)
    hold new)            │  │
                         │  └──────────► EXPIRED     (sweep, no bidder:
                         │                            item mailed back)
                         └─────────────► SOLD        (sweep, has bidder:
                                                      cash mailed to seller,
                                                      item mailed to buyer)
```

Accept/reject decisions are factored out into pure predicates in
`validate.rs`
so every rejection branch is unit-testable without a database. Bid
precedence is fixed: auction-gone → is-seller → bid-too-low →
insufficient-funds.

### 4. Escrow is a DELETE, not a flag

`createAuction` escrows by **deleting the inventory instance row** and
recording its snapshot (`item_def_id`, `stack_size`, `durability`,
`charges`) onto the auction. The `DELETE … WHERE character_id = $1 AND
item_id = $2 RETURNING …` both proves ownership and yields the snapshot
in one statement — a miss means the seller did not own that instance, and
`validate_create` fails closed on `Ok(None)`.

Returning an item (cancel, or either sweep path) **re-inserts a fresh
instance** via `return_item`: the original `item_id` was consumed by the
escrow DELETE, so the returned item gets a new id from the sequence. It
lands in container 0 at `COALESCE(MAX(slot_id), -1) + 1` — never at
`slot_id = -1`, which is the inventory swap sentinel elsewhere in the
codebase and would break the move/swap path if a row parked there.

Cash escrow is symmetric: a bid **debits the bidder immediately** and the
prior high bidder is credited back in the same transaction. The overdraw
guard is in SQL (`WHERE naquadah + $1::int >= 0`) so check and write are
atomic — two concurrent bids cannot both pass a stale balance snapshot. A
`RETURNING` miss is disambiguated from a missing player row by a
follow-up existence probe, so callers get `InsufficientFunds` vs
`NoSuchPlayer` correctly.

One subtlety worth preserving: a bidder **raising their own** high bid is
validated against the post-refund effective balance
(`balance + auction.current_bid`), because the refund happens before the
new debit. Validating on the pre-refund snapshot would wrongly reject a
legitimate self-raise.

Helpers are shared and executor-generic (`sqlx::PgExecutor`) so the same
functions compose inside a transaction or against a bare pool —
`helpers.rs`.

### 5. Row locks, not optimistic retry

`placeBid` and `cancelAuction` both `SELECT … FOR UPDATE` the auction row
before touching cash. The sweep does the same, and re-reads the locked
row rather than trusting its own pre-lock snapshot, so the sold/unsold
decision uses post-lock `current_bid` / `current_bidder`. Concurrent
settlement is harmless: the second worker's `status == ACTIVE` guard
fails and it returns `Ok(None)`.

### 6. Expiry sweep: a 30-second background task, one transaction per auction

`sweep.rs` mirrors
the outbox-drainer pattern — `tokio::spawn`, a startup pass to settle
anything already due from before the process started, then an interval
ticker at `SWEEP_INTERVAL = 30s`.

Settlement is per-auction, not per-batch, so a crash mid-sweep cannot
double-deliver: each auction commits its own item movement, mail, and
status flip together. Payout reuses the existing `sgw_gate_mail` table
(the same COD mechanism the original game used) — sold auctions mail cash
to the seller and the item to the buyer; unsold auctions mail the item
back to the seller. `settle_expired_once` carries no transport state so
the live-DB test can drive it directly; the notification fan-out
(`onBMAuctionRemove` to any online seller/buyer) is layered on top by
`run_sweep_pass`.

### 7. Boot-seed uses a reserved system seller

The house seeds three listings at boot (Pistol 55, P90 21, Health
Slappack TC1 2893) so search returns data before any player posts
anything. These are **real `sgw_auction` rows** — served by the normal
search path, expired by the normal sweep — so the seed exercises the live
system rather than a special-cased send. It is idempotent: it inserts
only when the house has zero active listings, so it never duplicates and
quietly re-seeds an emptied house.

`seller_id` carries an FK to `sgw_player`, so the seed needs a real
player row. Earlier code picked the first real player, which routed bid
cash through a live account and minted unsold items into that account's
inventory on sweep settlement. The fix is a **reserved system seller** at
`account_id = 1` / `player_id = 1`, ensured idempotently
(`INSERT … ON CONFLICT DO NOTHING`) before the listings are inserted.
Both ids sit **below their sequence start** — `accounts_account_id_seq`
starts at 2, `sgw_characters_character_id_seq` at 61 — so neither can
ever be allocated to a real account. Two `const` assertions pin that
invariant; raising `SYSTEM_SELLER_ID` into sequence range would let a
freshly-created player become the implicit system seller.

### 8. Persistence: two tables, `sequence_id` is the wire identity

[`db/sgw/BlackMarket/`](../../db/sgw/BlackMarket/) adds `sgw_auction`
(one row per listing; `sequence_id` is both the primary key and the
identity the client tracks across `onBMAuctions` / `onBMAuctionUpdate` /
`onBMAuctionRemove`) and `sgw_auction_bid` (append-only bid history for
refund/audit — the *live* current bid is denormalised onto `sgw_auction`).
`auction_length` is stored `SMALLINT` because PostgreSQL has no unsigned
one-byte integer; time columns are unix epoch seconds `INTEGER`, matching
`sgw_gate_mail.sent_time`.

### 9. Player entry is a content chain, not a hardcoded interaction

The auctioneer (`BlackMarket_Auctioneer`, spawn 238 / template 168, in
Castle_CellBlock) is reached through the ordinary content engine: chain
5030 sets the `INT_Auction` interaction bit on `player_loaded` so the
prompt survives relog, and chain 5031 fires `open_black_market` on
`interact_tag`. The action handler
(`cell/content/executor/black_market.rs`)
resolves the auctioneer entity id with the same precedence
`dialog::display` uses — chain `params["target_entity_id"]`, then the
player's `last_interaction_target` pin — and **aborts with a warn** if
neither resolves, rather than binding the window to the player's own id.

## The client-side problem

The server is correct and the window still does not open on a stock
client. Incoming entity methods are routed by
`Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`), which searches the
entity description's method-handler map keyed by
`(componentKey, methodIndex)`. **All six BM methods have array indices
but no map node** — a live log breakpoint on the silent-drop path
(`0x00c6fa8a`) recorded `idx=0x5A` (90) exactly once per auctioneer
interaction while `ContactList` (85–89) and `onDialogDisplay` (105)
dispatched normally through the same machinery.

Every alternative explanation was eliminated by byte-level comparison:
`onBMOpen`'s `MethodDescription` is identical to `onDialogDisplay`'s in
flags (`4` = Exposed), sentinel, and detail distance. There is no
per-method flag distinguishing them. A walk of the CME signal registry
(723 events) found no `Event_NetIn_onBM*` signal at all. The feature was
shelved before the incoming-event subscriber was ever wired.

The restoration is therefore a **runtime patch of the client process**,
not a server change. Two shapes are proven live:

- **Deferred wide-Lua-injection** (method 90 only): a network-thread cave
  at the drop path sets a flag; a `FEngineLoop::Tick` cave on the main
  thread consumes it and calls `BlackMarketMod.onBMOpen()` through
  `Lua_doString_wide`. Two constraints are non-negotiable — the client's
  Lua buffers are **UTF-16LE with `len` in characters**, and the
  dispatcher runs on a **network thread**, so touching the VM there
  crashes.
- **Hand-built dispatch node** (generalises to 91–95): splice a BST leaf
  for `(componentKey, methodIndex)` into the live method map, borrow an
  already-registered signal's name at `node+0x18` purely to satisfy the
  found-path's unconditional resolve, and put the real handler in the
  node's arg-handler vector, where it receives the decoded args.

Shipping this is issue **#587** — the launcher applies the patch at
client startup so it is a one-time install, not a per-session x64dbg
ritual. Addresses are build-specific to this `SGW.exe`.

## Alternatives considered

**Escrow by flagging the inventory row instead of deleting it.** Rejected:
a flag leaves the instance addressable by every other inventory path
(move, equip, split, vendor-sell), so every one of them would need a
new "is this escrowed?" check, and any path that forgot one would let a
seller sell the item twice. Deleting the row makes the item
*unreachable* by construction, and the escrow DELETE doubles as the
ownership proof. The cost is that returned items get a new `item_id`,
which is acceptable because the client tracks auctions by `sequence_id`,
not by item instance.

**Immediate buyout settlement at bid time.** Deferred, not rejected. The
original game settled a buyout instantly; the current code records a
buyout-clearing bid as a normal high bid and lets the sweep settle it at
expiry. The blocker is that settlement needs the COD/mail payout path
that the sweep owns — the right fix is to factor that into a shared
helper both call. Recorded as a `TODO` in
`bid.rs`.

**Applying the `BMSearchOptions` filters as SQL predicates now.** All 11
fields are parsed and forwarded, but only `sort_id` is used (echoed back
as `view`). Deferred until the client-side result shape is confirmed via
x64dbg — pushing guessed predicates into SQL would produce a filtered
result set we could not verify against the client's rendering, and the
filter semantics (`filter_flags` in particular) are still unknown.

**Native binding of the client methods instead of a runtime patch.** Not
available: a bare dispatch node whose `eventKey` does not resolve is a
guaranteed null-deref, because the dispatcher's found-path dereferences
the lookup result unconditionally. The signal-borrowing recipe above is
what makes native dispatch reachable at all.

**Reviving the client's own C++ auction store for methods 92–95.**
Rejected in favour of maintaining our own store and repointing the four
Lua read bindings. Two reasons: the engine's `AuctionItem` decoder
*throws* on `sellerName` (below), so we must parse the wire manually
regardless — the "free arg-decode" advantage disappears — and the store's
`AuctionItem` record is refcounted with a sub-object and string members
whose constructors are dead code.

## Known-open items

These are **not** oversights to be quietly fixed by the next reader —
each one is blocked on evidence we do not have.

### `next_min_bid` is a guess

`wire.rs:59-67`
computes the bid floor as a **5 % increment with a floor of +1**
(`current + max(current / 20, 1)`). The real `nextMinBidPrice` formula is
unknown — this is an admitted placeholder pending **x64dbg D.6**
(capture several `currentBid → nextMinBidPrice` pairs from the client).

**The bid floor is therefore not known-correct.** It is load-bearing in
two places: the `nextMinBidPrice` field the client displays
(`push_auction_item`) and `required_min_bid`, which the server enforces
in `validate_bid`. If the real formula differs, the server will reject
bids the client presents as legal, or accept bids below the client's
displayed floor. The guess is isolated to a single named function
precisely so swapping in the captured value is a one-line edit — do not
inline it at call sites.

Two neighbouring constants have the same status: the `EBlackMarketError`
ordinals in `wire::error_code` (pending **D.1**) and the
`auction_length` duration→hours table in `auction_length_seconds`
(pending **D.5**, currently 12/24/48/72/96 h).

### Auction search has no `LIMIT` — CAT-I-05, still open

`search.rs:37-45`
runs `SELECT … FROM sgw_auction WHERE status = $1 ORDER BY sequence_id`
with `fetch_all` — **every matching row**, no cap, no pagination, no
per-request bound. `handle_search` then resolves a seller name per row
and serializes the lot into a single `onBMAuctions` payload.

This is the **one CAT-I finding the implementation did not address**. It
is a self-inflicted DoS surface that grows with the size of the auction
house: any player can trigger an unbounded query, an unbounded
serialization, and an unbounded reliable send at whatever rate they can
click. The `BMSearchOptions` wire already carries the cursor fields
(`client_key`, `sequence_id`, `b_forward`) the real pagination would use,
so the fix is a bounded page plus a cursor translation — but per CAT-I-05
the server must also treat `sequenceId` as a *client-asserted cursor*,
not a trusted row id, and translate it through the caller's own most
recent result set.

### `sellerName` cannot be decoded by the client's engine — and that is probably why the feature was shelved

The `AuctionItem` FIXED_DICT's tenth field, `sellerName`, is a narrow
`StringDataType` (`0xEF8A0D00`). Its stream decoder (`0x01597FF0`)
**throws by design**, with the message *"streamToProperty(List):
StringDataType should not be used between the client and server."*

So the engine's own array→element→field decode for method 92 throws
before any handler runs. The client **cannot** decode the auction array
on the wire through its normal path — not because of a bug in our
serializer, but because the shipped type definition uses a type the
engine explicitly forbids on the network. This is almost certainly the
original reason the data side of the Black Market was abandoned, and it
is what forces the client-side workaround: methods 92 and 94 must parse
the raw wire **manually** (count + 7×INT32 + UINT8 + INT32 +
length-prefixed narrow string) and must not route through the engine
arg-decode.

**Do not "fix" `wire.rs` to emit WSTRING.** The narrow encoding is
wire-correct for a manual parser and matches the shipped field
descriptor; widening it would break the manual parser without making the
engine decoder work.

### Smaller gaps

- **Watch list (65/66, and method 95) is unimplemented.** Both cell
  methods log `UNIMPLEMENTED` and hold no state, so
  `onBMWatchedItemsUpdate` is never sent. The `SGWBlackMarket` entity's
  one property (`watchedItems: PYTHON`, an itemDefId → subscriber
  registry) has no server-side equivalent yet.
- **Bid fan-out is requester-only.** `onBMAuctionUpdate` after a bid goes
  to the bidder; the seller learns about it from the sweep or their next
  search. Per-witness fan-out was out of scope for Phase 2.
- **Offline seller names render empty.** `player_name_for_player_id`
  scans the connected-session map, so an offline seller's name falls back
  to `""`. Cosmetic — no behaviour gates on it — but it means the search
  result for a mostly-offline population shows mostly blank sellers. A DB
  lookup is the obvious fix.
- **No listing fee and no per-player listing cap** (the remaining half of
  CAT-I-02).
- **A prior bidder whose account is gone cannot be refunded.** Both
  `bid.rs` and `cancel.rs` log a loud `warn` and proceed rather than
  blocking the new bid; the held cash is unrecoverable.

## Consequences

- **New schema**: `sgw_auction` + `sgw_auction_bid` and their sequences,
  under `db/sgw/BlackMarket/`. No migration script — per repo convention
  the seed in `db/` is edited directly.
- **New background task**: the expiry sweep is spawned at base startup
  alongside the boot seed. Both are fire-and-forget `tokio::spawn`
  spawners so the caller need not be async, and they are benign if they
  race — seeded listings carry a future `expires_at`, so the sweep's
  first pass ignores them.
- **Reserved ids 1/1** in `account` / `sgw_player` are now permanently
  spoken for. Anything that enumerates players (rosters, leaderboards,
  GM listings) will see a "Black Market" player with no inventory, no
  missions, and no contact list.
- **`sgw_gate_mail` is now written by a system path.** Auction mail has
  `sender_id = NULL` and `sender_name = "Black Market"`; any mail code
  that assumes a non-null sender must tolerate it.
- **Reusable helpers landed**: `send_mail_to_player`,
  `adjust_player_cash`, `escrow_item`, `return_item` are deliberately
  generic over the executor and are the right building blocks for other
  systems that move items and cash atomically (trade, guild bank).
- **Test coverage** is in
  `base/black_market/tests/`
  (live-DB: create/bid/cancel, search, sweep) plus in-module unit tests
  for the pure validators, the wire serializers (byte-exact layout
  guards), the `BMSearchOptions` deserializer (11-field round-trip and a
  UTF-8-not-WSTRING pin), and the seed's system-seller invariants.
  `fetch_active_auctions` is exposed as a test seam so the search tests
  assert the real `WHERE status = ACTIVE` result set instead of decoding
  an encrypted Mercury packet.
- **The feature is not player-visible on merge.** Server-side correctness
  buys nothing until the client patch of issue #587 ships; today only the
  window chrome opens (method 90, by hand-applied patch) and its tabs
  render empty.

## Confidence

| Area | Confidence | Basis |
|---|---|---|
| Cell/base split, state machine, escrow semantics | **High** | Code + unit and live-DB tests in-branch |
| Inbound wire layouts (61–64) | **High** | Ghidra emitter decompiles; the 13-byte `BMCreateAuction` and 11-field `BMSearchOptions` are corrections to earlier docs |
| Outbound `AuctionItem` field order | **High** | Matches the client's 10 field descriptors at `0xEF770400` exactly |
| `next_min_bid`, `EBlackMarketError` ordinals, duration table | **Low** | Admitted guesses, pending x64dbg D.6 / D.1 / D.5 |
| COD/payout shape | **Medium** | Architecture-inferred from `sgw_gate_mail` + `payCODForMailMessage`; no Python reference implementation exists |
| Client-side binding diagnosis and patch | **High** | Owner-confirmed working in-world, 2026-06-21; byte-level descriptor comparison plus a live registry walk |

## See also

- [gameplay/black-market.md](../gameplay/black-market.md) — the system reference: what the auction house *does*, entity definitions, per-message wire tables, and current implementation status. This ADR is the complement — *why* the server is shaped the way it is.
- [black-market-restoration.md](../reverse-engineering/findings/black-market-restoration.md) — server-side RE, entity model, completeness assessment
- [black-market-wire-formats.md](../reverse-engineering/findings/black-market-wire-formats.md) — per-message wire tables
- [black-market-client-window-patch.md](../reverse-engineering/findings/black-market-client-window-patch.md) — the client binding gap, both patch recipes, and the fork-B build spec for methods 91–95
- [CAT-I-black-market.md](../security-audit/2026-05-31-server-authority/findings/CAT-I-black-market.md) — the six audit findings and their current status
