# CAT-I — Black Market / Auction House (server-authority audit)

> **Status re-verification (2026-07-25)** — this category has moved from
> "latent, post-implementation" to real code, but that code is **not yet on
> `origin/main`**. The Black Market Phases 1–3 live on
> `feat/571-black-market-phase1` (`crates/services/src/base/black_market/`,
> a directory that does not exist on `main`). Treat every "addressed" below
> as **fixed-but-unmerged** — it becomes live only when that branch merges,
> and the guards should be re-checked at merge time.
>
> - **CAT-I-01 (whole interface stubbed): superseded.** `BMSearch`,
>   `BMCreateAuction`, `BMPlaceBid`, and `BMCancelAuction` now have real
>   handlers with a shared validation module
>   (`black_market/validate.rs`). `BMStartWatchingItem` /
>   `BMStopWatchingItem` are still `UNIMPLEMENTED` stubs.
> - **CAT-I-02 (create): PARTIALLY addressed.** Ownership is enforced by
>   escrow — `validate_create` fails closed unless the escrow `DELETE`
>   matched a row the seller owned (`validate.rs:18-31`) — and prices are
>   floored at zero. Duration is bounded because it is an *enum*, not a
>   raw span: `auction_length_seconds` maps any out-of-range byte to 96 h
>   (`wire.rs:48-57`). **Still open**: no listing-fee deduction and no
>   per-player listing cap.
> - **CAT-I-03 (bid): addressed.** `validate_bid` checks
>   auction-active → not-self-bid → meets-`required_min_bid` →
>   sufficient-funds (`validate.rs:38-58`), and the whole bid runs in one
>   transaction with a row lock, refunding the prior bidder before holding
>   the new bid (`bid.rs:68-232`). Caveat: `next_min_bid` is a **guessed**
>   5 %-increment formula pending x64dbg capture (`wire.rs:59-67`), so the
>   floor may not match the client's.
> - **CAT-I-04 (cancel): addressed.** `validate_cancel` requires the caller
>   to be the seller and the auction to be active (`validate.rs:73`); item
>   return and bidder refund both run inside the same transaction
>   (`cancel.rs:39-187`).
> - **CAT-I-05 (search): STILL OPEN.** The search query has **no `LIMIT`**
>   and no result-size cap — `fetch_all` returns every matching row
>   (`search.rs:37-45`).
> - **CAT-I-06 (expiry sweep): addressed.** `black_market/sweep.rs`
>   implements the expiry pass with the COD-to-seller / item-return mail
>   cascade via `send_mail_to_player`.
>
> Note the cross-cutting dependency: CAT-I-03's "replayable" qualifier
> traces to **CAT-A-03**, which is still open. Replaying a captured bid at
> the same amount now fails the `BID_TOO_LOW` floor, so the specific dupe
> is closed, but the transport-level replay gap is not.

## Trust posture summary

The SGW Black Market / Auction House surface is **fully unimplemented**
server-side. Every CellMethod under `SGWBlackMarketManager` (indices 61–66 —
`BMSearch`, `BMCreateAuction`, `BMPlaceBid`, `BMCancelAuction`,
`BMStartWatchingItem`, `BMStopWatchingItem`) is a stub that decodes a
fixed slice of `args` with `i32::from_le_bytes`, emits a
`tracing::info!("UNIMPLEMENTED: …")`, and returns `true` to the dispatcher
(handler at `crates/services/src/cell/cell_methods/black_market.rs:14-80`).
No DB tables back the surface (no `auctions` / `bm_listings` / `bm_bids`
in `db/database.sql` or anywhere under `db/`), no `ON_BM_*` client method
is ever invoked by the server, no item lock, no currency deduction, no
listing-belongs-to-caller check, no expiry sweep, no COD-to-seller
cascade.

This is a security-relevant state for two reasons:

1. The wire surface is **fully exposed** — Ghidra confirms the client
   constructs and emits `Event_NetOut_BMCreateAuction`,
   `Event_NetOut_BMPlaceBid`, `Event_NetOut_BMCancelAuction`, and
   `Event_NetOut_BMSearch` with the full intended payload shapes (see
   per-finding evidence). Mercury accepts the frames, the dispatcher
   routes them, and the stub parses them. A scriptable client can land
   bytes at the handler today; only the absence of a state-mutation body
   prevents the exploits below. Whoever wires the handler bodies next
   inherits a wire surface that *already accepts* every client-asserted
   field (item instance id, starting price, buyout price, auction
   length, sequence id, bid amount, sort id, client key, filter flags,
   bForward, seller/bidder/item names) with no validation plumbed in
   and no DB schema to enforce ownership / atomicity. That is the
   exploit-chain-on-merge shape that CAT-G called out for mail; CAT-I
   is the same shape with a much larger blast radius because auctions
   are the primary economy surface (currency + item flow between
   players, multi-step transactions, time-based expiry, COD-to-seller).
