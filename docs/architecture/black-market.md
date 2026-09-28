# Black Market / Auction House

> **Last updated**: 2026-09-27
> **Audience**: Engineers touching the auction house, escrow, or the client-side method-binding patch
> **Type**: ADR + reference
> **Owner**: Social systems
> **Status**: Implemented on `main`, not player-visible. Phases 1–3 were written on `feat/571-black-market-phase1` (PR #586, issue #571) and ported onto the split crates by packet BM-01 of the [restoration plan](../analysis/black-market/README.md), with no behaviour change. The base side is `crates/base-methods/src/base/world_entry/methods/black_market/`. Packet BM-02 fixed the contract mismatches the plan's client-IO pass found (S1–S8: `onBMAuctions` argument order and `clientKey`, `BMCreateAuction` field order, duration and time-left enums, search paging, caller-scoped views, seller names), so the tables below now describe what the client expects. Packet BM-02b moved every settlement onto the mail module's system-mail writer: items and cash leave an auction only as mail from "Black Market" (§5, §8). The client still drops methods 90–95 until the client patch ships (issue #587; packets BM-03 to BM-06). The server and the patch DLL share one codec crate, `cimmeria-patch-wire` (`crates/patch-wire`), which the server re-exports as `cimmeria_wire::black_market`.
> **Confidence**: High for the server state machine (code + tests); High for the wire contract (client IO decompiles, one shared codec); Low only for the duration table, which is design; High for the client-binding diagnosis (owner-confirmed live, 2026-06-21)

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
in the cell and forwarded to the base; the cell decides only whether the
caller may reach the auction house at all (§3), and nothing about an
auction itself.

| Cell method | Name | Payload | Handling |
|---|---|---|---|
| 61 | `BMSearch` | `BMSearchOptions` (11 fields, variable) | `BlackMarketCellToBase::Search`; ungated |
| 62 | `BMCreateAuction` | `INT32 itemInstanceId, INT32 buyoutPrice, UINT8 auctionLength, INT32 startingPrice` — **13 bytes**, `.def` order (S4). A payload with trailing bytes is refused | `BlackMarketCellToBase::CreateAuction`, auctioneer-gated |
| 63 | `BMPlaceBid` | `INT32 sequenceId, INT32 bidAmount` — 8 bytes | `BlackMarketCellToBase::PlaceBid`, auctioneer-gated |
| 64 | `BMCancelAuction` | `INT32 sequenceId` — 4 bytes | `BlackMarketCellToBase::CancelAuction`, auctioneer-gated |
| 65 | `BMStartWatchingItem` | `INT32 itemDefId` | answered `onBMError(WatchUnavailable)` (D4), no state |
| 66 | `BMStopWatchingItem` | `INT32 itemDefId` | answered `onBMError(WatchUnavailable)` (D4), no state |

Every argument layout, inbound and outbound, is defined once in
`cimmeria-patch-wire` (`crates/patch-wire/src/black_market/`), the
std-only codec crate the injected client-patch DLL links too, so the
server and the patch cannot drift apart. `crates/wire/src/black_market.rs`
re-exports it for the cell and the base. The cell-side decoders and
routing live in
`crates/cell-methods/src/cell/cell_methods/black_market/mod.rs`;
the base-side routing arms in
`crates/base-world-entry/src/base/world_entry/cell_dispatch/black_market_dispatch.rs`.
Every base-side file named below without a path is in
`crates/base-methods/src/base/world_entry/methods/black_market/`.

| Client method | Name | Args | Sent by |
|---|---|---|---|
| 90 | `onBMOpen` | `INT32 entityId` (the auctioneer NPC) | content-engine `Action::OpenBlackMarket` |
| 91 | `onBMError` | `INT32 errorId` — a `BMError` id (D3, table below) | every refusal branch of search / create / bid / cancel / watch, and the cell's auctioneer gate |
| 92 | `onBMAuctions` | `ARRAY<AuctionItem>`, `INT32 totalResults`, `INT32 clientKey` (`.def` order, S1). `clientKey` is the `UIAuctionView` the client asked for, echoed back (S2); `totalResults` is the full match count, not the page size | search |
| 93 | `onBMAuctionRemove` | `INT32 sequenceId` | cancel, buyout, expiry sweep |
| 94 | `onBMAuctionUpdate` | one `AuctionItem` | create, bid (to the bidder and any online outbid player) |
| 95 | `onBMWatchedItemsUpdate` | `ARRAY<INT32>` | never (watch list deferred, D4) |

The base-side serializers are in `wire/`, except `onBMOpen`'s and
`onBMError`'s, which both halves of the split send and so live in
`crates/wire/src/black_market.rs`; the send wrappers are in `send.rs`.
Indices are pinned in
`crates/wire/src/cell/client_methods/black_market.rs` (90–95) and
`crates/wire/src/cell/cell_methods/black_market.rs` (61–66).

**Names are narrow `STRING`** (4-byte LE length prefix + UTF-8 body), not
`WSTRING`/UTF-16 as most other SGW social systems use. This is
deliberate and load-bearing — see the open item on `sellerName` below.

**Error ids are a decision, not a recovery (D3).** The shipped
`EBlackMarketError` has two values, `0 InvalidSortType` and
`1 BMUnavailable`; the server keeps both and numbers its own refusals
after them, `2 NotEnoughFunds` through `14 WatchUnavailable`. The enum is
`BMError` in `cimmeria-patch-wire`, and the client patch's UI overlay maps
each id to text. The full table, with the `reason` label each refusal
logs, is in [gameplay/black-market.md § Error ids](../gameplay/black-market.md#error-ids).

### 2. `player_id` resolution fails closed

Every inbound method resolves the caller's `player_id` through
`resolve_player_id`, which returns `None` rather than defaulting to `0`.
An auction op keyed on `player_id = 0` would target a sentinel row, so
the dispatcher logs a warn and drops the action instead.

### 3. Create, bid and cancel need an open auctioneer

Without a gate, any client could call 62–64 from anywhere in the world:
the audit's missing-authorisation shape (CWE-862). The cell forwards
those three methods only while `black_market_access`
(`crates/cell-world/src/cell/black_market.rs`) passes, and it checks three
things on every call:

- **A recorded session.** The `open_black_market` action records a
  `BlackMarketSessions` entry, keyed by `player_id`, after it sends
  `onBMOpen`. A player the server never sent to an auctioneer has none.
- **The same auctioneer.** The player's `last_interaction_target` is
  still that auctioneer; interacting with anything else ends the right to
  trade.
- **In range.** `interact_range`, the same rule the `interact` handler
  uses: the NPC exists, is in the player's space, and is within
  `MAX_INTERACT_DISTANCE` (5 units).
- **An auctioneer.** The NPC is `NpcInteractionType::Auctioneer` (BM-07).

A refusal is `onBMError(NotAtAuctioneer)`, logged with `access=<label>`
naming which check failed. `BMSearch` is ungated on purpose: it is
read-only and every view is scoped server-side to the caller (§7), so it
exposes nothing a gate would protect. The server-authority-enforcer
reviewed the gate.

**Who is an auctioneer (BM-07).** `NpcInteractionType::Auctioneer` is
derived at spawn, and only there, from `INT_Auction` on the NPC's
**template** (`static_interaction_for_flags`). A chain's
`set_interaction_type` changes `interaction_type_flags`, the client's
cursor, never the derived type, so no chain can promote an NPC. The open
runs the same `auctioneer_check` (exists, same space, in range, an
auctioneer) before it sends `onBMOpen`; a refusal sends nothing but a chat
line and logs `bm.open_refused reason=not_at_auctioneer` with the `access`
label. So a chain bound to the wrong NPC cannot make it a Black Market
terminal. The server-authority-enforcer reviewed the check.

### 4. Lifecycle: four states, one terminal transition each

`sgw_auction.status` is the whole state machine: `0 = ACTIVE`,
`1 = SOLD`, `2 = CANCELLED`, `3 = EXPIRED`, `4 = QUARANTINED`. There is
no intermediate "settling" state — every transition out of `ACTIVE`
happens inside one transaction that also writes the mails that move the
item and the cash. `QUARANTINED` is the one exception: the sweep sets it,
in a transaction of its own, on an auction whose settlement failed for
good (§8).

```text
                     createAuction
                          │  (item row moved into container 18)
                          ▼
   placeBid ────────►  ACTIVE  ────────► CANCELLED   (seller reclaim:
   (prior bid mailed     │  │  │ │        row mailed to seller,
    back, hold new)      │  │  │ │        bid mailed to bidder)
                         │  │  │ │
                         │  │  │ └─────► SOLD        (buyout bid, D8:
                         │  │  │                      settled at once)
                         │  │  └───────► EXPIRED     (sweep, no bidder:
                         │  │                         row mailed to seller)
                         │  └──────────► SOLD        (sweep, has bidder:
                         │                            cash mailed to seller,
                         │                            row mailed to buyer)
                         └─────────────► QUARANTINED (sweep could not
                                                      settle; operator)
```

Accept/reject decisions are factored out into pure predicates in
`validate.rs`
so every rejection branch is unit-testable without a database. Bid
precedence is fixed: auction-gone → is-seller → bid-too-low →
insufficient-funds.

**The window closes at `expires_at`, not at the sweep.** Between
`expires_at` and the next sweep pass the row is still `ACTIVE`, but
`is_open` treats it as closed: a bid or cancel in that gap is refused
with `AuctionGone`, and search stops offering the row. Otherwise a late
bid could land on an auction the sweep is about to settle for the
previous high bidder.

**Rules the validators enforce.** The starting price is at least 1; a
buyout, if set, is at least the starting price (`InvalidPrice`). Only
unbound rows in the main bag (1) or the crafting bag (15) are listable
(`InvalidItem`, `ItemBound`). A seller holds at most 20 active listings,
and listing is free (D5, `TooManyListings`). The next minimum bid is 5 %
over the standing bid, and at least +1 (D6).

**A buyout settles at once (D8).** A bid at or over a non-zero buyout
price is charged the buyout price, not the bid, refunds the prior
bidder, and settles in the same transaction through `settle.rs`, the
same settlement the sweep uses. The item goes by mail, so the buyer needs
no free bag slot (D-BM10).

### 5. Escrow is a container move, not a DELETE

`createAuction` escrows by **moving the `sgw_inventory` row into the
seller's container 18** (`INV_AUCTION`). The row keeps its instance id,
its owner and every column — durability, charges, ammo — and the auction
records the listed row's `item_id` plus a snapshot (`item_def_id`,
`stack_size`, `durability`, `charges`) for the wire. The move is
`list_into_escrow` in `escrow.rs`; it proves ownership and the listable
bag under the seller's inventory locks, and fails closed on a miss.

Why a move: every settlement mails the row through the social-systems
system-mail writer (BM-02b), and `send_system_mail_tx` with
`SystemItem::ExistingInstance` accepts only a container-18 row owned by
`owner_player_id`. The DELETE-and-snapshot design this replaces could
never feed that writer, and it lost data on the way back: a re-inserted
row had a new instance id and none of the columns the snapshot did not
carry.

Container 18 is server-held, so an escrowed row is still unreachable to
the player — but that property now comes from two places rather than from
the row being gone:

- **Read filters.** The login inventory load, every resync, the
  one-item snap-back read after a refused move, and the crafting reads
  exclude container 18.
- **Container allowlists.** The move, use, trade, mail and vendor paths
  accept only the containers they name, and none names 18.

A new inventory path that reads or writes by `item_id` without a
container allowlist would reopen the hole; see Consequences.

Returning or delivering an item is a system mail (`payout_mail.rs`,
decision D-BM10). `escrowed_item` (`escrow.rs`) names what the mail
carries, under the seller's escrow lock:

- **A player's listing** mails the container-18 row itself
  (`SystemItem::ExistingInstance { item_id, owner_player_id: seller }`):
  to the buyer on a sale, back to the seller on cancel or expiry. The
  writer moves the whole row into `sgw_gate_mail_item`, and the recipient
  takes it from the mailbox, so a full bag never blocks a settlement.
- **A missing row is never minted.** A player's listing whose
  container-18 row is gone has nothing to mail: it is logged
  `bm.escrow_missing`, a cancel is refused (`Internal`) and the sweep
  quarantines the auction (§8).
- **A boot-seed listing** (`item_id = 0` alone; since BM-07 the seller is
  not a test, so a real player 1's listing settles through escrow) never had
  an instance, so a sale mails the buyer a new one of the listed type
  (`SystemItem::Minted`) and pays nobody; an unsold one moves nothing.

Cash escrow is symmetric: a bid **debits the bidder immediately** and the
prior high bidder is mailed their held bid in the same transaction (the
seller's payout and a cancelled auction's refund are mail too).
`adjust_player_cash` (`helpers.rs`) does the arithmetic in `bigint`
inside the `UPDATE`, guarded by `naquadah + delta BETWEEN 0 AND
2147483647`, so check and write are atomic — two concurrent bids cannot
both pass a stale balance snapshot, and a credit past the column's
maximum is a named `BalanceOverflow` rather than a Postgres
integer-overflow error. A `RETURNING` miss is disambiguated by a
follow-up existence probe, so callers get `InsufficientFunds`,
`BalanceOverflow` or `NoSuchPlayer` correctly.

One subtlety worth preserving: a bidder **raising their own** high bid is
validated against the effective balance (`balance + auction.current_bid`),
because the held bid counts toward the new one. Nothing is mailed for a
self-raise: only the difference is charged. Validating on the balance
alone would wrongly reject a legitimate self-raise.

### 6. Row locks in one order, not optimistic retry

Every writer takes its locks in a fixed order, so no two of them can
deadlock:

- **Create:** the seller's inventory advisory locks
  (`take_inventory_locks`), then the item row `FOR UPDATE`, then the
  seller's `sgw_player` row. Locking the seller's player row also
  serializes the listing-cap count, so two concurrent creates cannot both
  see 19 listings.
- **Bid, cancel and settlement:** the `sgw_auction` row `FOR UPDATE`,
  then the seller's escrow advisory locks (`lock_escrow`), then the
  escrowed item row, then every `sgw_player` row the transaction touches
  in ascending `player_id` (each mail recipient included,
  `lock_players`), and last the mail writer, whose own locks are then
  re-locks of rows already held (caller rule 1 of the SS-U1 API).

Every writer of a container-18 row holds the seller's escrow advisory
lock before touching it, and no writer takes an advisory lock and then an
`sgw_auction` row lock. Keep both true when you add a path.

The sweep re-reads the locked row rather than trusting its own pre-lock
snapshot, so the sold/unsold decision uses post-lock `current_bid` /
`current_bidder`. The settlement status write comes first and is
conditional (`… AND status = 0 RETURNING`), so a second worker — the
sweep racing a buyout, or two sweep passes, even one holding a stale
`ACTIVE` read — finds nothing to update (`SettleError::Gone`) and writes
no mail. That gate is what makes settlement exactly-once: the mail writer
mints cash on every call.

### 7. Search: caller-scoped views, one bounded page

`search.rs` answers `BMSearch` with one `onBMAuctions` per request, and
`clientKey` picks the view (S2): `0` Search, `1` My Auctions, `2` My
Bids. An unknown key is refused with `onBMError(InvalidSortType)`.

- **My views are the caller's own (S3).** My Auctions is the caller's
  listings and My Bids is every open auction with an `sgw_auction_bid`
  row by the caller, so an outbid player still sees it. Both are scoped
  to the caller's `player_id` server-side; the client's `sellerName` and
  `bidderName` are ignored, so no one can read another player's views.
- **One page, one message (S7).** The query reads at most
  `SEARCH_PAGE_ROWS` (50) rows from the `sequenceId` / `bForward` cursor,
  and the reply is cut to `AUCTIONS_ARG_BUDGET` (1,200 bytes of
  arguments) so it fits one unfragmented Mercury message: `MAX_BODY`
  1,348, less the 8-byte header and room for acks. That is about 23
  typical rows, more than two of the UI's 8-row pages. `totalResults`
  is the full match count.
- **Filters.** `itemName` is a case-insensitive substring (`ILIKE`);
  `minTC` / `maxTC` bound tech competency, with 0 meaning no bound.
  `sortId`, `quality` and `filterFlags` are logged and not applied.
  Rows past `expires_at` are not offered.
- **Seller names come from `sgw_player` (S8)**, so offline sellers show
  too.
- **`endTimeValue` is a time-left bucket (S6):** the smallest
  `UIAuctionTime` tier whose duration covers the time left, which is
  what the client's timer icon expects. `auctionLength` on create is the
  1-based `UIAuctionTime` value 1–5 (S5); an out-of-range byte is clamped
  to the nearest tier and the clamped tier is stored, so storage and
  duration agree.

### 8. Expiry sweep: a 30-second background task, one transaction per auction

`sweep.rs` mirrors
the outbox-drainer pattern — `tokio::spawn`, a startup pass to settle
anything already due from before the process started, then an interval
ticker at `SWEEP_INTERVAL = 30s`.

Settlement is per-auction, not per-batch, so a crash mid-sweep cannot
double-deliver: each auction commits its status flip and its mails
together, through `settle_locked` in `settle.rs` — the function a buyout
calls too. Sold auctions mail the row to the buyer and the winning bid to
the seller; unsold auctions mail the row back to the seller. Every mail
goes through `payout_mail.rs`, which builds the `SystemMail` (sender
"Black Market", the subject and body per outcome) for the mail module's
`send_system_mail_tx`, the one writer of server-originated mail.

**One bad auction cannot stop the pass (BM-02b).** The due set is read in
`expires_at, sequence_id` order and each auction settles in its own
transaction. A failure that will recur (`SettleError::is_permanent`: the
escrowed row is missing, or the writer refuses a mail, such as a bound
row won by someone else) rolls back, and a second transaction sets the
auction `QUARANTINED` (status 4) with ERROR `bm.quarantined` and its
`reason`. The item stays in container 18 and a standing bid stays held
for an operator. A database error leaves the auction `ACTIVE` for the
next pass (WARN `bm.settle_retry`). The pass then goes on, and
`settle_expired_once` returns a `SweepReport` (settled, quarantined,
retried).

`settle_expired_once` carries no transport state so the live-DB test can
drive it directly; the notification fan-out (`onBMAuctionRemove` to any
online seller/buyer, and the mail module's new-mail notice to each online
recipient) is layered on top by `run_sweep_pass`.

### 9. Boot-seed uses a reserved system seller

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

The sequences keep ids 1 free, but an import or an operator can still put
a real account or character there, and `ON CONFLICT DO NOTHING` would
hide it. So `ensure_system_seller` reads the rows back (BM-07): account 1
must be the `Black Market` account, checked before player 1 is inserted so
a squatter's account never gains the character, and player 1 must be its
`Black Market` character. Otherwise the seed lists nothing and logs
`bm.seed_refused` at ERROR with a `reason` (`account_missing`,
`account_taken`, `player_missing`, `player_taken`) and what the ids hold.
The account is created disabled, and an older boot's enabled one is
switched off. The GM `.bm_seed` runs the same check.

Seed listings have `item_id = 0`: they never had an inventory row, so
they are the one case where settlement mints an item, a new instance of
the listed type mailed to the buyer, and the one case with no seller to
pay (§5).

### 10. Persistence: two tables, `sequence_id` is the wire identity

[`db/sgw/BlackMarket/`](../../db/sgw/BlackMarket/) adds `sgw_auction`
(one row per listing; `sequence_id` is both the primary key and the
identity the client tracks across `onBMAuctions` / `onBMAuctionUpdate` /
`onBMAuctionRemove`) and `sgw_auction_bid` (append-only bid history for
refund/audit and the My Bids view — the *live* current bid is
denormalised onto `sgw_auction`). `auction_length` is stored `SMALLINT`
(the 1-based tier) because PostgreSQL has no unsigned one-byte integer;
time columns are unix epoch seconds `INTEGER`, matching
`sgw_gate_mail.sent_time`.

**Character deletion (D-BM09).** `sgw_auction.seller_id` is
`ON DELETE CASCADE` and `current_bidder` is `ON DELETE SET NULL`. Before
either fires, the `BEFORE DELETE` trigger
`sgw_player_before_delete_auctions` runs `bm_player_before_delete()`
(`db/sgw/_functions.sql`, `db/sgw/_triggers.sql`): it locks the affected
open auctions, clears the deleted character's standing bids so the
auction reopens with no phantom bid, and refunds the standing bidders of
the deleted seller's open auctions (capped at the column's maximum). The
earlier `RESTRICT` blocked the deletion of every character that had ever
listed or won anything. A deleted seller's listings and escrowed rows go
with the character, like the rest of its inventory.

### 11. Player entry is a content chain, not a hardcoded interaction

The auctioneer is reached through the ordinary content engine: chain
5030 fires `open_black_market` on `interact_tag` for
`BlackMarket_Auctioneer`, Machra (template 305, spawn 405) in the
Castle_CellBlock stasis room (BM-07). The branch also had a `player_loaded`
chain that set `INT_Auction` at runtime; BM-07 seeds the bit on the
template instead, because the bit is now the authority marker and must
come from the seed (§3). Chain 5031 is reserved for an in-world
auctioneer. The branch's template and spawn ids (168 / 238) collided with
the Castle rebuild, so BM-07 used the Black Market block (305-309,
405-409). The action handler
(`crates/cell-content/src/cell/content/executor/black_market.rs`)
resolves the auctioneer entity id with the same precedence
`dialog::display` uses — chain `params["target_entity_id"]`, then the
player's `last_interaction_target` pin — and **aborts with a warn** if
neither resolves, rather than binding the window to the player's own id.
After it sends `onBMOpen` it records the session the §3 gate checks.

### 12. Telemetry: every transition and refusal is an event

The system is debuggable from SigNoz alone:

- **Payouts** (BM-02b) are INFO `bm.payout`, one per mail after the
  commit, with `reason` (`sold`, `buyout`, `expired`, `cancelled`,
  `outbid`), `role`, `recipient_player_id`, `mail_id`, `cash` and the
  item (`item_source`, `item_id`, `type_id`, `stack_size`), plus the
  actor's ids; the writer's `mail.system_sent` follows each. The sweep
  logs ERROR `bm.quarantined` and WARN `bm.settle_retry`.
- **Transitions** are DEBUG events — `bm.listed`, `bm.bid`,
  `bm.outbid_refund`, `bm.cancelled`, `bm.sold`, `bm.expired` — carrying
  `auction_id`, the seller and bidder ids, the bid and the escrowed cash
  before and after, `item_def_id`, and the actor's `account_id` and
  `player_id`.
- **Refusals** are `bm.refused` at INFO with `reason` (`BMError::reason`)
  and `error_id`, and every request counts on
  `bm_outcome_total{op, outcome}`.
- **Search** logs `bm.search` at INFO with `client_key`, the filters,
  `rows_read`, `rows_returned` and `total_results`.
- **The cell** logs `bm.decode_failed` (WARN, with `arg_len` and
  `reason`) and `bm.open` (DEBUG, the recorded auctioneer); at logout,
  `bm.open_without_client_call` (INFO) marks a player who was sent
  `onBMOpen` but whose client never called 61–66 — the sign the client
  patch is missing.
- **Sends:** each `onBM*` send logs `bm.send` at DEBUG with the method
  and payload size.

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

**Escrow by flagging the inventory row instead of moving it.** Rejected:
a flag leaves the instance in a player container, addressable by every
inventory path (move, equip, split, vendor-sell), so every one of them
would need a new "is this escrowed?" check, and any path that forgot one
would let a seller sell the item twice. Moving the row into container 18
reuses the checks those paths already have — their container allowlists
and the read filters — instead of adding a new one.

**Escrow by deleting the row and snapshotting it.** This was the Phase 1
design, and BM-02 replaced it. Deleting made the item unreachable by
construction and doubled as the ownership proof, but the snapshot
carried only four columns, so a returned or delivered item came back with
a new `item_id` and without its ammo or any column the snapshot missed.
It also could not feed the social-systems system-mail writer, which
BM-02b needs, because that writer mails an existing container-18 row.

**Returning cancelled and expired items straight to the bags.** This was
BM-02's delivery, and D-BM10 replaced it with mail. A direct return needs
a free slot: cancel refused on full bags, and the sweep, which cannot
refuse, placed the row past the main bag's last slot. It also meant a
second item-moving path beside the mail writer that every sale already
uses. Mail has neither problem, works for an offline seller, and keeps
one writer for everything an auction pays out; the cost is that the
seller takes the item from the mailbox instead of finding it in the bag.

**Settling a buyout at the next sweep.** Rejected by D8. The original
game settled a buyout instantly, and a buyer who paid the buyout price
should not wait up to 30 seconds and see the auction still listed. Once
settlement was factored into `settle.rs`, the bid path could call the
same function the sweep does, so there is one settlement and no second
payout path to keep in step.

**Applying every `BMSearchOptions` filter as a SQL predicate.** Only
`itemName` and `minTC` / `maxTC` are applied. `sortId` and `quality` are
logged, not applied, and `filterFlags` (the `.def` calls it
`monikerCRC`) is always 0 from the shipped UI, so its semantics are
unknown. Pushing guessed predicates into SQL would produce a result set
we could not verify against the client's rendering.

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
each one is either blocked on evidence we do not have or scheduled into a
named packet.

### Durations are design

`auction_length_seconds` (`wire/mod.rs`) maps the five `UIAuctionTime`
tiers to 12 / 24 / 48 / 72 / 96 hours. No source gives the shipped
durations, so this table is design, not recovery. It is isolated in one
named function (with `time_left_bucket` built on it), so replacing it is
a one-place edit — do not inline the hours at call sites.

The bid increment and the error ids used to share this status. They are
now decisions: the increment is 5 %, at least +1 (D6, `next_min_bid`),
and the ids are D3's `BMError`. Both are load-bearing — the client
displays `nextMinBidPrice` and the server enforces `required_min_bid`
in `validate_bid`, and the patch overlay maps each id to text — so a
change to either is a change to the shared codec and the plan, not a
local edit.

### Quarantined auctions need an operator

The sweep quarantines an auction it cannot settle (§8), but nothing
resolves one yet: there is no GM command, so an operator mails the
container-18 row and any held bid by hand and sets a final status. A
database error is retried every pass without a limit; only a failure
that will recur is quarantined.

### The character-delete trigger refunds directly

D-BM09's `bm_player_before_delete()` credits the standing bidders'
balances in SQL. A trigger cannot call the mail writer, so those refunds
are the one Black Market payout that is not mail.

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
arg-decode. The patch DLL does exactly that, by hand, with the
`cimmeria-patch-wire` decoder the server encodes with.

**Do not "fix" the encoder to emit WSTRING.** The narrow encoding is
wire-correct for a manual parser and matches the shipped field
descriptor; widening it would break the manual parser without making the
engine decoder work.

### Smaller gaps

- **Watch list (65/66, and method 95) is deferred (D4).** Both cell
  methods answer `onBMError(WatchUnavailable)` and hold no state, so
  `onBMWatchedItemsUpdate` is never sent. The `SGWBlackMarket` entity's
  one property (`watchedItems: PYTHON`, an itemDefId → subscriber
  registry) has no server-side equivalent yet; packet BM-08 picks it up
  once the core loop passes UAT.
- **Bind-on-acquire items are listable**, the same way they are
  tradeable and mailable. Bound rows are refused; items that bind on
  acquire are not bound yet while they sit in a bag. This is a systemic
  gap, not a Black Market one.
- **The seller is not told about bids.** `onBMAuctionUpdate` after a bid
  goes to the bidder and any online outbid player; the seller learns
  about it from the sweep or their next search.
- **No listing fee** (D5). The economy-sink design in
  [gameplay/black-market.md](../gameplay/black-market.md#economy-sink-design-unbuilt)
  waits on currency-flow instrumentation.
- **A prior bidder whose row is gone cannot be refunded.** The D-BM09
  trigger refunds bidders before a deletion, so this should not happen;
  if it does, `payout_mail::refund_standing_bid` logs WARN
  `bm.refund_skipped` and the bid or cancel proceeds rather than being
  blocked.

## Consequences

- **New schema**: `sgw_auction` + `sgw_auction_bid` and their sequences,
  under `db/sgw/BlackMarket/`, plus the `bm_player_before_delete()`
  function and its `sgw_player` trigger. No migration script — per repo
  convention the seed in `db/` is edited directly.
- **Container 18 is now live.** Escrowed rows sit in `sgw_inventory`
  with `container_id = 18`. Any new inventory path that reads rows for
  the client, or acts on a client-supplied `item_id`, must exclude
  container 18 or use a container allowlist that omits it; a path that
  does neither lets a seller touch an item that is up for auction.
- **A second writer of container 18 must follow the lock order** in §6:
  the seller's escrow advisory lock first, and never an advisory lock
  before an `sgw_auction` row lock.
- **A second wire consumer shares the codec.** `cimmeria-patch-wire` is
  built into both the server and the client-patch DLL, so it stays
  std-only, and a layout change there is a change to both sides at once.
- **New background task**: the expiry sweep is spawned at base startup
  (`crates/base/src/base/service.rs`) alongside the boot seed. Both are fire-and-forget `tokio::spawn`
  spawners so the caller need not be async, and they are benign if they
  race — seeded listings carry a future `expires_at`, so the sweep's
  first pass ignores them.
- **Reserved ids 1/1** in `account` / `sgw_player` are now permanently
  spoken for. Anything that enumerates players (rosters, leaderboards,
  GM listings) will see a "Black Market" player with no inventory, no
  missions, and no contact list.
- **Auction mail is system mail.** It is written only by the mail
  module's `send_system_mail_tx` (BM-02b), with `sender_id = NULL` and
  `sender_name = "Black Market"`, so it is not returnable and follows the
  mail module's expiry and quarantine rules.
- **Reusable helpers landed**: `adjust_player_cash` (`helpers.rs`) is
  deliberately generic and the right building block for other systems
  that move cash atomically (trade, guild bank).
- **Test coverage** is in
  `crates/base-methods/src/base/world_entry/methods/black_market/tests/`
  (live-DB: create/bid/cancel, buyout, search, sweep, escrow, settlement
  mail (exactly once, quarantine) and the deletion trigger; sentinels in the `0x7000_Axxx` block) plus in-module
  unit tests for the pure validators, the auctioneer gate, the wire
  serializers (byte-exact layout guards) and the seed's system-seller
  invariants. The codec's own `.def`-order and round-trip tests live in
  `crates/patch-wire/src/black_market/tests/`.
- **The feature is not player-visible on merge.** Server-side correctness
  buys nothing until the client patch of issue #587 ships; today only the
  window chrome opens (method 90, by hand-applied patch) and its tabs
  render empty.

## Confidence

| Area | Confidence | Basis |
|---|---|---|
| Cell/base split, state machine, escrow semantics, lock order | **High** | Code + unit and live-DB tests on `main` |
| Auctioneer gate (62–64) | **High** | Unit tests over `black_market_access`; server-authority review |
| Inbound wire layouts (61–64) | **High** | Ghidra emitter decompiles; the 13-byte `BMCreateAuction` and 11-field `BMSearchOptions` are corrections to earlier docs |
| Outbound layouts (`onBMAuctions` order, `clientKey`, `AuctionItem`) | **High** | Client IO decompiles ([black-market-client-io.md](../reverse-engineering/findings/black-market-client-io.md) §4); `AuctionItem` matches the client's 10 field descriptors at `0xEF770400`; one codec on both sides |
| Error ids, bid increment, listing cap, buyout | **High** (as decisions) | D3, D6, D5, D8 in the [restoration plan](../analysis/black-market/README.md); not recovered from the client, and not meant to be |
| Duration table | **Low** | Design; no source gives the shipped durations |
| COD/payout shape | **Medium** | Architecture-inferred from `sgw_gate_mail` + `payCODForMailMessage`; no Python reference implementation exists |
| Client-side binding diagnosis and patch | **High** | Owner-confirmed working in-world, 2026-06-21; byte-level descriptor comparison plus a live registry walk |

## See also

- [gameplay/black-market.md](../gameplay/black-market.md) — the system reference: what the auction house *does*, entity definitions, per-message wire tables, the error-id and rules tables, and current implementation status. This ADR is the complement — *why* the server is shaped the way it is.
- [analysis/black-market/README.md](../analysis/black-market/README.md) — the restoration plan: S1–S8, decisions D1–D8, and the packet sequence (BM-02b, BM-03 to BM-08)
- [black-market-client-io.md](../reverse-engineering/findings/black-market-client-io.md) — the client's own IO: the evidence behind S1–S8
- [black-market-restoration.md](../reverse-engineering/findings/black-market-restoration.md) — server-side RE, entity model, completeness assessment
- [black-market-wire-formats.md](../reverse-engineering/findings/black-market-wire-formats.md) — per-message wire tables
- [black-market-client-window-patch.md](../reverse-engineering/findings/black-market-client-window-patch.md) — the client binding gap, both patch recipes, and the fork-B build spec for methods 91–95
- [CAT-I-black-market.md](../security-audit/2026-05-31-server-authority/findings/CAT-I-black-market.md) — the six audit findings and their current status
