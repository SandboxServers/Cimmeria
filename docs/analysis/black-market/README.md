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

An i686 `cdylib` in the same shape as `cimmeria-client-telemetry`. The launcher injects it on every launch, independent of the telemetry opt-in. It reuses the telemetry crate's audited hooking primitives, which were written with "the future client-patch crates" in mind (`hooks/primitives/mod.rs`).

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
| BM-00 | Live spike: checks V1–V5 from the evidence doc, run through the lab bridge (main-thread native calls, SEH-guarded) or non-freezing x64dbg reads | — | Updates the evidence doc; go/no-go for BM-03/04 |
| BM-01 | Port `feat/571` onto the split crates. Tests green, no behavior change | — | PR |
| BM-02 | Server contract fixes S1–S8, plus the shared codec crate | BM-01 | PR with byte-exact wire tests and live-DB search/paging guards |
| BM-02b | S9: move sweep and buyout payouts onto the social-systems mail API | BM-01, SS-M1 + SS-M2 merged | PR with live-DB guards for sold, unsold and cancelled settlement |
| BM-03 | Patch DLL skeleton: fingerprint gate, receive hooks, decode, main-thread delivery | BM-00, BM-02 codec | PR; off-target unit tests for the decoders |
| BM-04 | Patch DLL send natives and `CimmeriaBM` registration | BM-03 | PR |
| BM-05 | UI overlay: Lua store, read-binding replacements, U1–U12, error text | BM-03/04 surface | Overlay files + diff |
| BM-06 | Launcher: always-inject the patch DLL (with an opt-out), manifest overlay entry, docs | BM-03 | PR; closes #587 |
| BM-07 | Content and UAT: the auctioneer template, spawn and chains 5030/5031, seed listings, a UAT checklist, and a `.`-console helper to seed or expire listings. The branch's ids (template 168, spawn 238) now collide with the Castle rebuild. Use the Black Market seed block allocated by the social-systems coordinator: **templates 305–309, spawns 405–409**. Put the chains in `castle_cellblock_chains.sql` with scope `'space', 12` | BM-02 | PR + checklist |
| BM-08 | Watch list (65/66/95), if D4 says yes | BM-05 | PR |

BM-01, BM-00 and BM-03 can run in parallel. BM-02b waits for the social-systems mail packets; the coordinator (cimmeria-19) will say when SS-M1 and SS-M2 merge. D8 (immediate buyout) shares the same payout path, so it lands with BM-02b.

### 5.1 Telemetry acceptance

Owner rule (2026-09-26): a restored system must be debuggable from telemetry alone, in SigNoz, with no repro and no debugger. Every packet follows [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [negative-logging-convention.md](../../architecture/negative-logging-convention.md) and [observability.md](../../architecture/observability.md). A packet is not done until its line below holds.

| Packet | Telemetry acceptance |
|---|---|
| BM-00 | Each live check records what it observed (addresses, bytes, return values) in the evidence doc. No product telemetry. |
| BM-01 | The port keeps every log the branch had. Each BM dispatch entrypoint (cell 61–66 decode, the base create/bid/cancel/search handlers, each sweep pass) has an info span, and its player logs carry `account_id` + `player_id`. Every new log target has a pinned `OTEL_FILTER` row. |
| BM-02 | Every state transition (listed, bid, outbid refund, cancelled, sold, expired) is a debug event `event = "bm.<transition>"`. Each carries `auction_id`, seller and bidder ids, the bid before and after, the escrowed cash before and after, and `item_def_id`. Every refusal logs an enumerated `reason=` that matches the `onBMError` id, with a `LogCapture` test per refusal seam. Search logs `client_key`, the filters, rows returned and `total_results`. A decode failure on 61–66 logs the payload length and the reason. Every `onBM*` send logs the method, the auction id or row count, and the payload size. A counter `bm.outcome{op, outcome}` uses enumerated labels only. A player who received `onBMOpen` but sent no 61–66 call in that session is logged once at logout as `bm.open_without_client_call`, the server-side sign that the client patch is missing. |
| BM-02b | Each settlement logs the mail id it produced, the cash and item moved, and why (sold, expired, cancelled, buyout). |
| BM-03 | The DLL writes a local log in a format the launcher's telemetry tailer can read. It records the fingerprint result per hook address, hook install success, each BM event decoded (method, size, outcome), decode errors with a reason, the Lua delivery outcome, and a count of events dropped because the overlay was missing. |
| BM-04 | Each native send logs the method, sub-index and payload size, and every refusal (offline, bad arguments) with a reason. The server-side receive logs from BM-02 complete the round trip. |
| BM-05 | Every overlay handler is `pcall`-guarded, and errors reach the client log that the launcher tails, tagged `[Cimmeria BM]`. |
| BM-06 | With telemetry opted in, the launcher tails the patch DLL's log and records the DLL version and fingerprint result once per session. |
| BM-07 | The UAT checklist names the SigNoz query for each step, backed by a saved "Black Market" view, so a failed step can be diagnosed from telemetry. |
| BM-08 | Watch and unwatch transitions and each watch notification sent are logged with the same fields as BM-02. |

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
| D7 | Tech-competency column: add a native getter in the DLL, or leave it blank? | **Native getter.** One read, with the item-cache refcount handled. |
| D8 | Buyout: settle immediately or at expiry? | **Immediately.** Lands with BM-02b, because it shares the payout path with the sweep. |

## 7. Risks

- **Build specificity.** Every address is for this QA `SGW.exe`. The fingerprint gate turns a different build into "Black Market unavailable" instead of a crash.
- **Two DLLs hooking the same function.** Both must chain safely. BM-03 tests this with the telemetry DLL injected.
- **Item definitions not cached on the client.** Rows for never-seen items may lack a name until the cooked-data element arrives. V5 measures this; re-render on arrival if needed.
- **Crash history.** Every earlier crash came from building engine objects by hand or calling engine code on the wrong thread. This design does neither: no fake C++ objects, all engine calls on the main thread, and only byte reads on the network thread.

## 8. Testing

Per [TESTING.md](../../../TESTING.md): byte-exact wire tests on the shared codec for every method in both directions; live-DB guards for search filtering, paging and `clientKey` routing (S2/S3/S7); a regression guard per contract fix that fails if the old order is restored; and an in-game UAT checklist in BM-07 covering open → search → page → bid → outbid → buyout → create → cancel → expiry, with two clients for the outbid case.