2. The stub's wire decoding is **already wrong** in ways that will leak
   into the future implementation if not fixed at the decoder layer:
   - `BMCreateAuction` reads `auction_length` as a 4-byte LE i32, but
     Ghidra shows the client emits it as a 1-byte field (Mercury
     property-tree encoding picks the type from the property
     registry; the emit at `00e59970` passes `param_4` as a `char`
     widened into the byte slot — see CAT-I-02 evidence).
   - `BMPlaceBid` and `BMCancelAuction` use the field name
     `auction_id` (i32) in Rust, but Ghidra shows the client wire field
     is `sequenceId` and the cancel path emits it as a `char` (i8 —
     `FUN_00e59c70`, param_1 is `char`), while the place-bid path
     emits it as a wider int (`FUN_00e59da0`, param_1 is `char` but
     `param_2` is `uint`). The "auction id" identifier is **not a
     32-bit row id** on the wire — the client's "sequenceId" appears
     to be a per-listing index into a paginated result set, not a
     stable DB key. Any future implementation that treats it as a row
     id will dereference attacker-chosen indices.

The dominant finding for this category is therefore "an entire economy
vertical will land as an exploit chain the moment it is implemented".
The remaining findings nail down each specific trust gap that the
implementation must close, with evidence anchored in the client's
Ghidra-decoded emit functions so the implementer cannot accidentally
re-derive a wrong invariant.

---

### CAT-I-01 — Entire SGWBlackMarketManager interface is a stub that accepts every client field unvalidated

**Severity**: High (latent — becomes Critical when handler bodies are filled in without validation)
**Class**: Missing handler / silent client trust / latent exploit surface
**Wire surface**: `Event_NetOut_BMSearch`, `Event_NetOut_BMCreateAuction`, `Event_NetOut_BMPlaceBid`, `Event_NetOut_BMCancelAuction` (CellMethod indices 61–64), plus indices 65–66 (`BMStartWatchingItem` / `BMStopWatchingItem` — no `Event_NetOut_*` discovered in the binary for those, may be invoked via a different path).
**Demonstrable / Likely-theoretical**: Demonstrable (Ghidra confirms the emits; server confirms the stub).

**Trust violation**
The dispatcher at `crates/services/src/cell/cell_methods/black_market.rs`
returns `true` (handled) from every arm but performs **no** state
mutation, **no** validation, **no** reply, and the surface has **no**
backing DB schema. The wire is open, the client can land bytes at the
handler, the stub silently absorbs them. A future implementer who fills
in the body will, by default, build a wire-trust violation unless the
audit catches it at design time. The five subsequent CAT-I findings
(02–06) enumerate the specific server-authority invariants that the
implementation must enforce; they are filed pre-emptively because the
wire surface already accepts every field a finished feature would need
to mis-trust.

**Evidence**
- Ghidra: `019dd370` — `Event_NetOut_BMCreateAuction` string; emit
  registration at `00e5c200` (`register_NetOut_BMCreateAuction`); the
  actual emit constructor at `00e59970` (`FUN_00e59970`) packs
  `itemInstanceId`, `startingPrice`, `buyoutPrice`, `auctionLength`
  into a Mercury property tree and dispatches.
- Ghidra: `019dd3e8` — `Event_NetOut_BMPlaceBid` string; emit
  constructor at `00e59da0` packs `sequenceId` (auction selector) and
  `bidAmount`.
- Ghidra: `019dd3ac` — `Event_NetOut_BMCancelAuction` string; emit
  constructor at `00e59c70` packs `sequenceId` only.
- Ghidra: `019dd41c` — `Event_NetOut_BMSearch` string; emit
  constructor at `00e59f70` packs `sortId`, `clientKey`,
  `sequenceId`, `bForward`, `sellerName`, `bidderName`, `itemName`,
  `minTC`, `maxTC`, `quality`, `filterFlags`.
