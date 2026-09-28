# Client UI overlay

Patched copies of stock client UI files. The launcher installs them over the
client's own files, next to the `cimmeria-client-patches` DLL (packet BM-06).
The tree below `overlay/` mirrors the client install below
`Working/SGWGame/`. For example, `overlay/Content/UI/Core/BlackMarket/BlackMarket.lua`
replaces `Working/SGWGame/Content/UI/Core/BlackMarket/BlackMarket.lua`.

[MANIFEST.txt](MANIFEST.txt) lists every overlay file by its client-relative
path, one per line. The packager reads it, so a file that is not listed there
is not shipped. `test/` is the logic UAT and is not shipped.

## Base files

| Overlay file | Stock original |
|---|---|
| `Content/UI/Core/BlackMarket/BlackMarket.lua` | QA client build, `Working/SGWGame/Content/UI/Core/BlackMarket/BlackMarket.lua`, 2009-06-30, 33,682 bytes, SHA-256 `66e159de2e90e441e7be7f80542614aad67b89d5aeb00d1169aadf68b617a2da` |
| `Content/UI/Core/BlackMarket/BlackMarket.layout` | the same folder, `BlackMarket.layout`, 2009-06-30, 79,891 bytes, SHA-256 `8fe7ad58cd9869837327360a045ace74d03ade00deb5fcdec6b38c8878885173` |

The module's other files are unchanged and are not in the overlay:
`BlackMarket.toc` (it loads the `.lua` and the `.layout`), the two row
layouts `BlackMarket_ItemRow.layout` and `BlackMarket_YourAuctionRow.layout`,
and the `.int` string files.

## Diff approach

The PR that added the overlay committed the stock files unmodified first,
then the patch as a second commit, so `git log -p` on these files shows the
whole patch against the originals. The squash merge keeps only the result, so
the summary below is the record.

- **Encoding.** The stock files are ASCII with CRLF line endings. The overlay
  keeps both, with no non-ASCII byte, and `.gitattributes` marks
  `crates/client-patches/overlay/Content/**` as `-text` so no checkout
  converts them.
- **Naming.** Stock names are kept wherever the stock code was right, so the
  diff stays readable. New code sits beside the function it replaces.
- **One file.** `BlackMarket.lua` is one file of about 1,560 lines because the
  module's `.toc` loads one script. It is not split.

## What changed in `BlackMarket.lua`

The defect ids are the U-list in
[black-market-client-io.md](../../../docs/reverse-engineering/findings/black-market-client-io.md) §5.
The plan is [docs/analysis/black-market/README.md](../../../docs/analysis/black-market/README.md) §3.3.

