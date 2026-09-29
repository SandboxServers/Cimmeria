# Black Market UAT Checklist

> **Type**: how-to (UAT checklist). **Audience**: the owner and testers with the Black Market client patch.
> **Written**: 2026-09-27 by packet BM-07 of the [Black Market plan](README.md). This file is the canonical checklist; the [unified UAT guide](../../guides/unified-uat.md#black-market) follows it.

Every step names what you should see and one SigNoz query that shows what the server did. If a step fails, the query tells you where the request stopped. You do not need a debugger or the server log.

## Before you start

**What each step needs.** The `Needs` column says what must be in place:

| Needs | Meaning |
|---|---|
| `server` | A stock client is enough. The server answers with a chat line or a mail. |
| `patch` | The client patch: the patch DLL (BM-03, BM-04), the Lua and layout overlay (BM-05), and the launcher that installs both (BM-06). Without it, the Black Market window never opens and no search, bid or listing reaches the server. |
| `patch + BM-00` | The patch, plus a result of the BM-00 live spike. Check V5 in the [evidence doc](black-market-client-io.md): a row whose item the client has never cached may show no name until the item's data arrives. |

Until the patch ships, only the `server` steps can run. The rest are listed so you can plan.

**Where.** The auctioneer is **Machra** in the Castle_CellBlock stasis room, the room every new character starts in. He stands by the exit wall, beside the doorway, facing the middle of the room ([debug-hub.md](../../content/debug-hub.md#black-market-auctioneer-template-305)). Clicking him fires chain 5030, which runs `open_black_market`.

**Characters.** You need a GM character (access level 2 or higher) for the `.bm_*` commands. Steps U8 and U9 need a second character online at the same time, on a second client (the outbid refund). Give each character cash with `.givecash <amount>` on the target.

**GM tools** (typed in chat, GM only; a player gets "is a GM command"):

| Command | What it does |
|---|---|
| `.bm_seed [count]` | Lists `count` auctions (default 8, at most 60) from the system seller "Black Market", cycling through the test set below, and names their ids |
| `.bm_list` | Shows the 10 newest active auctions with their ids, seller, price and time left. The client window never shows auction ids |
| `.bm_expire <auctionId>` | Makes one active auction due now and settles it at once, as the expiry sweep would. Says how it went: sold (to whom, for how much), returned to its seller, or a system listing that simply ends |

`.bm_seed`'s test set is the SI 3 9mm Pistol at tech competency 1, 5, 10, 15 and 20, a stack of 5 Health Slappacks, and the SGHC 6 SMG at tech competency 1 and 5. It covers every duration tier, bid-only and buyout listings, and a pistol with a buyout of 40, cheap enough for a new character. The boot seed lists three more when the house is empty at startup.

## The SigNoz view

Every query below starts with `service.name = 'cimmeria-server' AND`, left out of the table for width. In SigNoz the Rust log target is the `scope_name` column.

Save this as the Logs Explorer view **Black Market**, in the category `black-market`. The definition is committed at [docs/operations/signoz/black-market.view.json](../../operations/signoz/black-market.view.json) (the `signoz_create_view` shape used for the [NPC AI views](../../operations/signoz/npc-ai-views.md#recreating-a-view)):

```text
service.name = 'cimmeria-server' AND (event LIKE 'bm.%' OR scope_name LIKE '%black_market%')
```

Columns: `event, reason, role, mail_id, access, op, command, action, outcome, account_id, player_id, auction_id, seller_id, bidder_id, bid_before, bid_after, escrow_cash_before, escrow_cash_after, item_def_id, client_key, cursor, rows_returned, total_results, method, body`.

To narrow it to one step, add the step's clause below. To narrow it to one tester, add `AND player_id = <P>`.

The counter `bm_outcome_total{op, outcome}` (Metrics, `increase`, grouped by `op` and `outcome`) gives the totals: `op` is `open`, `search`, `create`, `bid`, `cancel`, `watch`, `settle` or `seed`, and `outcome` is `ok` or a refusal reason. BM-02b adds `op="payout"`, one per mail a settlement writes.

## Steps

Run them in order; later steps use the auctions the earlier ones create.

| Step | Do | You should see | SigNoz (after the service clause) | Needs |
|---|---|---|---|---|
| U0 | As the GM, type `.bm_seed` | "Listed 8 Black Market auction(s) from the system seller: ids A to B." | `event = 'bm.gm_action' AND action = 'bm_seed'` | `server` |
| U1 | Type `.bm_list` | Up to 10 lines `#id item from Black Market: starts N, buyout M, Hh MMm left`, then the total | `event = 'bm.gm_action' AND action = 'bm_list'` | `server` |
| U2 | Walk up to Machra and right-click him | The chat line "The auctioneer opens the Black Market. (No window? The Black Market needs the Cimmeria client patch.)". With the patch, the Black Market window opens on the Search tab | `event = 'bm.open'` (one row, with `auctioneer_entity_id`) | `server` for the line, `patch` for the window |
| U3 | Press Search with every filter empty | The seeded listings, each with its item name and icon, price and a timer icon; the total count above the list. A listing nobody has bid on shows `Min <price>` in the Bid column (the Health Slappacks: `Min 30`), and one with no buyout shows `Bid only` in the Buyout column. Select the slappacks and press Buyout: the status line says "This auction is bid only. Enter at least 30 and press Bid." on the first press | `event = 'bm.search' AND client_key = 0` (`rows_returned`, `total_results`) | `patch + BM-00` |
| U4 | Search `pistol`, then set tech competency 10 to 20 | First every pistol, then only the TC 10, 15 and 20 pistols | `event = 'bm.search'` (`item_name`, `min_tc`, `max_tc` are on the row) | `patch` |
| U5 | Type `.bm_seed 40`, search again, page forward and back | More than one page; Next shows new rows, Back returns to the first page, and the total stays the same | `event = 'bm.search'` (`cursor`, `forward`, `total_results`) | `patch` |
| U6 | Open Create, pick an item from your main bag, start 10, buyout 100, the default duration | The duration labels under the three bars read `Short`, `Medium` and `Long`, each under its own bar. The item leaves your bag and appears on My Auctions | `event = 'bm.listed' AND player_id = <you>` | `patch` |
| U7 | On the second character, open My Bids: it is empty. Search and bid the minimum on character 1's listing | Your cash drops by the bid; the row shows your bid; My Bids lists it | `event = 'bm.bid'` (`bid_before`, `bid_after`, `escrow_cash_after`) | `patch` |
| U8 | Character 2 bids on a seeded listing; then character 1, on the other client, bids higher on the same listing | Character 2's cash comes back at once and its row shows the higher bid | `event = 'bm.outbid_refund'` | `patch` (two clients) |
| U9 | Bid the buyout (40) on the TC 1 pistol | The auction is gone from the list at once; a mail from "Black Market" carrying the pistol arrives | `event = 'bm.sold'`, `event = 'bm.payout' AND reason = 'buyout'`, and `bm_outcome_total{op="bid", outcome="buyout"}` | `patch` |
| U10 | On character 1, list another item, then cancel it from My Auctions | The row is gone and the item comes back by mail from "Black Market" | `event = 'bm.cancelled'`, then `event = 'bm.payout' AND reason = 'cancelled'` | `patch` |
| U11 | Character 2 bids on character 1's U6 listing (if not done), then the GM types `.bm_expire <id>` (the id from `.bm_list`) | The GM gets "Auction N (item) expired and sold to <character 2> for N naquadah. The item is mailed to the buyer and the cash to the seller..."; the row disappears for both | `event = 'bm.gm_action' AND action = 'bm_expire' AND outcome = 'sold'`, then `event = 'bm.sold'` | `server` (the settlement), `patch` (to create the listing and bid) |
| U12 | List an item with no bids, then `.bm_expire <id>` | The GM gets "...expired unsold. The item is mailed back to its seller..." | `event = 'bm.expired' AND auction_id = <id>` | `server` (the settlement), `patch` (to create the listing) |
| U13 | Open the mailbox on the seller and the buyer after U9, U11 and U12, and take the attachments | The buyer's mail carries the item; the seller of a sale gets the winning cash; an unsold seller gets the item back. Every mail is from "Black Market", its Sent date is today (not "Thu Jan 1st, 1970"), and Expires reads 30 days in the inbox (not "Soon") | `event = 'bm.payout' AND auction_id = <id>` (`role`, `reason`, `mail_id`, `cash`, `item_id`) | `server` |

### Error cases

Each refusal must show a message in the window and change nothing. The server sends `onBMError` with the id in brackets below, and the overlay (BM-05) turns it into text, so the exact wording is the overlay's. The server row is `bm.refused` with the `reason`, the id's label, and `error_id`.

| Step | Do | You should see | SigNoz | Needs |
|---|---|---|---|---|
| U14 | Bid less than the minimum next bid (5% over the standing bid, at least 1 more) | The too-low message (`BidTooLow`, 5); cash unchanged | `event = 'bm.refused' AND reason = 'bid_too_low'` | `patch` |
| U15 | Bid more cash than you have | The funds message (`NotEnoughFunds`, 2) | `reason = 'not_enough_funds'` | `patch` |
| U16 | Bid on your own listing | The seller message (`IsSeller`, 4) | `reason = 'is_seller'` | `patch` |
| U17 | Try a 21st active listing (decision D5) | The listing-cap message (`TooManyListings`, 10); the item stays in your bag | `reason = 'too_many_listings'` | `patch` |
| U18 | List a bound item | The bound message (`ItemBound`, 12) | `reason = 'item_bound'` | `patch` |
| U19 | Open the window, walk more than 5 units from Machra, then bid | The auctioneer message (`NotAtAuctioneer`, 9); nothing changes | `event = 'bm.refused' AND reason = 'not_at_auctioneer'` (`access = 'auctioneer_out_of_range'`) | `patch` |
| U20 | Press Watch on a row | The watch message (`WatchUnavailable`, 14, decision D4) | `reason = 'watch_unavailable'` | `patch` |
| U21 | On a client **without** the patch, click Machra, then log out | Only the U2 chat line. At logout the server records that the client never answered | `event = 'bm.open_without_client_call'` | `server` |
| U22 | `.bm_expire 999999`, and `.bm_expire` on the U10 auction | "no auction has id 999999"; "auction N was cancelled by its seller"; nothing changes | `event = 'bm.gm_rejected' AND command = 'bm_expire'` (`reason`) | `server` |
| U23 | On a non-GM character, type `.bm_seed` | ".bm_seed is a GM command; you do not have GM rights"; nothing is listed | `reason = 'not_gm'` (DEBUG) | `server` |

## What the server refuses that a tester cannot reach

These are covered by tests, not by UAT, and listed so a SigNoz row for one is recognisable:

- `bm.open_refused` (`reason = not_at_auctioneer`): a chain ran `open_black_market` at an NPC that is not an auctioneer, or after the player walked away. The player gets "Nobody here runs the Black Market." or "You are too far from the auctioneer."
- `bm.open_unwired` (`reason = auctioneer_without_chain`): an auctioneer was clicked but no chain opened the Black Market. The player gets "The auctioneer is not trading right now."
- `bm.seed_refused` (ERROR, at boot): the reserved system seller ids (account 1, player 1) hold another account or character, so nothing was listed. `found` says what is there. `.bm_seed` refuses the same way, as `bm.gm_rejected` with the same `reason`.

## Recording results

Report each step against its id (`U0` to `U23`) with pass or fail, the time, the character, and a `.bug` note at the moment of any failure. The [unified UAT guide](../../guides/unified-uat.md#recording-results) has the template.