- Client behavioral log: n/a (BM UI has never been triggered in
  captured logs).
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/black_market.rs:14-80`.

**Attack scenario**
1. The category is currently dormant — exploit requires the handler
   bodies to be filled in.
2. The "attack" today is the *latent* one: an implementer writes
   `BMCreateAuction` that consumes the client's `itemInstanceId` and
   `startingPrice` directly, ships, and ships a dupe / free-listing
   bug in the same patch because the audit didn't pre-flag the
   invariants.
3. Observable effect: economic catastrophe at activation time
   (CAT-I-02..06 each cite a different concrete exploit shape).

**Suggested remediation (one line)**
Land the per-handler authority invariants (CAT-I-02..06) as
comments-as-acceptance-criteria on `black_market.rs` *before* anyone
fills in a body; reject implementation PRs that mutate auction state
without citing the specific invariant they enforce.

**Would benefit from x64dbg trace?**
No — wire shapes are already pinned by Ghidra; the gap is server-side.

---

### CAT-I-02 — BMCreateAuction: no item ownership / tradeable / lock validation, no listing-fee deduction, no duration cap, no per-player cap

**Severity**: Critical (post-implementation) / High (latent)
**Class**: TOCTOU-shaped dupe, missing ownership check, missing currency deduction, missing rate-limit
**Wire surface**: `Event_NetOut_BMCreateAuction` (CellMethod index 62)
**Demonstrable / Likely-theoretical**: Likely-theoretical (the handler is a stub; the trust violation is the *absence of any validation in the wire shape*).

**Trust violation**
The client emits four fields — `itemInstanceId` (u32, the item to
list), `startingPrice` (i32), `buyoutPrice` (i32), `auctionLength`
(u8) — and the server must independently verify *all four* before
creating any persisted listing row. The required server-side checks,
in order:

1. **Item ownership**: server resolves the caller's character_id and
   confirms `itemInstanceId` lives in *that* character's inventory at
   the moment of the create call. The wire field is just a raw u32; a
   modified client can supply any item id it has ever seen (a friend's
   item, a vendor's stock id, a dropped-loot id). The handler MUST NOT
   key off the client-supplied id without an ownership join.
2. **Item is tradeable / unequipped / not soulbound / not container
   non-empty**: same shape — these are item properties the server
   must look up, not trust the client to have pre-filtered.
3. **Atomic lock-or-remove of the item at create time**: the item row
   must transition `owner=character_id → owner=null,
   reserved_by=auction_id` (or be moved to an auction-escrow
   container) inside the same transaction as the listing row insert.
   If this is two separate statements, a disconnect / concurrent
   `MoveItem` / `UseItem` between them is a dupe. This is the same
   TOCTOU shape that bandolier ammo type_id-vs-item_id had — keyed
   on the wrong row, the wrong record gets mutated.
4. **Listing-fee deduction from naqahdah** in the same transaction —
   client-side debit display is cosmetic.
5. **`startingPrice`, `buyoutPrice` bounds**: both must be > 0; on
   the wire they are signed i32 (the client packs them as
   `undefined4` per `FUN_00e59970`), so negative values are
   trivially injectable. `buyoutPrice >= startingPrice` (or
   `buyoutPrice == 0` for "no buyout") must be a server check.
   Without `i64`-cast bounds, multiplications during fee-percentage
   calculation overflow.
6. **`auctionLength` cap**: server-side max (e.g. 7 days). The wire
   field is a u8 (Ghidra: `FUN_00e59970` passes `param_4` as
   `param_2 = param_4; … FUN_00a4fb70(this,…)` — the second
   property-tree write at offset for `auctionLength` is a byte
   widening, not a 32-bit slot), so the maximum literal value is
   255 — but 255 days is still well past any reasonable cap. Note
   the current Rust stub reads this as a 4-byte i32 (bug — see
   below).
7. **Per-player listing cap**: a single attacker can DoS the listing
   table with infinite create calls; this needs an `active_listings
   < N`-per-character check at handler entry.

**Evidence**
- Ghidra: `00e59970` `FUN_00e59970` — the BMCreateAuction emit
  constructor. Field order: `itemInstanceId` (u32, prop name
  `"itemInstanceId"`), `startingPrice` (u32, prop name
  `"startingPrice"`), `buyoutPrice` (i32, prop name `"buyoutPrice"`),
  `auctionLength` (byte widened; prop name `"auctionLength"`).
- Ghidra: `019dd370` — `Event_NetOut_BMCreateAuction` string anchor.
- Ghidra: `019dd078` `auctionItems` / `019dd0e4` `buyoutPrice` /
  `019dd0f0` `auctionLength` — string anchors confirming prop names.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/black_market.rs:27-43`. Note
  the current stub names the field `duration_days` and decodes 16
  bytes as four i32s; the wire shape is `u32 + u32 + i32 + u8`, so
  the offset math will collide with the real Mercury frame when the
  handler is wired. Fix the wire decoder first; then enforce the
  validations above.