| Area | Change |
|---|---|
| Receive | A global `CimmeriaBM` table with plain function fields that the DLL calls without `self`: `onOpen(entityId)`, `onError(errorId)`, `onAuctions(items, totalResults, clientKey)`, `onAuctionRemove(sequenceId)`, `onAuctionUpdate(item)` and `onWatchedItems(itemList)`. `onOpen` opens the window through `BlackMarketMod.onBMOpen()`, the same path as `Events.BMOpen`, which stays subscribed. |
| Store | The four C++ read bindings the stock code called (`getAuctionItemInfo`, `getAuctionViewItems`, `getAuctionTotalCount`, `getAuctionVisibleCount`) are now `BlackMarketMod.*` functions over a Lua store, keyed by view (`clientKey` 0/1/2, plus 3 for Watched). `getAuctionItemInfo` returns the table shape the C++ built (evidence §4). Name and icon come from `getItemDefInfo`. Tech competency comes from `CimmeriaBMNative.techCompetency` and is blank when that returns nil (D7). Bidder name and bid count are not on the wire, so they are blank. The native globals are not overwritten. |
| Send | Every request goes through `BlackMarketMod.send`, which calls `CimmeriaBMNative.search/create/bid/cancel` under `pcall`. `nil, reason` or a thrown error shows a line saying why (offline, bad arguments, and so on), and the button works again. `search` takes the `.def`-named options table, with `bForward` as a boolean. |
| U1 | Bid and Buyout are subscribed. Bid sends `bid(seq, amount)` from the bid box and refuses locally, with a message, below `nextMinBidPrice`. Buyout sends `bid(seq, buyoutPrice)` and is enabled only when the auction has a buyout price. |
| U2, U3, U4 | Paging. The server returns one page per search, cut to fit one message, together with the full count. The overlay keeps the page's offset within the whole result: Next is enabled while `offset + rows < total`, and Prev while `offset > 0`. The cursor is the highest id shown (forward) or the lowest (backward). The page text is 1-based (`1/3`, or `0/0` when there are no results). The undefined `viewType` and `currentPage` are gone. |
| U5 | `selectRow` reads `viewData[view].auctionToRow`, and ignores a click on a row with no auction. |
| U6 | `initRows` uses its `viewType` argument, so rows in Search Results and My Auctions get click handlers. |
| U7 | My Auctions uses the row prefix `MyAuction`, the one the layout imports. |
| U8 | Row handling for My Bids (`MyBids<n>`) and Watched (`Watched<n>`), with the rows added in the layout. |
| U9 | Opening the My Auctions or My Bids tab sends a search with `clientKey` 1 or 2, and shows "Loading your auctions..." (or bids) until the reply arrives. |
| U10 | The row timer image switches on `timeLeft` (`endTimeValue`, 1–5), not tech competency. |
| U11 | `populateRow` returns early when a row window is missing, and clears a stale quantity on reused rows. |
| U12 | A create is acknowledged by the `onAuctionUpdate` for the new listing: the form clears, the listing is added to My Auctions, and the status line reads "Auction created.". |
| Input | `CimmeriaBMNative` refuses anything but whole INT32 numbers and strings over 255 UTF-8 bytes. So form text goes through `BlackMarketMod.toInt`, which floors it and gives nil when it is out of range, and the item-name filter is cut to 85 characters (at most 255 bytes, even when every character takes three). A buyout that is negative or not a number marks the field red. An empty buyout is sent as 0. |
| Errors | `onError` maps every `BMError` id (0–14, `crates/patch-wire/src/black_market/error.rs`) to a line of English and shows it in red. Unknown ids get a generic line with the number. |
| Feedback | Search, Next, Prev, Bid, Buyout, Create, Cancel and the tab buttons all change the status line on the first press ("Searching...", "Bid sent...", "Creating auction...", "Cancelling auction...", "Loading your bids..."), or explain why nothing was sent. The button in flight is disabled until the reply arrives. A request with no reply within 15 seconds times out with "The Black Market did not answer. Try again." |
| Degrade | If `CimmeriaBMNative` is missing (the game was started without the launcher), the window still opens and shows "The Black Market needs the Cimmeria client patch", and so does any button that would have sent a request. |
| Watch list | Deferred (D4). The Watched tab says "The watch list is not available yet." and sends nothing. `onWatchedItems` renders item definitions in the Watched rows, for BM-08. |
| Open | Each open resets the store, shows Search Results and runs an opening search, because the DLL keeps no state. Inside the stock 4-second search delay it says "Press Search to list auctions." instead. |
| Logging | Every UI and `CimmeriaBM` entry point is wrapped in a `pcall` guard. A failure logs `[Cimmeria BM] <handler> failed: <error>` through `Debug:log`, the client's own Lua logger (falling back to `print`). Nothing is written to the DLL's log. Load, refused sends, error ids and timeouts are logged with the same tag. |

## What changed in `BlackMarket.layout`

| Change | Why |
|---|---|
| `BlackMarket_TabButton4` (Watched Items) is no longer commented out | The Watched tab needs a button to show its refusal line |
| Eight `LayoutImport`s of `BlackMarket_ItemRow.layout` in the My Bids tab (`BlackMarket_MyBids1`–`8`) | U8: the stock tab had headers and a scrollbar but no rows |
| Eight more in the Watched tab (`BlackMarket_Watched1`–`8`) | U8 |
| A `BlackMarket_ErrorText` static text in the tab container, below the tab pages | The stock Lua writes to it on every tab switch and error, but the stock layout never defined it, so each of those writes raised an error. It is now the status and error line. |

## Logic UAT

`test/run.lua` is a REPL-style logic UAT in stock Lua 5.1. It builds stub
CEGUI windows from this `BlackMarket.layout` (every named window and every
imported row, and nothing else), stubs the tolua globals and
`CimmeriaBMNative`, loads the overlay `BlackMarket.lua`, and drives these
flows: open, search, results, scrolling, paging both ways, row selection,
bid, buyout, errors, create, cancel, the My Bids and Watched tabs, a missing
DLL, refused and throwing sends, reply timeouts, guarded handlers, and
reopening. It also checks that every `BMError` id in `error.rs` has text.
Run against the stock files, every scenario fails.

```sh
lua5.1 crates/client-patches/overlay/test/run.lua          # Linux; CI runs this (test.yml, "overlay-lua")

python -m pip install lupa                                 # Windows: lupa bundles Lua 5.1
python crates/client-patches/overlay/test/run_lupa.py
```

### Not covered by the logic UAT

These need the live client (BM-00 and the BM-07 UAT):

- How CEGUI really renders the layout: the new status line's position and
  wrapping, the fourth tab button, and the imported My Bids and Watched rows.
- The handler argument order for row clicks. The overlay reads the clicked
  row from either argument, so both orders work.
- Whether `Debug:log` reaches the client log the launcher tails.
- Whether `BlackMarketWin:isVisible()` exists in this binding. The overlay
  calls it under `pcall`.
- `getItemDefInfo` for items the player has never held (V5). Rows fall back
  to "Item <id>".
- Everything the DLL does: the real `CimmeriaBMNative` sends and the
  delivery of `CimmeriaBM.*` calls.
