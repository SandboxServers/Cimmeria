# Black Market Restoration: Path to a Shippable Client Patch

> **Status**: Accepted 2026-09-26. The decisions in §6 were answered in session (all as recommended). Packets are starting.
> **Tracking**: #571 (feature), #587 (launcher patch, which this plan widens), PR #586 (server branch, stale since 2026-06-22).
> **Evidence**: [black-market-client-io.md](../../reverse-engineering/findings/black-market-client-io.md) (this pass), [black-market-client-window-patch.md](../../reverse-engineering/findings/black-market-client-window-patch.md), [docs/architecture/black-market.md](../../architecture/black-market.md) (server ADR).

## 1. Where things stand

| Layer | State |
|---|---|
| Server | Search, create, bid, cancel, expiry sweep and escrow are built and tested on `feat/571-black-market-phase1`, but not merged. The branch predates the crate split (#825), so every file moves. Its wire contract has four mismatches with the client (§3.1). `main` has only `UNIMPLEMENTED` stubs. |
| Client binding | All six server→client methods (90–95) are silently dropped by the dispatcher. The client→server NetOut emitters are shelved, with no CME subscriber. The only fix so far is a hand-applied x64dbg patch that opens the window (method 90). Four attempts at the NetOut side crashed the client. |
| Client UI | `BlackMarket.lua` and `BlackMarket.layout` are unfinished: no bid button handler, several nil-variable errors, and two tabs without row widgets. See defects U1–U12 in the evidence doc. |
| Delivery | #587 plans for the launcher to write code caves into the process. Nothing has been built. |

## 2. What changed in this pass

The evidence doc replaces the two hardest parts of the old plan with smaller ones:

- **Sending no longer goes through CME.** Native code on the main thread can call `ServerConnection::startEntityMessage(conn, 0x3D, 0)`, write the sub-index byte and the payload through the bundle's `reserve`, and the message goes out on the next tick. This is the call every working NetOut ends in. It removes the whole registration chain that crashed four times.
- **Receiving needs no hand-built dispatch nodes.** A detour on the dispatcher (`0x00c6f8f0`) plus the drop callee (`0x01590f30`) sees the stream and the resolved `MethodDescription`. So it can match on the method *name* and read the arguments itself.
- **Lua can be driven from Rust.** `lua51.dll` exports the full wide-character C API under mangled names. Native Lua functions and table pushes replace `Lua_doString_wide` string-building and hand-written caves.
- **The contract gaps close:** the view and duration enums, `clientKey` semantics, `onBMAuctions` argument order, and `BMCreateAuction` wire order.

## 3. Proposed architecture

Three layers ship together. Each has one job.

```text
 server (Rust)            patch DLL (Rust, i686)                   UI overlay (Lua + layout)
 ─────────────            ──────────────────────                   ─────────────────────────
 onBM* 90–95  ──wire──▶  network thread: dispatcher + drop hooks
                          decode by method name ─▶ queue
                          main thread (Tick): pop ─▶ lua C API ──▶  CimmeriaBM.onAuctions(...)
                                                                    Lua store, views, rows
 cell 61–66  ◀──wire──  CimmeriaBM.search/create/bid/cancel   ◀──  button handlers
                          startEntityMessage + payload
```

### 3.1 Server: rebase `feat/571` and fix the contract

Port the branch onto the split crates and keep its tests. Fix these contract points in the same pass. Each one is a client fact from the evidence doc, not a guess:

| # | Fix | Evidence |
|---|---|---|
| S1 | `onBMAuctions` sends `(items, totalResults, clientKey)`, the `.def` order. The branch sends `(items, view, total)` | `.def`; `FUN_00ae3130`/`FUN_00ae31f0` |
| S2 | Echo `clientKey` (0 Search / 1 MyAuctions / 2 MyBids), not `sortId` | `EBlackMarketSearchType`, `refreshMyAuctions` / `refreshMyBids` |
| S3 | For `clientKey` 1 and 2, filter by the **caller's** player id and ignore the client's `sellerName`/`bidderName` strings | server authority; the strings are only the client's own name |
| S4 | Decode `BMCreateAuction` in `.def` order: `item, buyout, length:u8, starting`. The branch decodes `item, starting, buyout, length` | evidence doc §4 |
| S5 | `auctionLength` is 1–5 (`UIAuctionTime`), and the UI sends 3/4/5. Map 1-based; the branch maps 0-based | tolua constants |
| S6 | `endTimeValue` is a time-left bucket 1–5, not the listing duration | `getAuctionItemInfo.timeLeft` drives the row timer |
| S7 | Bound the search (`LIMIT` + `sequenceId`/`bForward` cursor) and return the real `totalResults` | CAT-I-05; the UI pages 8 rows at a time |
| S8 | Resolve seller names from the DB, not from the online-session map | ADR "Smaller gaps" |
| S9 | Send sweep and buyout payouts (cash to the seller, item to the buyer, unsold item back to the seller) through the social-systems mail API, and drop the branch's own `send_mail_to_player` helper, so the codebase has one mail writer | Agreed with the social-systems coordinator, 2026-09-26: packets SS-M1 (plain send), SS-M2 (attachments and escrow in one transaction, locking players in ascending `player_id` order; delete refused while an attachment is present) and SS-U3 (`send_system_mail` content action); plan in PR #873, `docs/analysis/social-systems/` |

### 3.2 Patch DLL: a new crate, `cimmeria-client-patches`

An i686 `cdylib` in the same shape as `cimmeria-client-telemetry`. The launcher injects it on every launch, independent of the telemetry opt-in. The telemetry crate's hooking primitives (`hooks/primitives/`) are crate-private (`pub(crate)`), so the patch DLL links MinHook directly. That is the same library the telemetry DLL uses for its inline hooks, which is what makes chaining at the shared addresses safe. Extracting a shared hooking crate is a later cleanup, worth doing if a third injected DLL ever appears.

- **Build fingerprint gate.** Before installing any hook, check the expected prologue bytes at each address. On a mismatch, install nothing and log why. The addresses are specific to this `SGW.exe` build.
- **Receive (network thread).** The dispatcher and drop-callee hooks (evidence §2) act only when the entity is the local player and the `MethodDescription` name is one of the six BM names. They decode the arguments by hand through `retrieve`; the engine's own decoder throws on the narrow `sellerName`. Decoded events go onto a bounded queue. No Lua and no game state is touched on this thread.
- **Deliver (main thread).** The `FEngineLoop::Tick` drain pops events and calls Lua functions through the C API, for example `CimmeriaBM.onAuctions(items, totalResults, clientKey)`, with tables built through `lua_createtable`/`lua_setfield`.
- **Send (main thread).** At UI start it registers native functions on a `CimmeriaBM` table: `search`, `create`, `bid`, `cancel` and `watch`. Each validates its arguments, serializes the payload, checks `isOnline`, and calls `startEntityMessage`.
- **Safety.** `catch_unwind` on every FFI edge, plus the telemetry crate's SEH guard around native calls. The DLL never throws into the game.
- **One codec for both sides.** Put the BM wire codec in a new zero-dependency crate that both the server and the DLL use. `cimmeria-wire` is too heavy to link into an injected DLL: it pulls in tokio and mercury. With a shared codec, a server/client order mismatch like S1 or S4 becomes a compile-time or unit-test failure instead of a UAT surprise.

The receive path is generic: *a shelved client method, matched by name, decoded in Rust, and forwarded to Lua*. Other campaigns that hit the same silent drop can reuse it. The telemetry DLL's drop oracle already reports which methods drop.

### 3.3 UI overlay: patched `BlackMarket.lua` and `BlackMarket.layout`

Shipped as a signed overlay patch through the launcher manifest (`crates/launcher/src/manifest.rs`), the same channel as the seed and the other patches.

- **Keep the data in Lua.** Replace the four read bindings (`getAuctionItemInfo`, `getAuctionViewItems`, `getAuctionTotalCount`, `getAuctionVisibleCount`) with Lua functions over a Lua-side store keyed by view. They return the same table shape the C++ built (evidence §4). Name and icon come from `getItemDefInfo`. Tech competency needs a one-field native getter from the DLL or is left blank; see D7.
- **Fix U1–U12.** Wire Bid and Buyout to `CimmeriaBM.bid`. Fix the nil variables, paging, row lookup, `initRows` and the My Auctions prefix. Add row imports to the My Bids and Watched tabs. Refresh My Auctions and My Bids when their tab opens.
- **Map error ids to text** in Lua, so `onBMError` gets the string it expects.
- **Degrade cleanly.** If `CimmeriaBM` is missing (the game was started without the launcher), show one line: "The Black Market needs the Cimmeria client patch". Don't let the handlers throw.
- **Visible feedback on every press.** Search, Bid, Buyout, Create and Cancel each change something on screen immediately, per the project rule. Results or errors arrive afterwards.

## 4. Why not the earlier approaches

| Approach | Why not |
|---|---|
| Launcher writes x64dbg-style caves into the process (#587 as written) | Hand-assembled `rel32` math, one cave per method. The data methods (91–95) need a wire parser written in assembly. Nothing is testable off-target. |
| Revive the CME NetOut registration | Four crashes. It depends on template instantiations and a hash-bucket layout the shelved feature never initialized, and it is unnecessary once `startEntityMessage` is used. |
| Hand-built dispatch nodes spliced into the method map | Proven live for method 90, but it builds fake C++ objects on the heap per session and still cannot decode the `AuctionItem` array (the engine decoder throws). |
| On-disk `SGW.exe` patch | Distributing a modified executable. It still needs a parser in assembly. |
| Repurpose a working message (chat or dialog) as a transport, with Lua-only patches | No native code, but it invents a private protocol on top of chat. It drifts from the `.def` contract the server already implements, and needs a new-opcode-class owner decision. Kept as a fallback if D1 is refused. |

## 5. Work packets

| ID | Packet | Depends on | Output |
|---|---|---|---|
| BM-00 | Live spike: checks V1–V6 from the evidence doc, run through the lab bridge (main-thread native calls, SEH-guarded) or non-freezing x64dbg reads, then the BM-03/BM-04 DLL checks in [5.5](#55-bm-04-outcome) | — | Updates the evidence doc; go/no-go for BM-03/04 |
| BM-01 | Port `feat/571` onto the split crates. Tests green, no behavior change | — | PR |
| BM-02 | Server contract fixes S1–S8, plus the shared codec crate | BM-01 | PR with byte-exact wire tests and live-DB search/paging guards. **Done** (PR #971): see [5.3](#53-bm-02-outcome) |
| BM-02b | S9: move sweep and buyout payouts onto the social-systems mail API | BM-01, SS-M1 + SS-M2 merged | PR with live-DB guards for sold, unsold and cancelled settlement. **Done**: see [5.4](#54-bm-02b-outcome) |
| BM-03 | Patch DLL skeleton: fingerprint gate, receive hooks, decode, main-thread delivery | BM-00, BM-02 codec | PR; off-target unit tests for the decoders |
| BM-04 | Patch DLL send natives and `CimmeriaBMNative` registration | BM-03 | PR. **Done** (statically verified only): see [5.5](#55-bm-04-outcome) |
| BM-05 | UI overlay: Lua store, read-binding replacements, U1–U12, error text | BM-03/04 surface | Overlay files + diff. **Done**: `crates/client-patches/overlay/` (patched `BlackMarket.lua` and `.layout`, `MANIFEST.txt`, and the diff summary in its README). U1–U12 are fixed, plus the status line the stock layout never defined. A Lua 5.1 logic UAT runs in CI (`overlay-lua`). Live rendering is still owed to the BM-07 UAT |
| BM-06 | Launcher: always-inject the patch DLL (with an opt-out), manifest overlay entry, docs | BM-03 | PR; closes #587. **Done** (PR #984): 64-bit launcher injecting through the embedded i686 `sgw-start32` helper, DLL embedded, `client_patches.enabled` opt-out, patches before telemetry, `"root": "sgw_game"` overlay patch packed by `pack-client-overlay`, `client.patches.boot` event. Live launch still owed |
| BM-07 | Content and UAT: the auctioneer template, spawn and chains 5030/5031, seed listings, a UAT checklist, and a `.`-console helper to seed or expire listings. The branch's ids (template 168, spawn 238) now collide with the Castle rebuild. Use the Black Market seed block allocated by the social-systems coordinator: **templates 305–309, spawns 405–409**. Put the chains in `castle_cellblock_chains.sql` with scope `'space', 12` | BM-02 | PR + checklist. **Done**: see [5.6](#56-bm-07-outcome) and [uat.md](uat.md) |
| BM-08 | Watch list (65/66/95), if D4 says yes | BM-05 | PR |

BM-01, BM-00 and BM-03 can run in parallel. BM-02b waits for the social-systems mail packets; the coordinator (cimmeria-19) will say when SS-M1 and SS-M2 merge. D8 (immediate buyout) landed in BM-02 on the sweep's settlement step; BM-02b moves that shared step onto the mail API.

### 5.1 Telemetry acceptance

Owner rule (2026-09-26): a restored system must be debuggable from telemetry alone, in SigNoz, with no repro and no debugger. Every packet follows [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [negative-logging-convention.md](../../architecture/negative-logging-convention.md) and [observability.md](../../architecture/observability.md). A packet is not done until its line below holds.

| Packet | Telemetry acceptance |
|---|---|
| BM-00 | Each live check records what it observed (addresses, bytes, return values) in the evidence doc. No product telemetry. |
| BM-01 | The port keeps every log the branch had. Each BM dispatch entrypoint (cell 61–66 decode, the base create/bid/cancel/search handlers, each sweep pass) has an info span, and its player logs carry `account_id` + `player_id`. Every new log target has a pinned `OTEL_FILTER` row. |
| BM-02 | Every state transition (listed, bid, outbid refund, cancelled, sold, expired) is a debug event `event = "bm.<transition>"`. Each carries `auction_id`, seller and bidder ids, the bid before and after, the escrowed cash before and after, and `item_type_id` with `item_name`. Every refusal logs an enumerated `reason=` that matches the `onBMError` id, with a `LogCapture` test per refusal seam. Search logs `client_key`, the filters, rows returned and `total_results`. A decode failure on 61–66 logs the payload length and the reason. Every `onBM*` send logs the method, the auction id or row count, and the payload size. A counter `bm.outcome{op, outcome}` uses enumerated labels only. A player who received `onBMOpen` but sent no 61–66 call in that session is logged once at logout as `bm.open_without_client_call`, the server-side sign that the client patch is missing. |
| BM-02b | Each settlement logs the mail id it produced, the cash and item moved, and why (sold, expired, cancelled, buyout). **Held**: INFO `bm.payout` per mail, with outbid refunds too (§5.4). |
| BM-03 | The DLL writes a local log in a format the launcher's telemetry tailer can read. It records the fingerprint result per hook address, hook install success, each BM event decoded (method, size, outcome), decode errors with a reason, the Lua delivery outcome, and a count of events dropped because the overlay was missing. |
| BM-04 | Each native send logs the method, sub-index and payload size, and every refusal (offline, bad arguments) with a reason. The server-side receive logs from BM-02 complete the round trip. |
| BM-05 | Every overlay handler is `pcall`-guarded, and errors reach the client log that the launcher tails, tagged `[Cimmeria BM]`. |
| BM-06 | With telemetry opted in, the launcher tails the patch DLL's log and records the DLL version and fingerprint result once per session. |
| BM-07 | The UAT checklist names the SigNoz query for each step, backed by a saved "Black Market" view, so a failed step can be diagnosed from telemetry. |
| BM-08 | Watch and unwatch transitions and each watch notification sent are logged with the same fields as BM-02. |

### 5.2 Review follow-ups deferred from BM-01

BM-01 (#965) ported the branch without changing its behaviour. The review of #965 raised these points; each belongs to the packet named here.

| Finding | Packet | Note |
|---|---|---|
| `auction_length` is stored and echoed as the raw client byte, but `auction_length_seconds` maps every value above 3 to the 96 h tier | BM-02 | **Done.** Clamped to 1–5 and stored clamped (S5); `endTimeValue` is the time-left bucket (S6) |
| A bid or cancel in the window between `expires_at` and the next sweep pass changes or voids a closed auction | BM-02 | **Done.** Both refused `AuctionGone` when `expires_at <= now` |
| Search sizes its reply by row count, not serialized size, so a large result can exceed the packet limit | BM-02 | **Done.** At most 50 rows read, the reply cut to a 1,200-byte argument budget; `totalResults` stays the full count (S7) |
| Cell methods 62-64 are forwarded without proof that the player is at an auctioneer (CWE-862) | BM-02 | **Done.** `black_market_access` (cell-world): a session recorded by `open_black_market`, the interaction target still that auctioneer, in range; refusal `NotAtAuctioneer`. Reviewed by the server-authority-enforcer |
| `ON DELETE RESTRICT` on `sgw_auction.seller_id` / `current_bidder` blocks deleting a character forever once it has listed or won, because settled rows are never removed | BM-02 | **Done.** Decision D-BM09 |
| One failing auction aborts the whole sweep pass (no `ORDER BY`, `?` on the first error); `return_item` uses `fetch_one` and fails on a missing `item_def_id` | BM-02b | **Done.** The due set is ordered (`expires_at, sequence_id`), each auction settles in its own transaction, and a failure that will recur is quarantined (status 4, `bm.quarantined`); `return_item` is gone |
| Settlement mail is inserted directly into `sgw_gate_mail`: no `expires_at` and no `sgw_gate_mail_item` escrow row | BM-02b | **Done.** S9: every payout goes through `send_system_mail_tx`; `send_mail_to_player` is deleted |
| The boot seed's reserved system seller (account 1, player 1) is inserted without checking what those ids hold | BM-07 | **Done.** `ensure_system_seller` reads both rows back and the seed refuses (`bm.seed_refused`) unless they are the system seller; seed listings are keyed on `item_id = 0` alone |
| `open_black_market` pins whatever NPC its chain names; nothing checks the NPC is an auctioneer (no auctioneer interaction type exists), so a chain bound to another NPC would make it a Black Market terminal | BM-07 | **Done.** `NpcInteractionType::Auctioneer`, derived at spawn from the template's `INT_Auction`; `auctioneer_check` on the open and on 62-64. Reviewed by the server-authority-enforcer |
| A settlement whose escrow row is missing, or that fails, is skipped and retried by every sweep pass (`bm.escrow_missing` each time); there is no quarantine | BM-02b | **Done.** Quarantined once (`reason = escrow_missing`); a database error is still retried each pass (`bm.settle_retry`) |
| Quarantined auctions have no GM tool: an operator must mail the container-18 row and any held bid by hand | Follow-up | Found in BM-02b. A `.bm` console command (resolve, or retry a quarantined auction) fits BM-07's `.`-console helper |
| The D-BM09 delete trigger refunds standing bidders by a direct balance credit, not by mail | Follow-up | Found in BM-02b. A trigger cannot call the mail writer; move the refund into the character-delete path if it ever becomes Rust |
| Bind-on-acquire items are listable, as they are tradeable and mailable: grants never set `bound` | Systemic | Found in the BM-02 authority review; not Black Market specific |

### 5.3 BM-02 outcome

- **S1–S8** as §3.1, through the shared codec: the server decodes 61–66 and encodes 90–95 with `cimmeria-patch-wire` (re-exported as `cimmeria_wire::black_market`), so the server and the client patch cannot disagree on a layout. Byte-exact tests in `base-methods`' `methods/black_market/wire/tests.rs` and the cell's `black_market/tests.rs`.
- **D3** `BMError` in `cimmeria-patch-wire`: 0 and 1 shipped, 2–14 the server's refusals, each with a logged `reason`. **D4** watch calls answer `WatchUnavailable`. **D5** 20 active listings. **D6** +5%, at least +1. **D8** immediate buyout. **D7** is the DLL's (no server change).
- **Escrow.** A listing moves its row into the seller's container 18 instead of deleting it; cancel and expiry move it back, a sale moves it to the buyer. Container 18 is excluded from every client-bound inventory read. Starting price at least 1, bound items refused, only bags 1 and 15 listable.
- **Authority, expired window, paging, FK**: the §5.2 rows marked done.
- **Telemetry**: the §5.1 BM-02 line, with `bm_outcome_total{op, outcome}` as the counter's exported name.
- **Deferred**: the new §5.2 rows (BM-07, BM-02b, systemic).

### 5.4 BM-02b outcome

- **One mail writer (S9, D-BM10).** `payout_mail.rs` builds every Black Market mail and hands it to the mail module's `send_system_mail_tx` inside the caller's transaction. Sold (sweep or buyout): the container-18 row to the buyer as `ExistingInstance`, the winning bid minted to the seller. Expired unsold and cancelled: the row back to the seller. Outbid and a cancelled auction's standing bid: cash to the bidder. `send_mail_to_player`, `deliver_from_escrow` and every direct `sgw_gate_mail` insert are deleted. A boot-seed sale mails the buyer a minted instance and pays nobody.
- **The module moved.** The mail writer lives in `cimmeria-base-methods`, which depends on `cimmeria-base-session`, so `base/black_market/` moved from base-session to `crates/base-methods/src/base/world_entry/methods/black_market/` (a pure move, its own commit).
- **Exactly once.** Each settlement's first write is its conditional status change (`WHERE status = ACTIVE RETURNING`); a second settlement from a stale read is `SettleError::Gone` and writes nothing. Cancel does the same. Guarded live (`a_second_settlement_pays_nothing`).
- **Poison rows.** Ordered due set, one transaction per auction, `QUARANTINED` (status 4) for a failure that will recur, retry for a database error, and the pass goes on. A missing escrow row is `bm.escrow_missing` and is never minted.
- **Lock order** after the auction row: the seller's escrow advisory locks, the escrowed item row, every `sgw_player` row in ascending id (mail recipients included), then the writer.
- **Telemetry**: the §5.1 BM-02b line; also `bm.quarantined`, `bm.settle_retry`, `bm.refund_skipped`, and `bm_outcome_total{op="payout"|"settle"}`.
- **Tests**: live-DB guards for sold, unsold, phantom bidder, cancelled, outbid, buyout, a buyout into full bags, the double settlement and the poison rows, each asserting the mail, the `sgw_gate_mail_item` row and the cash.
- **Deferred**: the two new §5.2 follow-up rows.

### 5.5 BM-04 outcome

- **Contract** as agreed with BM-05: a global `CimmeriaBMNative` owned by the DLL, with `search(opts)`, `create(itemInstanceId, startingPrice, buyoutPrice, auctionLength)`, `bid`, `cancel`, `watch(itemDefId, enable)`, `techCompetency` and `version`. Each send returns `true` or `nil, reason` (`bad_args`, `offline`, `not_main_thread`, `engine_error`). Rules and defaults: [crates/client-patches/README.md](../../../crates/client-patches/README.md#the-send-contract-for-the-ui-overlay). Two choices the contract left open: `create`'s `buyoutPrice` may be `nil` (no buyout), and out-of-enum values are refused as `bad_args` (`clientKey` outside 0–2, `auctionLength` outside 1–5).
- **Engine path**: `startEntityMessage(conn, 0x3D, 0)`, `reserve(1)` for the sub-index, `reserve(n)` for the payload, after checking `[[0x01ef244c]+8]` is non-null, `[conn+0x30c] != 0` and a local player id. `startEntityMessage` logs and carries on when offline, so the check is the DLL's. Engine calls run under `microseh`; the i686 tests raise an access violation and an MSVC C++ exception code through it.
- **Fingerprint gate** extended with four pinning sites (`startAvatarMessage`, `isOnline`, the `GameEntityManager` getter, the byte-writer that calls `reserve`). Any mismatch installs nothing.
- **D7**: `techCompetency` returns `nil`. The static read of the lookup is in the evidence doc §4; V6 is the live check.
- **Telemetry**: the §5.1 BM-04 line, in the DLL's local log.
- **Verified**: host and i686 unit tests (argument rules, byte-exact payloads against `cimmeria-patch-wire`, reason mapping, registration idempotency, the engine path against a fake connection and bundle). **Not verified live**: nothing has run in `SGW.exe`.

Live checks owed to BM-00 for the send side, in order:

1. The DLL's log shows every fingerprint site `stock` (or `chaining` for Tick and the drop callee) and `registered CimmeriaBMNative`.
2. The lab bridge's Lua relay (or the overlay) sees `CimmeriaBMNative.version` equal to the DLL version, and again after a UI reload.
3. `CimmeriaBMNative.cancel(1)` returns `true`; the server logs a cell 64 decode for the player (V4). The server then refuses it, which is expected without a listing.
4. `search({})`, `search({clientKey = 1})`, `bid(seq, amount)`, `create(item, start, nil, 5)` and `watch(id, true)` each reach the server and decode with no trailing bytes. The server's refusals come back through `CimmeriaBM.onError`.
5. At character select, `cancel(1)` returns `nil, "offline"` and the server logs nothing.
6. `create(1, 2, 3, 0)` returns `nil, "bad_args"` and the log names `auctionLength`.
7. V6: the tech-competency lookup chain in the evidence doc returns the value `getAuctionItemInfo` shows, without a crash, for a cached and an uncached item.

### 5.6 BM-07 outcome

- **Auctioneer.** Machra (template 305, spawn 405, tag `BlackMarket_Auctioneer`) in the stasis-room debug hub, on the C-D exit wall. Chain 5030 (`interact_tag`, scope space 12) runs `open_black_market`. The template's seeded `INT_Auction` bit is the cursor and the authority marker, so no `player_loaded` chain sets it. Chain 5031 and ids 306-309, 406-409 stay reserved: no in-world auctioneer is seeded, because the client carries no NPC placements and the reconstructed spawn list has none.
- **Authority (§5.2).** `NpcInteractionType::Auctioneer` is derived only at spawn, from the template. `auctioneer_check` (cell-world) runs before `onBMOpen` and inside `black_market_access`. Every click answers with a chat line; a refusal logs `bm.open_refused`.
- **System seller (§5.2).** `ensure_system_seller` checks account 1 before inserting player 1, then both, and refuses with `bm.seed_refused reason=…`. The account is created disabled, and an older enabled one is switched off. `is_seed_listing` is `item_id = 0` alone (authority review).
- **GM tools.** `.bm_seed [count]` (the UAT set: a pistol at five tech tiers, slappacks, two SMGs, every duration tier), `.bm_list`, `.bm_expire <auctionId>` (settles at once through the sweep). Carried by `BlackMarketCellToBase::GmSeed`, `GmExpire` and `GmList`, handled in `base-methods` `methods/black_market/gm/`. `.bm_expire` settles through the same `settle_expired_once` pass as the sweep, so its payouts are BM-02b's system mail.
- **UAT.** [uat.md](uat.md): steps U0-U23, each with its expected result, its SigNoz query and whether it needs the client patch. The saved view is [black-market.view.json](../../operations/signoz/black-market.view.json). The unified UAT guide's Black Market section follows it.
- **Deferred.** The auctioneer's role is not restored if he dies (death writes `Loot`; the respawn tick restores only the flags), as for every Banker and Vendor; he is faction 1 and cannot be killed by a player. A GM tool for quarantined auctions (§5.2 follow-up) is not in BM-07.

### 5.7 First live run (2026-09-29): presentation fixes

A colo player's first run with the patch found three defects. None changes the wire contract.

| Defect | Cause | Fix | Guard |
|---|---|---|---|
| A bid-only listing (the seeded Health Slappacks: start 30, buyout 0) read as "no price, can't buy" | The rows showed `currentBid`, 0 before any bid, and a blank Buyout; Buyout was disabled, so pressing it did nothing | The Bid column shows `Min <nextMinBidPrice>` until someone bids and the Buyout column says `Bid only`, in Search Results, My Auctions and My Bids. Buyout stays enabled on a bid-only row and the press says "This auction is bid only. Enter at least 30 and press Bid." | Overlay logic UAT, scenario "bid-only listing" |
| The Create tab's duration labels read "SHORMEDIUM ONG" | Stock layout: labels 32, 60 and 24 px wide, Medium's box over Short's, so CEGUI clipped them | `Short`, `Medium`, `Long`, 52 px each on their own slots, bars and highlights re-centred | Overlay logic UAT, scenario "Create tab: Short/Medium/Long labels" (geometry from the layout) |
| The "Auction Won" mail showed Sent "Thu Jan 1st, 1970 @ 2:2 am" and Expires "Soon" | Not the payout: every mail's header sent `sentTime` as epoch seconds. The client reads it as the mail's age: Sent is local now minus `sentTime`, and ExpiresHours is `720 - sentTime / 3600` (`SGW.exe@0x00eb5ab0`, `0x00eb5a10`) | `mail/headers.rs` sends `now - sent_time` (at least 0). The database keeps epoch seconds | `headers::tests::header_sent_time_*` (unit) and `mail::tests::header_time_live` (live-DB, a Black Market payout read back through `read_one`) |

The overlay change ships as a new `bm-ui-overlay` zip from the next launcher release, with its entry swapped in the signed manifest.

Found on the way, not fixed here: the same constructor sets `HasBeenRead` when `readTime >= 0.0`, and the server sends `read_time` 0 for an unread mail, so no mail ever shows the inbox's "New" marker. See [mail-wire-formats.md](../../reverse-engineering/findings/mail-wire-formats.md#m-q3--expireshours-source-and-ttl-closed).

## 6. Decisions

All eight were answered in session on 2026-09-26, each as recommended.

| ID | Question | Decision |
|---|---|---|
| D1 | This is a client patch (native code plus UI files). Is it acceptable in this form, widening #587 from "open the window" to the full feature? | **Yes.** The feature cannot work without it, and this shape is testable and removable. |
| D2 | A new always-injected `cimmeria-client-patches` DLL, or fold the patch into the telemetry DLL? | **New DLL.** Players who opted out of telemetry never load the telemetry DLL, and gameplay does not depend on a telemetry toggle. Cost: hook chaining at two shared addresses (Tick and the drop callee), handled with MinHook in both DLLs. |
| D3 | Error vocabulary: the shipped enum has only `InvalidSortType`/`BMUnavailable`. Keep the server's rejection ids and map them to English text in the overlay? | **Yes.** Ids 0 and 1 stay the shipped values, and the server's rejection ids follow them. |
| D4 | Watch list: build it, or refuse with visible feedback? | **Defer.** Refuse with a one-line message until the core loop has passed UAT. |
| D5 | Listing fee and per-player listing cap (CAT-I-02)? No source shows the original had either. | **A cap of 20 active listings per player, no fee.** |
| D6 | Next-minimum-bid rule. The client only displays the server's `nextMinBidPrice`, so this is design, not recovery. | **5% of the current bid, at least +1.** |
| D7 | Tech-competency column: add a native getter in the DLL, or leave it blank? | **Native getter.** One read, with the item-cache refcount handled. **BM-04 outcome:** not safe to ship unverified; `techCompetency` returns `nil` until live check V6 confirms the lookup chain ([5.5](#55-bm-04-outcome)). |
| D8 | Buyout: settle immediately or at expiry? | **Immediately.** Landed in BM-02 through the sweep's settlement step (`settle.rs`); BM-02b moves that step onto the mail API. |
| D-BM10 | Cancel and expiry: return the item straight to the seller's bags, or by mail like a sale? | **By mail.** Every item and coin an auction moves is system mail from "Black Market" through `send_system_mail_tx`: a full bag no longer refuses a cancel or pushes an expired item past the main bag's last slot, an offline seller loses nothing, and there is one writer for every payout. Outbid refunds and a cancelled auction's standing bid are mail too; only D-BM09's trigger still credits directly. Decided in BM-02b. |
| D-BM09 | Character deletion: `sgw_auction`'s `RESTRICT` foreign keys block deleting any character that ever listed or won. What happens to its auctions? | **The listings go with the seller, and nobody else loses cash.** `seller_id` is `ON DELETE CASCADE` (the escrowed item is a container-18 row of the seller and cascades with the inventory), `current_bidder` is `ON DELETE SET NULL` (settled rows keep their history). A `BEFORE DELETE` trigger on `sgw_player`, `bm_player_before_delete()`, refunds the standing bidders of the seller's open auctions and reopens the open auctions the deleted character was winning. Decided in BM-02. |

## 7. Risks

- **Build specificity.** Every address is for this QA `SGW.exe`. The fingerprint gate turns a different build into "Black Market unavailable" instead of a crash.
- **Two DLLs hooking the same function.** Both must chain safely. BM-03 tests this with the telemetry DLL injected.
- **Item definitions not cached on the client.** Rows for never-seen items may lack a name until the cooked-data element arrives. V5 measures this; re-render on arrival if needed.
- **Crash history.** Every earlier crash came from building engine objects by hand or calling engine code on the wrong thread. This design does neither: no fake C++ objects, all engine calls on the main thread, and only byte reads on the network thread.

## 8. Testing

Per [TESTING.md](../../../TESTING.md): byte-exact wire tests on the shared codec for every method in both directions; live-DB guards for search filtering, paging and `clientKey` routing (S2/S3/S7); a regression guard per contract fix that fails if the old order is restored; and an in-game UAT checklist in BM-07 covering open → search → page → bid → outbid → buyout → create → cancel → expiry, with two clients for the outbid case.