**Attack scenario** (post-implementation, given a naive port)
1. Attacker scripts a client that emits `BMCreateAuction` with
   `itemInstanceId` = the id of a high-value item that lives in a
   friend's inventory (observable via party / `Show*` GM commands /
   linkshell sharing). `startingPrice` = `-2147483647`.
2. Server creates a listing keyed on the borrowed item id; if it
   then proceeds to delete-from-inventory on the wrong character's
   row (via item_id alone, no ownership join), the friend's item is
   consumed. Even if the delete fails, the listing row exists and a
   colluder can bid `0` (negative-`startingPrice` allows `0` to win
   the buyout check) and "purchase" the item from the listing,
   netting it without ever owning it.
3. Observable effect: dupe + cross-player item theft.

**Suggested remediation (one line)**
Implement the create path as a single transaction that joins the
inventory row by `(character_id, item_id)`, locks-or-moves the item,
deducts the listing fee, inserts the listing row, and runs every
field-bound check above before the COMMIT; reject any
`startingPrice <= 0`, `buyoutPrice < startingPrice` (when
non-zero), `auctionLength > 7`, `active_listings >= N` at the
handler entry.

**Would benefit from x64dbg trace?**
No — wire shape and field names confirmed by Ghidra; the gap is
in the absent server implementation.

---

### CAT-I-03 — BMPlaceBid: no current-bid comparison, no naqahdah hold, no self-bid prevention, no listing-visibility check, replayable

**Severity**: Critical (post-implementation) / High (latent)
**Class**: Bid-integrity TOCTOU, replay, currency overdraft, self-bid dupe
**Wire surface**: `Event_NetOut_BMPlaceBid` (CellMethod index 63)
**Demonstrable / Likely-theoretical**: Likely-theoretical (handler is a stub).

**Trust violation**
The client emits `sequenceId` (auction selector — see below) and
`bidAmount` (u32 on the wire). Required server-side checks:

1. **Server-side `bidAmount > current_high_bid + min_increment`** —
   the client's emit at `FUN_00e59da0` has a *client-side* sanity
   check (`if ((int)param_2 <= *(int *)(*(int *)(*(int *)(iVar1 + 0x8c) + 0x24) + 0x60))` —
   comparing the bid to what appears to be the caller's naqahdah
   balance), but that gate lives entirely in the client and is
   trivially bypassed by a modified emit. The server must
   independently compare against the listing's persisted
   `current_bid`, not against any client-asserted value.
2. **Currency hold at bid time**, not at win time. The standard
   auction-house pattern is: deduct `bidAmount` from the bidder
   immediately, refund the previous high bidder. Any deferred-debit
   model lets the bidder spend the same naqahdah elsewhere between
   bid-place and bid-resolve, and the auction settles in the
   server's favor (item never delivered, currency never collected).
3. **Self-bid prevention** — the listing's seller_character_id must
   not equal the bidder's character_id. Otherwise the seller bids
   on their own listing, "wins", and the listing-fee + bid-fee
   round-trip is a small naqahdah sink the seller can use to
   launder funds across mules (or, more dangerously, the
   collusion partner who placed a real bid is refunded but the
   item still goes to the seller — depends on the resolution
   logic).
4. **Listing-visibility check**: `sequenceId` is the client's
   *paginated index* into the search result set (see CAT-I-05 —
   the BMSearch emit packs `clientKey` and `sequenceId` together,
   strongly suggesting the auction selector is a per-search
   cursor, not a stable row id). The server MUST translate
   `sequenceId` through the caller's most recent search-result
   cache (keyed by `clientKey`) before mutating; otherwise the
   client can supply any integer and the server dereferences an
   arbitrary listing the player was never authorized to see
   (cross-faction listing, GM-only listing, removed-but-not-
   reaped listing). A client that has never opened the BM UI can
   place bids on any listing by enumerating `sequenceId`.
5. **Replay**: the framing layer's per-tick authenticate token
   and 512-entry dedup hash (spec §1.7) defends against bit-for-
   bit packet replay, but the bid handler must *also* be
   idempotent within an in-flight tick — two `BMPlaceBid` calls
   with the same `(character_id, sequenceId, bidAmount)` in the
   same tick must produce one persisted bid, not two. The stub
   has neither.
6. **Race condition on equal-amount concurrent bids**: two
   clients submitting the same `bidAmount = current_bid +
   min_increment` simultaneously must serialize through a row
   lock on the listing; the loser's currency must be refunded
   atomically. Without `SELECT … FOR UPDATE` (or equivalent) on
   the listing row, both bidders' currency gets deducted and
   only one wins, netting a free naqahdah burn.

**Evidence**
- Ghidra: `00e59da0` `FUN_00e59da0` — the BMPlaceBid emit
  constructor. Field order: `sequenceId` (i8 widened to int
  per the prop-tree write; prop name `"sequenceId"`),
  `bidAmount` (u32, prop name `"bidAmount"`). The client-side
  balance gate is `param_2 <= *(*(*(*(iVar1+0x8c)+0x24)+0x60)`.
- Ghidra: `019dd3e8` — `Event_NetOut_BMPlaceBid` string anchor.
- Ghidra: `019dd118` `bidAmount` — string anchor.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/black_market.rs:44-56`.
  The stub names the wire field `auction_id` and decodes it as
  i32 — the wire field is `sequenceId` and is a paginated
  index, not a row id.

**Attack scenario** (post-implementation, given a naive port)
1. Attacker scripts a client that emits `BMPlaceBid` with
   `sequenceId` chosen by brute-forcing 0..N until a listing
   responds; `bidAmount = 1`. If the server treats sequenceId
   as a row id and skips the current-bid comparison, the
   listing is "won" for a single naqahdah.
2. Variant: attacker scripts the same emit with `bidAmount =
   currentBid` (read from a prior `Event_NetIn_BMAuctions`
   payload) — if the server compares against
   *attacker-asserted* current bid (stored client-side after
   the search), the attacker simply lies about the current
   bid value.
3. Variant: attacker lists an item via mule, scripts the mule
   to bid against itself with the buyout amount; the listing
   settles immediately and the mule pockets the buyout
   minus the listing fee (less than 100%), turning the BM
   into a per-listing-fee laundering surface.
4. Observable effect: bid-integrity collapse, free items,
   currency laundering, listing-discovery bypass.

**Suggested remediation (one line)**
Translate `sequenceId` through a caller-scoped (`clientKey`-
matching) search-result cache, lock the listing row, validate
`bidAmount > current_bid + min_increment`, deduct from
bidder naqahdah and refund prior high bidder atomically, and
reject `seller_character_id == bidder_character_id`.

**Would benefit from x64dbg trace?**
Yes — confirming the wire type width of `sequenceId` (i8 vs i32 vs
"the prop-tree encoder writes whatever fits") needs a live capture
of one `BMPlaceBid` packet to lock the decoder before writing the
handler. The Ghidra-decoded `param_1` being `char` is suggestive
but the prop-tree write may widen it.

---

### CAT-I-04 — BMCancelAuction: no ownership check, no bidder refund, no item-return atomicity

**Severity**: Critical (post-implementation) / High (latent)
**Class**: Cross-player listing pull, dupe via cancel-with-active-bid
**Wire surface**: `Event_NetOut_BMCancelAuction` (CellMethod index 64)
**Demonstrable / Likely-theoretical**: Likely-theoretical (handler is a stub).

**Trust violation**
The client emits a single field — `sequenceId` (i8 / paginated
index — see CAT-I-03 for the type-width caveat). Required
server-side checks:

1. **Ownership**: the listing being cancelled must have
   `seller_character_id = caller_character_id`. Without this, a
   modified client can cancel any visible listing, denying
   sale to anyone in the marketplace.
2. **Active-bid handling**: if the listing has a `current_bid`,
   the bid currency must be returned to the prior high bidder
   *before* the listing is closed. Otherwise a seller who sees
   their listing about to settle for less than they'd like can
   cancel, pocket nothing, the bidder's naqahdah is forfeit, and
   the bidder has no recourse. (Or the seller may also need a
   listing-fee forfeiture penalty to prevent the inverse
   griefing — a server-side policy decision, but the
   *currency-return-to-bidder* invariant is non-negotiable.)
3. **Item return atomicity**: the escrowed item must be moved
   back to the seller's inventory in the same transaction as
   the listing-row delete. A partial commit (item already
   removed from escrow, listing still exists) and a partial
   commit (item never returned, listing already deleted) are
   both dupe-shaped: the former lets a future
   create-from-escrow re-list the same item via a sibling
   listing if escrow keys collide; the latter is a
   straight-up item loss recoverable via GM intervention but
   noisy.
4. **Disconnect-mid-cancel**: same shape as the trade
   disconnect-timing dupe — if the cancel handler commits
   item-return-to-seller before commit-listing-delete, a
   well-timed client disconnect between the two writes lets
   the seller keep the item AND the listing remains live, so a
   colluder can bid on it and "win" the still-escrowed copy.

**Evidence**
- Ghidra: `00e59c70` `FUN_00e59c70` — the BMCancelAuction emit
  constructor. Single field: `sequenceId` (`param_1: char`, prop
  name `"sequenceId"`). No ownership marker on the wire — the
  server must derive that from the caller's session.
- Ghidra: `019dd3ac` — `Event_NetOut_BMCancelAuction` string anchor.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/black_market.rs:57-63`.

**Attack scenario** (post-implementation, given a naive port)
1. Attacker scripts a client that emits `BMCancelAuction` with
   `sequenceId` from a competitor's listing observed via a
   normal `BMSearch`. Server obeys, the competitor's listing
   dies, the competitor's item is left in escrow forever.
2. Variant: attacker times a disconnect between item-return
   and listing-delete on their own listing to keep both the
   escrowed item and an active listing pointing at it.
3. Observable effect: market-wide denial of sale (variant 1)
   or self-dupe (variant 2).

**Suggested remediation (one line)**
Cancel handler: `WHERE seller_character_id = :caller AND
sequence_id = :sequenceId` on the listing fetch, single
transaction wrapping item-return, bidder-refund (if any),
listing-delete; rollback on disconnect-before-commit.

**Would benefit from x64dbg trace?**
No — wire shape is minimal and Ghidra-confirmed.

---

### CAT-I-05 — BMSearch: no result-size cap, no faction / region filter enforcement, paginated `clientKey` is a client-asserted cursor

**Severity**: High (post-implementation) / Medium (latent — DoS surface today)
**Class**: Search-result-DoS, cross-faction listing leak, client-trusted cursor
**Wire surface**: `Event_NetOut_BMSearch` (CellMethod index 61)
**Demonstrable / Likely-theoretical**: Likely-theoretical (handler is a stub today; the wire shape is fully exposed and the stub returns `true` immediately).

**Trust violation**
The client emits a *very* rich filter set — `sortId`, `clientKey`,
`sequenceId`, `bForward`, `sellerName`, `bidderName`, `itemName`,
`minTC`, `maxTC`, `quality`, `filterFlags`. The required
server-side properties:

1. **Result-size cap server-side**: the client cannot ask for
   "all listings". A modified emit can set `filterFlags` to a
   value that disables every filter and the server must still
   only return e.g. the first 50 rows. Without a server-side
   cap, a single search can pull the entire listing table —
   classic search-DoS shape.
2. **Faction / region / level-gate enforcement**: SGW
   maintains faction-based content gating. If the BM is faction-
   scoped (Lucian Alliance can't see Tau'ri listings, etc.),
   that scope must be applied server-side by joining caller
   faction against listing faction; client-supplied
   `filterFlags` cannot be the only gate.
3. **`clientKey` is a client-asserted pagination cursor** —
   the emit packs `clientKey` (likely a per-search
   correlation id the server hands back in
   `Event_NetIn_BMAuctions` so subsequent paged calls can be
   matched against the cached result set). The server must
   not trust the `clientKey` value to point at any session's
   cache other than the caller's own. If the cache is keyed
   by `(clientKey)` alone (not `(character_id, clientKey)`),
   a colluder can read another player's result page — minor
   leak but real.
4. **`itemName` / `sellerName` / `bidderName` are wide
   strings**: the wire emit packs them as `wchar_t*` (per
   `FUN_0043d380` at offsets `puVar1 + 0` (seller),
   `puVar1 + 7` (bidder), `puVar1 + 0xe` (item)). These
   flow into a `WHERE name LIKE :pattern` shape; without
   server-side length caps and pattern-escape rules, the
   query is a polynomial-time string-search amplification
   vector.

**Evidence**
- Ghidra: `00e59f70` `FUN_00e59f70` — the BMSearch emit
  constructor. Fields (in emit order): `sortId` (u32, prop
  name `"sortId"`), `clientKey` (u32, prop name
  `"clientKey"`), `sequenceId` (u32, prop name
  `"sequenceId"`), `bForward` (bool, prop name
  `"bForward"`), `sellerName` (wchar_t*, prop name
  `"sellerName"`), `bidderName` (wchar_t*, prop name
  `"bidderName"`), `itemName` (wchar_t*, prop name
  `"itemName"`), `minTC` (u32, prop name `"minTC"`),
  `maxTC` (u32, prop name `"maxTC"`), `quality` (u32,
  prop name `"quality"`), `filterFlags` (u32, prop name
  `"filterFlags"`). Eleven fields, all driven from a
  caller-controlled struct passed in `param_1`.
- Ghidra: `019dd41c` — `Event_NetOut_BMSearch` string anchor.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/black_market.rs:22-26`.
  The stub does not even attempt to parse `args` — comment
  `"BMSearchOptions is a complex struct, just log arrival"`.

**Attack scenario** (post-implementation, given a naive port)
1. Attacker emits `BMSearch` with all filter strings empty,
   `filterFlags = 0xFFFFFFFF`, and a `sortId` that the
   server doesn't recognize (so server returns
   unfiltered). Server returns the entire listings table.
2. Variant: attacker emits `BMSearch` with a
   pathological `itemName` value (e.g. a long `%` -heavy
   wildcard pattern) and observes a CPU-bound DB response.
3. Variant: attacker emits `BMSearch` with `clientKey`
   read from a packet capture of another player's session
   and reads that player's paginated result cache.
4. Observable effect: server-side DoS (1, 2) or minor
   cross-player query-cache leak (3).

**Suggested remediation (one line)**
Cap result count server-side (e.g. 50 rows / page);
enforce faction / region scope by joining caller session;
key the result cache by `(character_id, clientKey)` not
`clientKey` alone; bound all name fields to a max length
and escape `%` / `_` before injecting into `LIKE`.

**Would benefit from x64dbg trace?**
Yes — locking the exact prop-tree decoding of the eleven
fields requires one captured `BMSearch` packet to confirm
type widths before writing the decoder.

---

### CAT-I-06 — Expiry sweep / COD-to-seller cascade is unimplemented and unscaffolded

**Severity**: Critical (post-implementation) / High (latent)
**Class**: Missing server-tick, missing atomicity boundary, item / currency loss
**Wire surface**: None client-side — this is a server-side scheduled-task gap. Surfaces back to the client via the `Event_NetIn_BMAuctionRemove` / `Event_NetIn_BMAuctionUpdate` reply paths (Ghidra string anchors `019bd874` and `019bd858`).
**Demonstrable / Likely-theoretical**: Likely-theoretical (the server tick is absent — there is no auction expiry task in `crates/services/src/` at all; grep returns zero hits for `auction`/`black_market` outside the stub).

**Trust violation**
Auction houses are not just request/response surfaces — they
require a server-tick sweep that closes expired listings,
delivers the winning bid's currency to the seller, delivers
the item to the winner, returns the item to the seller on a
no-bid expiry, and ages out abandoned listings. None of this
exists. Required invariants:

1. **Expiry-detection tick** — a periodic task that selects
   listings with `expires_at <= now()` and processes them.
   The current crate ships no scheduler for this.
2. **Atomic resolution per listing**:
   - With a bid: deduct nothing further from bidder (currency
     was held at bid time per CAT-I-03), credit seller with
     bid amount minus market fee, move escrowed item to
     winner's inventory, emit `ON_BM_AUCTION_REMOVE` (index
     93) to the winner and seller. All in one transaction.
   - Without a bid: return escrowed item to seller, refund
     any partial listing-fee policy decides to refund, emit
     `ON_BM_AUCTION_REMOVE`.
3. **Crash-resilient delivery** — if the server crashes mid-
   resolution, on restart the surviving "item is in escrow,
   listing is closed, winner has not been credited" state
   must be detectable and replayable. This is the same
   shape as the mail outbox pattern: an outbox row holds the
   pending delivery until it commits to the recipient. The
   stub lays no groundwork for this.
4. **COD-to-seller cascade**: if the auction model includes
   a COD shape (cash-on-delivery), the winner's bid currency
   must flow to the seller in a way that survives a winner
   disconnect during pickup. The cleanest pattern is "credit
   the seller's mail with currency on auction settle" so the
   currency-delivery surface is the existing mail vertical
   — but CAT-G found that mail itself is unimplemented, so
   the BM expiry cascade has no settled delivery surface
   today.

**Evidence**
- Ghidra: `019bd874` — `Event_NetIn_BMAuctionRemove` string anchor
  (server-to-client notification of listing closure — the trigger
  that the expiry sweep must emit).
- Ghidra: `019bd858` — `Event_NetIn_BMAuctionUpdate` string anchor
  (server-to-client listing-state change — needed for the
  "your listing just got outbid" / "your bid was beaten" UX).
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/client_methods/black_market.rs:9-13`.
  Server-side scheduler: **absent**. No `cell_methods/black_market.rs`
  callee references a Tokio interval or a periodic-task harness.

**Attack scenario** (post-implementation, given a naive port without
the expiry sweep)
1. Attacker creates a listing, waits past the duration, observes
   that the server never closes it. If a colluder bids during this
   window, the bid currency is held forever (per CAT-I-03) and the
   listing never settles. Either the attacker or the colluder can
   then file a GM ticket claiming the listing should have closed,
   and the GM workaround manually delivers — at scale, this is a
   support-cost amplifier and a route to GM-error dupes.
2. Variant: attacker creates a listing, lets a colluder win,
   then crashes the server (or just waits for a real crash);
   the resolution is mid-flight, item is in escrow, winner has
   not been credited. On restart, with no outbox / recovery
   step, the colluder gets the item but also gets the bid
   refund (or the seller gets the bid amount but also gets the
   item back) — either side wins double depending on which
   half of the resolution committed first.
3. Observable effect: market stalls on expiry, support-cost
   amplification, dupe windows on every server restart with
   active auctions.

**Suggested remediation (one line)**
Land an auction-expiry tick (Tokio periodic) and an
outbox-pattern resolution path *before* the create/bid/cancel
handlers are wired, so the wire surface cannot accept
listings the server has no path to close; back currency /
item delivery via the existing mail surface (once that is
itself implemented per CAT-G).

**Would benefit from x64dbg trace?**
No — this is a server-architecture gap, not a wire-trust one.

---

## Not Filed

- **"BMStartWatchingItem / BMStopWatchingItem (indices 65–66)
  trust violations"** — these stubs exist in the dispatcher and
  decode an `auction_id: i32` from `args`, but no
  `Event_NetOut_BMStartWatchingItem` or `Event_NetOut_BMStopWatchingItem`
  string was found in the binary via Ghidra string search. The
  client may invoke these via a different code path (e.g. an
  `onBM*` UI action that routes through a generic property
  setter), or they may be planned-but-unused indices. Without an
  emit trace I cannot characterize the wire-trust surface, and
  the surface is read-only (a "watching" relation is a per-player
  bookmark with no currency or item impact). Not filed — re-
  examine when the BM UI is exercised live and the actual emit
  path lands in a capture.
- **"Server-side stub args-length floor is undersized for the
  real Mercury frame"** — the current stub does
  `if args.len() >= 16` for `CREATE_AUCTION`, `>= 8` for
  `PLACE_BID`, `>= 4` for `CANCEL_AUCTION`. Per Ghidra the wire
  shape carries a Mercury property tree with name/length-tagged
  fields (not a flat 4-byte i32 array), so the byte budget is
  much larger. Not filed as a security finding because the
  stub never acts on the parsed values — it is a *correctness*
  issue that will be caught the moment a handler body is
  written. Mentioned inside CAT-I-02 / CAT-I-03 for the
  implementer's benefit.
- **"`Event_NetIn_BMOpen` reply path is unimplemented (server
  never opens the BM UI)"** — true, but the absence of a server-
  to-client open notification is a feature-completeness gap,
  not a server-authority gap. The wire surface still accepts
  `Event_NetOut_BM*` calls regardless of whether the client UI
  was opened, so a scripted client doesn't need the open
  notification. Mentioned in the summary; not a separate
  finding.
- **"The BMSearch stub doesn't decode `args` at all so it
  cannot mis-validate"** — true, but the absence of a parser
  is *also* the absence of validation. The latent surface is
  what CAT-I-05 captures; an additional finding here would
  duplicate it.
- **"GM-gate the BM dispatch entirely until implementation
  lands"** — considered, but the BM is not a GM surface; the
  fix is to land the handlers correctly, not to lock them
  out. A "feature gate per-build" suggestion sits outside the
  server-authority audit scope. Routed to the implementation
  PR's design discussion instead.
