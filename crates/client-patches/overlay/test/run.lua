-- REPL-style logic UAT for the Black Market UI overlay (BM-05).
--
--   lua5.1 crates/client-patches/overlay/test/run.lua
--
-- Loads the overlay's BlackMarket.lua against stubbed CEGUI windows (built
-- from the overlay's BlackMarket.layout) and a stubbed CimmeriaBMNative, then
-- drives each flow the way the client and the DLL would. Exits non-zero on
-- the first failed check in any scenario.

local here = ((arg and arg[0]) or ''):match('^(.*)[/\\][^/\\]*$') or '.'
local root = here..'/..'
local LUA = root..'/Content/UI/Core/BlackMarket/BlackMarket.lua'
local LAYOUT = root..'/Content/UI/Core/BlackMarket/BlackMarket.layout'
local ERROR_RS = root..'/../../patch-wire/src/black_market/error.rs'

local Stubs = dofile(here..'/stubs.lua')
local WINDOW_NAMES = Stubs.layoutWindowNames(LAYOUT)

local ITEM_DEFS = {}
for id = 100, 160 do
    ITEM_DEFS[id] = { Name = 'Widget '..id, Icon = 'set:Icons image:Item'..id }
end

--=============================================================================
local function eq( actual, expected, what )
    if actual ~= expected then
        error((what or 'value')..': expected '..tostring(expected)..', got '..tostring(actual), 2)
    end
end
local function ok( cond, what )
    if not cond then
        error(what or 'check failed', 2)
    end
end

local function newClient( opts )
    opts = opts or {}
    opts.windowNames = WINDOW_NAMES
    opts.itemDefs = ITEM_DEFS
    local env = Stubs.newEnv(opts)
    Stubs.loadOverlay(env, LUA)
    return env
end

-- A wire item table, keys exactly as the DLL builds them.
local function item( seq, fields )
    local t = {
        sequenceId = seq, itemDefId = 100 + (seq % 50), stackSize = 1, durability = 100,
        charges = 0, currentBid = 10 * seq, buyoutPrice = 50 * seq, endTimeValue = 5,
        nextMinBidPrice = 10 * seq + 1, sellerName = 'Seller'..seq,
    }
    for k, v in pairs(fields or {}) do
        t[k] = v
    end
    return t
end
local function items( first, last )
    local list = {}
    for seq = first, last do
        list[#list + 1] = item(seq)
    end
    return list
end

local function status( env ) return env.BlackMarket_ErrorText:getText() end
local function lastCall( env ) return env.CimmeriaBMNative.calls[#env.CimmeriaBMNative.calls] end
local function callCount( env ) return #env.CimmeriaBMNative.calls end
local function click( env, name ) return env.windows[name]:fire('EventClicked') end
local function row( env, format, n, child ) return env.windows['BlackMarket_'..format..n..(child or 'MainContainer')] end
local function logged( env, needle )
    for i, line in ipairs(env.logLines) do
        if line:find(needle, 1, true) then
            return true
        end
    end
    return false
end

-- Let time pass and run one frame.
local function tick( env, seconds )
    env.clock = env.clock + seconds
    env.BlackMarketWin:fire('Events.PreRender')
end

-- Open the window as the DLL would, and answer the opening search.
local function openWithResults( env, list, total )
    env.CimmeriaBM.onOpen(4242)
    env.CimmeriaBM.onAuctions(list, total, 0)
end

--=============================================================================
local scenarios = {}
local function scenario( name, fn ) scenarios[#scenarios + 1] = { name = name, fn = fn } end

scenario('loads cleanly: window hidden, every row hidden, no errors logged', function()
    local env = newClient()
    eq(env.BlackMarketWin.visible, false, 'window visible after load')
    for i, format in ipairs({ 'Search', 'MyAuction', 'MyBids', 'Watched' }) do
        for n = 1, 8 do
            ok(row(env, format, n), 'missing row '..format..n)
            eq(row(env, format, n).visible, false, format..n..' visible')
        end
    end
    ok(not logged(env, 'failed'), 'a handler failed while loading')
    ok(logged(env, '[Cimmeria BM] overlay loaded, CimmeriaBMNative version test'), 'load line missing')
end)

scenario('every window the Lua names exists in the patched layout (U7, U8, status line)', function()
    local src = Stubs.readFile(LUA)
    -- Whole names only: a quoted prefix such as 'BlackMarket_Tab'..i is built
    -- at run time, and the row scenarios cover those.
    for name, after in src:gmatch('(BlackMarket_[%w_]+)(.)') do
        if after ~= "'" and after ~= '"' then
            ok(WINDOW_NAMES[name], 'Lua uses '..name..', the layout has no such window')
        end
    end
    ok(WINDOW_NAMES['BlackMarket_ErrorText'], 'status line missing')
    ok(WINDOW_NAMES['BlackMarket_TabButton4'], 'tab button 4 missing')
end)

scenario('CimmeriaBM receive functions are plain fields', function()
    local env = newClient()
    eq(getmetatable(env.CimmeriaBM), nil, 'CimmeriaBM metatable')
    for i, name in ipairs({ 'onOpen', 'onError', 'onAuctions', 'onAuctionRemove', 'onAuctionUpdate', 'onWatchedItems' }) do
        eq(type(rawget(env.CimmeriaBM, name)), 'function', 'CimmeriaBM.'..name)
    end
end)

scenario('open: window shows, an opening search goes out with the fixed contract keys', function()
    local env = newClient()
    env.CimmeriaBM.onOpen(4242)
    eq(env.BlackMarketWin.visible, true, 'window visible')
    eq(status(env), 'Searching...', 'status on open')
    eq(env.BlackMarket_SearchButton.enabled, false, 'search button while searching')
    local call = lastCall(env)
    eq(call.op, 'search', 'op')
    local o = call.args[1]
    eq(o.clientKey, 0, 'clientKey'); eq(o.sequenceId, 0, 'sequenceId'); eq(o.bForward, true, 'bForward')
    eq(o.sortId, 0, 'sortId'); eq(o.sellerName, '', 'sellerName'); eq(o.bidderName, '', 'bidderName')
    eq(o.itemName, '', 'itemName'); eq(o.minTC, 0, 'minTC'); eq(o.maxTC, 0, 'maxTC')
    eq(o.quality, 1, 'quality (the Normal entry is preselected)'); eq(o.filterFlags, 0, 'filterFlags')
end)

scenario('Events.BMOpen still opens the window (stock entry point)', function()
    local env = newClient()
    env.BlackMarketWin:fireGame('Events.BMOpen')
    eq(env.BlackMarketWin.visible, true, 'window visible')
    eq(lastCall(env).op, 'search', 'opening search')
end)

scenario('search results render from the Lua store (read bindings, U10, U11)', function()
    local env = newClient()
    env.CimmeriaBMNative.tc[101] = 7
    local list = items(1, 10)
    list[1].stackSize = 5
    list[1].endTimeValue = 1
    list[2].endTimeValue = 3
    openWithResults(env, list, 30)
    eq(status(env), 'Found 30 auctions.', 'status')
    eq(row(env, 'Search', 1).visible, true, 'row 1 visible')
    eq(row(env, 'Search', 1):getID(), 1, 'row 1 carries the auction id')
    eq(row(env, 'Search', 1, 'NameText'):getText(), 'Widget 101', 'name from getItemDefInfo')
    eq(row(env, 'Search', 1, 'Icon'):getProperty('Image'), 'set:Icons image:Item101', 'icon')
    eq(row(env, 'Search', 1, 'TCText'):getText(), '7', 'TC from techCompetency')
    eq(row(env, 'Search', 2, 'TCText'):getText(), '', 'TC blank when the DLL returns nil')
    eq(row(env, 'Search', 1, 'QtyText'):getText(), '5', 'stack size')
    eq(row(env, 'Search', 1, 'TimerImage'):getProperty('Image'), 'set:CoreWidgets2_2 image:MeasurementBars_Low', 'timer, VeryShort')
    eq(row(env, 'Search', 2, 'TimerImage'):getProperty('Image'), 'set:CoreWidgets2_2 image:MeasurementBars_Medium', 'timer, Medium')
    eq(row(env, 'Search', 3, 'TimerImage'):getProperty('Image'), 'set:CoreWidgets2_2 image:MeasurementBars_High', 'timer, VeryLong')
    eq(row(env, 'Search', 1, 'CurrentBidText'):getText(), '10', 'current bid')
    eq(row(env, 'Search', 1, 'BuyoutText'):getText(), '50', 'buyout')
    eq(row(env, 'Search', 1, 'SellerText'):getText(), 'Seller1', 'seller')
    eq(row(env, 'Search', 8).visible, true, 'row 8 visible')
    local info = env.BlackMarketMod.getAuctionItemInfo(3)
    eq(info.auctionId, 3, 'info.auctionId'); eq(info.nextBidPrice, 31, 'info.nextBidPrice')
    eq(info.timeLeft, 5, 'info.timeLeft'); eq(info.bidCount, 0, 'info.bidCount')
    eq(next(env.BlackMarketMod.getAuctionItemInfo(999)), nil, 'a miss is an empty table')
    eq(env.BlackMarketMod.getAuctionTotalCount(0), 30, 'total')
    eq(env.BlackMarketMod.getAuctionVisibleCount(0), 10, 'visible')
    -- scrolling within the page
    env.BlackMarket_SearchScroll:setScrollPosition(2)
    eq(row(env, 'Search', 1):getID(), 3, 'row 1 after scrolling two')
    eq(row(env, 'Search', 8):getID(), 10, 'row 8 after scrolling two')
    -- a short page hides the unused rows
    env.CimmeriaBM.onAuctions(items(1, 3), 3, 0)
    eq(row(env, 'Search', 4).visible, false, 'row 4 hidden on a 3-row page')
end)

scenario('paging: page text, Next/Prev enable, cursor and direction (U2, U3, U4)', function()
    local env = newClient()
    openWithResults(env, items(1, 10), 30)
    eq(env.BlackMarket_SearchPageText:getText(), '1/3', 'page text')
    eq(env.BlackMarket_NextSearchPageButton.enabled, false, 'Next while the search delay runs')
    tick(env, 5)
    eq(env.BlackMarket_NextSearchPageButton.enabled, true, 'Next with more results')
    eq(env.BlackMarket_PrevSearchPageButton.enabled, false, 'Prev on page 1')
    eq(env.BlackMarket_SearchButton.enabled, true, 'Search after the delay')

    click(env, 'BlackMarket_NextSearchPageButton')
    eq(status(env), 'Searching...', 'Next feedback')
    local o = lastCall(env).args[1]
    eq(o.sequenceId, 10, 'Next cursor is the highest id shown'); eq(o.bForward, true, 'Next direction')
    env.CimmeriaBM.onAuctions(items(11, 20), 30, 0)
    tick(env, 5)
    eq(env.BlackMarket_SearchPageText:getText(), '2/3', 'page 2 text')
    eq(env.BlackMarket_PrevSearchPageButton.enabled, true, 'Prev on page 2')
    eq(env.BlackMarket_NextSearchPageButton.enabled, true, 'Next on page 2')

    click(env, 'BlackMarket_NextSearchPageButton')
    env.CimmeriaBM.onAuctions(items(21, 30), 30, 0)
    tick(env, 5)
    eq(env.BlackMarket_SearchPageText:getText(), '3/3', 'page 3 text')
    eq(env.BlackMarket_NextSearchPageButton.enabled, false, 'Next on the last page')

    click(env, 'BlackMarket_PrevSearchPageButton')
    o = lastCall(env).args[1]
    eq(o.sequenceId, 21, 'Prev cursor is the lowest id shown'); eq(o.bForward, false, 'Prev direction')
    env.CimmeriaBM.onAuctions(items(11, 20), 30, 0)
    tick(env, 5)
    eq(env.BlackMarket_SearchPageText:getText(), '2/3', 'back on page 2')
    click(env, 'BlackMarket_PrevSearchPageButton')
    env.CimmeriaBM.onAuctions(items(1, 10), 30, 0)
    tick(env, 5)
    eq(env.BlackMarket_SearchPageText:getText(), '1/3', 'back on page 1')
    eq(env.BlackMarket_PrevSearchPageButton.enabled, false, 'Prev on page 1 again')

    -- a new search starts over and sends the filters
    env.BlackMarket_SearchItemText:setText('zat')
    env.BlackMarket_MinTC:setText('3')
    click(env, 'BlackMarket_SearchButton')
    o = lastCall(env).args[1]
    eq(o.itemName, 'zat', 'itemName'); eq(o.minTC, 3, 'minTC'); eq(o.sequenceId, 0, 'fresh cursor')
    env.CimmeriaBM.onAuctions({}, 0, 0)
    eq(status(env), 'No auctions match your search.', 'empty result status')
    eq(env.BlackMarket_SearchPageText:getText(), '0/0', 'empty page text')
end)

scenario('row selection highlights one row and fills the bid box (U5, U6)', function()
    local env = newClient()
    openWithResults(env, items(1, 10), 10)
    row(env, 'Search', 1):fire('EventMouseButtonDown')
    eq(row(env, 'Search', 1, 'Highlight').visible, true, 'row 1 highlighted')
    eq(env.BlackMarket_NewBidText:getText(), '11', 'next bid in the bid box')
    eq(env.BlackMarket_SearchBidButton.enabled, true, 'Bid enabled')
    eq(env.BlackMarket_SearchBuyoutButton.enabled, true, 'Buyout enabled')
    row(env, 'Search', 2):fire('EventMouseButtonDown')
    eq(row(env, 'Search', 1, 'Highlight').visible, false, 'row 1 unhighlighted')
    eq(row(env, 'Search', 2, 'Highlight').visible, true, 'row 2 highlighted')
    eq(env.BlackMarketMod.viewData[0].selectedAuctionId, 2, 'selection')
    -- cannot afford
    env.cash = 5
    env.BlackMarketWin:fireGame('Events.InventoryUpdateCash', 5)
    eq(env.BlackMarket_SearchBidButton.enabled, false, 'Bid disabled when short of cash')
end)

scenario('Bid: immediate feedback, bid() sent, update clears the pending state (U1)', function()
    local env = newClient()
    openWithResults(env, items(1, 10), 10)
    row(env, 'Search', 2):fire('EventMouseButtonDown')
    local before = callCount(env)
    env.BlackMarket_NewBidText:setText('5')
    click(env, 'BlackMarket_SearchBidButton')
    eq(status(env), 'Your bid must be at least 21.', 'too-low bid is refused locally')
    eq(callCount(env), before, 'nothing sent for a too-low bid')

    env.BlackMarket_NewBidText:setText('25')
    click(env, 'BlackMarket_SearchBidButton')
    eq(status(env), 'Bid sent...', 'Bid feedback')
    eq(env.BlackMarket_SearchBidButton.enabled, false, 'Bid disabled while pending')
    local call = lastCall(env)
    eq(call.op, 'bid', 'op'); eq(call.args[1], 2, 'sequenceId'); eq(call.args[2], 25, 'amount')

    env.CimmeriaBM.onAuctionUpdate(item(2, { currentBid = 25, nextMinBidPrice = 27 }))
    eq(status(env), 'Bid placed.', 'accepted')
    eq(env.BlackMarket_SearchBidButton.enabled, true, 'Bid enabled again')
    eq(row(env, 'Search', 2, 'CurrentBidText'):getText(), '25', 'row shows the new bid')
    eq(env.BlackMarket_NewBidText:getText(), '27', 'bid box shows the new minimum')
    ok(env.BlackMarketMod.viewContains(2, 2), 'the auction is in My Bids')
end)

scenario('Buyout: immediate feedback, bid() at the buyout price, removal settles it (U1)', function()
    local env = newClient()
    openWithResults(env, items(1, 10), 10)
    row(env, 'Search', 3):fire('EventMouseButtonDown')
    click(env, 'BlackMarket_SearchBuyoutButton')
    eq(status(env), 'Buyout sent...', 'Buyout feedback')
    local call = lastCall(env)
    eq(call.op, 'bid', 'op'); eq(call.args[1], 3, 'sequenceId'); eq(call.args[2], 150, 'buyout price')
    env.CimmeriaBM.onAuctionRemove(3)
    eq(status(env), 'You bought the item. It will arrive by mail.', 'bought')
    ok(not env.BlackMarketMod.viewContains(0, 3), 'removed from the results')
    eq(env.BlackMarketMod.getAuctionTotalCount(0), 9, 'total drops by one')
    eq(env.BlackMarketMod.viewData[0].selectedAuctionId, 0, 'selection cleared')
    eq(env.BlackMarket_SearchBidButton.enabled, false, 'nothing selected, Bid disabled')
end)

scenario('onError maps every BMError id to text and clears pending requests', function()
    local env = newClient()
    local src = Stubs.readFile(ERROR_RS)
    local count = 0
    for name, id in src:gmatch('\n%s+(%w+) = (%d+),') do
        count = count + 1
        ok(env.BlackMarketMod.ERROR_TEXT[tonumber(id)], 'no text for BMError::'..name..' = '..id)
    end
    eq(count, 15, 'BMError variants parsed from error.rs')

    openWithResults(env, items(1, 10), 10)
    row(env, 'Search', 2):fire('EventMouseButtonDown')
    env.BlackMarket_NewBidText:setText('21')
    click(env, 'BlackMarket_SearchBidButton')
    env.CimmeriaBM.onError(5)
    eq(status(env), 'Your bid is below the minimum bid.', 'BidTooLow text')
    eq(env.BlackMarket_ErrorText:getProperty('TextColours'):sub(1, 11), 'tl:FFFF6060', 'error colour')
    eq(env.BlackMarketMod.pending.bid, nil, 'pending bid cleared')
    eq(env.BlackMarket_SearchBidButton.enabled, true, 'Bid enabled again')
    env.CimmeriaBM.onError(99)
    eq(status(env), 'The Black Market refused the request (error 99).', 'unknown id')
    ok(logged(env, '[Cimmeria BM] onError id=99'), 'error logged')
end)

scenario('Create: form checks, create() in .def order, ack clears the form (U12)', function()
    local env = newClient()
    env.CimmeriaBM.onOpen(4242)
    click(env, 'BlackMarket_TabButton2')
    eq(status(env), 'Loading your auctions...', 'tab 2 feedback')
    local o = lastCall(env).args[1]
    eq(lastCall(env).op, 'search', 'tab 2 refreshes My Auctions (U9)'); eq(o.clientKey, 1, 'clientKey 1')
    env.CimmeriaBM.onAuctions({}, 0, 1)
    eq(status(env), 'You have no active auctions.', 'empty My Auctions')

    local before = callCount(env)
    click(env, 'BlackMarket_CreateButton')
    eq(status(env), 'Drag an item from your bags onto this tab first.', 'Create with no item gives feedback')
    eq(callCount(env), before, 'nothing sent')

    env.bags['1:4'] = { itemId = 777, itemDefId = 120, qty = 3 }
    env.drag = { env.UIDragType.Item, { container = 1, slot = 4 } }
    env.BlackMarket_Tab2:fire('EventDragDropItemDropped')
    eq(env.BlackMarket_NewAuctionImage:getID(), 777, 'item instance on the form')
    eq(env.BlackMarket_NewAuctionQtyText:getText(), '3', 'quantity')
    env.BlackMarket_NewAuctionBidText:setText('100')
    env.BlackMarket_NewAuctionBuyoutText:setText('50')
    env.BlackMarket_NewAuctionBidText:fire('EventTextChanged')
    eq(env.BlackMarket_CreateButton.enabled, false, 'buyout below start disables Create')
    env.BlackMarket_NewAuctionBuyoutText:setText('500')
    env.BlackMarket_NewAuctionBuyoutText:fire('EventTextChanged')
    eq(env.BlackMarket_CreateButton.enabled, true, 'valid form enables Create')
    env.BlackMarket_DurationShortImage:fire('EventMouseButtonDown')

    click(env, 'BlackMarket_CreateButton')
    eq(status(env), 'Creating auction...', 'Create feedback')
    eq(env.BlackMarket_CreateButton.enabled, false, 'Create disabled while pending')
    local call = lastCall(env)
    eq(call.op, 'create', 'op'); eq(call.args[1], 777, 'itemInstanceId'); eq(call.args[2], 100, 'startingPrice')
    eq(call.args[3], 500, 'buyoutPrice'); eq(call.args[4], 3, 'auctionLength (Medium, the short button)')

    env.CimmeriaBM.onAuctionUpdate(item(40, { itemDefId = 120, currentBid = 0, buyoutPrice = 500 }))
    eq(status(env), 'Auction created.', 'ack')
    eq(env.BlackMarket_NewAuctionImage:getID(), 0, 'form cleared')
    eq(env.BlackMarket_NewAuctionBidText:getText(), '', 'bid box cleared')
    eq(row(env, 'MyAuction', 1).visible, true, 'My Auctions row 1 shown (U7)')
    eq(row(env, 'MyAuction', 1, 'NameText'):getText(), 'Widget 120', 'the new listing')
    eq(row(env, 'MyAuction', 1, 'BidderText'):getText(), '', 'bidder column blank')
    eq(env.BlackMarketMod.savedItemPrices[120].bid, 100, 'prices remembered')
end)

scenario('Cancel: feedback with no selection, cancel() sent, removal confirms', function()
    local env = newClient()
    env.CimmeriaBM.onOpen(4242)
    click(env, 'BlackMarket_TabButton2')
    env.CimmeriaBM.onAuctions(items(1, 2), 2, 1)
    eq(status(env), 'You have 2 active auctions.', 'My Auctions status')
    local before = callCount(env)
    click(env, 'BlackMarket_CancelButton')
    eq(status(env), 'Select one of your auctions first.', 'Cancel with no selection gives feedback')
    eq(callCount(env), before, 'nothing sent')

    row(env, 'MyAuction', 2):fire('EventMouseButtonDown')
    eq(row(env, 'MyAuction', 2, 'Highlight').visible, true, 'My Auctions rows are clickable (U6)')
    click(env, 'BlackMarket_CancelButton')
    eq(status(env), 'Cancelling auction...', 'Cancel feedback')
    eq(env.BlackMarket_CancelButton.enabled, false, 'Cancel disabled while pending')
    eq(lastCall(env).op, 'cancel', 'op'); eq(lastCall(env).args[1], 2, 'sequenceId')
    env.CimmeriaBM.onAuctionRemove(2)
    eq(status(env), 'Auction cancelled. The item is back in your bags.', 'confirmed')
    eq(env.BlackMarket_CancelButton.enabled, true, 'Cancel enabled again')
    eq(row(env, 'MyAuction', 2).visible, false, 'row gone')
end)

scenario('My Bids tab: refresh on open, rows render and select (U8, U9)', function()
    local env = newClient()
    env.CimmeriaBM.onOpen(4242)
    click(env, 'BlackMarket_TabButton3')
    eq(env.BlackMarket_Tab3.visible, true, 'tab 3 shown'); eq(env.BlackMarket_Tab1.visible, false, 'tab 1 hidden')
    eq(status(env), 'Loading your bids...', 'tab 3 feedback')
    eq(lastCall(env).args[1].clientKey, 2, 'clientKey 2')
    env.CimmeriaBM.onAuctions(items(5, 7), 3, 2)
    eq(status(env), 'You have bids on 3 auctions.', 'My Bids status')
    eq(row(env, 'MyBids', 3).visible, true, 'My Bids row 3 shown')
    eq(row(env, 'MyBids', 4).visible, false, 'My Bids row 4 hidden')
    row(env, 'MyBids', 1):fire('EventMouseButtonDown')
    eq(row(env, 'MyBids', 1, 'Highlight').visible, true, 'My Bids row selectable')
    -- results for another view never land in this one
    env.CimmeriaBM.onAuctions(items(1, 1), 1, 0)
    eq(env.BlackMarketMod.getAuctionVisibleCount(2), 3, 'My Bids untouched by a search reply')
end)

scenario('Watched tab: one-line refusal (D4), no send; watched items render', function()
    local env = newClient()
    env.CimmeriaBM.onOpen(4242)
    local before = callCount(env)
    click(env, 'BlackMarket_TabButton4')
    eq(env.BlackMarket_Tab4.visible, true, 'tab 4 shown')
    eq(status(env), 'The watch list is not available yet.', 'watch refusal')
    eq(callCount(env), before, 'nothing sent')
    env.CimmeriaBM.onWatchedItems({ 101, 102 })
    eq(row(env, 'Watched', 2, 'NameText'):getText(), 'Widget 102', 'watched item shown')
    eq(row(env, 'Watched', 3).visible, false, 'unused watched row hidden')
end)

scenario('missing CimmeriaBMNative: one clear line, no errors, every button answers', function()
    local env = newClient({ native = false })
    env.BlackMarketWin:fireGame('Events.BMOpen')
    eq(env.BlackMarketWin.visible, true, 'window still opens')
    eq(status(env), 'The Black Market needs the Cimmeria client patch', 'open')
    env.BlackMarket_ErrorText:setText('')
    click(env, 'BlackMarket_SearchButton')
    eq(status(env), 'The Black Market needs the Cimmeria client patch', 'Search')
    env.BlackMarket_ErrorText:setText('')
    click(env, 'BlackMarket_TabButton2')
    eq(status(env), 'The Black Market needs the Cimmeria client patch', 'tab 2')
    click(env, 'BlackMarket_CancelButton')
    eq(status(env), 'Select one of your auctions first.', 'Cancel still answers')
    ok(not logged(env, 'failed'), 'no handler failed')
    ok(logged(env, 'CimmeriaBMNative missing'), 'logged')
    eq(row(env, 'Search', 1).visible, false, 'no rows')
end)

scenario('CimmeriaBMNative registered after the file loaded is found at open and press time', function()
    -- The DLL registers the table from its Tick detour, which can run after
    -- the UI has loaded BlackMarket.lua.
    local env = newClient({ native = false })
    ok(logged(env, 'CimmeriaBMNative not present yet'), 'load noted the missing table')
    env.CimmeriaBMNative = Stubs.newNative(env)
    env.CimmeriaBM.onOpen(4242)
    eq(status(env), 'Searching...', 'open searches once the table exists')
    eq(lastCall(env).op, 'search', 'search sent')
    env.CimmeriaBM.onAuctions(items(1, 2), 2, 0)
    row(env, 'Search', 1):fire('EventMouseButtonDown')
    click(env, 'BlackMarket_SearchBidButton')
    eq(lastCall(env).op, 'bid', 'bid sent through the late table')
    eq(row(env, 'Search', 1, 'TCText'):getText(), '', 'TC blank when techCompetency returns nil (D7 fallback)')
end)

scenario('form input reaches the native as whole INT32 numbers and short strings', function()
    -- CimmeriaBMNative refuses "5", 2.5 and out-of-range numbers (bad_args),
    -- and strings over 255 UTF-8 bytes.
    local function isInt( v ) return type(v) == 'number' and v == math.floor(v) and v >= -2147483648 and v <= 2147483647 end
    local env = newClient()
    env.CimmeriaBM.onOpen(4242)
    tick(env, 5)
    env.BlackMarket_SearchItemText:setText(string.rep('z', 300))
    env.BlackMarket_MinTC:setText('2.5')
    env.BlackMarket_MaxTC:setText('9e99')
    click(env, 'BlackMarket_SearchButton')
    local o = lastCall(env).args[1]
    eq(string.len(o.itemName), 85, 'item-name filter trimmed')
    eq(o.minTC, 2, 'fractional minTC floored'); eq(o.maxTC, 0, 'out-of-range maxTC dropped')
    for k, v in pairs(o) do
        if type(v) ~= 'string' and type(v) ~= 'boolean' then
            ok(isInt(v), 'search.'..k..' is not a whole INT32: '..tostring(v))
        end
    end

    env.CimmeriaBM.onAuctions(items(1, 2), 2, 0)
    row(env, 'Search', 2):fire('EventMouseButtonDown')
    env.BlackMarket_NewBidText:setText('25.9')
    click(env, 'BlackMarket_SearchBidButton')
    eq(lastCall(env).op, 'bid', 'bid sent'); eq(lastCall(env).args[2], 25, 'fractional bid floored')
    env.CimmeriaBM.onError(5)
    env.BlackMarket_NewBidText:setText('1e12')
    local before = callCount(env)
    click(env, 'BlackMarket_SearchBidButton')
    eq(callCount(env), before, 'out-of-range bid not sent')
    eq(status(env), 'Your bid must be at least 21.', 'out-of-range bid refused locally')

    env.bags['1:1'] = { itemId = 900, itemDefId = 130, qty = 1 }
    env.drag = { env.UIDragType.Item, { container = 1, slot = 1 } }
    env.BlackMarket_Tab2:fire('EventDragDropItemDropped')
    env.BlackMarket_NewAuctionBidText:setText('100.7')
    env.BlackMarket_NewAuctionBuyoutText:setText('-5')
    env.BlackMarket_NewAuctionBuyoutText:fire('EventTextChanged')
    eq(env.BlackMarket_CreateButton.enabled, false, 'negative buyout disables Create')
    env.BlackMarket_NewAuctionBuyoutText:setText('abc')
    env.BlackMarket_NewAuctionBuyoutText:fire('EventTextChanged')
    eq(env.BlackMarket_CreateButton.enabled, false, 'non-numeric buyout disables Create')
    env.BlackMarket_NewAuctionBuyoutText:setText('')
    env.BlackMarket_NewAuctionBuyoutText:fire('EventTextChanged')
    eq(env.BlackMarket_CreateButton.enabled, true, 'no buyout is allowed')
    click(env, 'BlackMarket_CreateButton')
    local call = lastCall(env)
    eq(call.op, 'create', 'create sent')
    eq(call.args[2], 100, 'fractional start floored'); eq(call.args[3], 0, 'no buyout is 0')
    for i = 1, 4 do
        ok(isInt(call.args[i]), 'create arg '..i..' is not a whole INT32: '..tostring(call.args[i]))
    end
    ok(call.args[4] >= 1 and call.args[4] <= 5, 'auctionLength in 1..5')
end)

scenario('a refused send says why and re-enables the button', function()
    local env = newClient()
    env.CimmeriaBMNative.reply.search = { nil, 'offline' }
    env.CimmeriaBM.onOpen(4242)
    eq(status(env), 'You are not connected to the server.', 'offline text')
    eq(env.BlackMarket_SearchButton.enabled, true, 'Search usable again')
    ok(logged(env, '[Cimmeria BM] send search refused: offline'), 'logged')
    env.CimmeriaBMNative.reply.search = nil
    -- a native that throws counts as a refusal
    env.CimmeriaBMNative.bid = function() error('boom') end
    openWithResults(env, items(1, 2), 2)
    row(env, 'Search', 1):fire('EventMouseButtonDown')
    click(env, 'BlackMarket_SearchBidButton')
    ok(status(env):find('could not send', 1, true), 'thrown send reported: '..status(env))
    eq(env.BlackMarket_SearchBidButton.enabled, true, 'Bid usable again')
end)

scenario('no reply: pending request times out with a message', function()
    local env = newClient()
    openWithResults(env, items(1, 2), 2)
    row(env, 'Search', 1):fire('EventMouseButtonDown')
    click(env, 'BlackMarket_SearchBidButton')
    eq(env.BlackMarket_SearchBidButton.enabled, false, 'pending')
    tick(env, 5)
    eq(env.BlackMarket_SearchBidButton.enabled, false, 'still pending at 5s')
    tick(env, 11)
    eq(status(env), 'The Black Market did not answer. Try again.', 'timeout text')
    eq(env.BlackMarket_SearchBidButton.enabled, true, 'Bid usable again')
    eq(env.BlackMarketWin:subscriberCount('Events.PreRender'), 0, 'frame hook removed when idle')
end)

scenario('handlers are pcall-guarded and log [Cimmeria BM]', function()
    local env = newClient()
    env.getCash = function() error('no cash binding') end
    env.BlackMarketWin:fireGame('Events.InventoryUpdateCash', 1)
    ok(logged(env, '[Cimmeria BM] BlackMarketMod.onCashUpdate failed'), 'UI handler guarded')
    env.CimmeriaBM.onAuctions('not a table', 1, 0)
    ok(logged(env, '[Cimmeria BM] CimmeriaBM.onAuctions failed'), 'DLL entry guarded')
    env.CimmeriaBM.onAuctions({}, 0, 7)
    ok(logged(env, 'onAuctions: unknown clientKey 7'), 'unknown view logged')
    env.CimmeriaBM.onAuctionUpdate(nil)
    ok(logged(env, 'onAuctionUpdate: malformed item'), 'malformed update logged')
end)

scenario('reopening while open refreshes; reopening during the search delay says so', function()
    local env = newClient()
    openWithResults(env, items(1, 2), 2)
    tick(env, 5)
    local before = callCount(env)
    env.CimmeriaBM.onOpen(4242)
    eq(callCount(env), before + 1, 'a second open searches again')
    env.CimmeriaBM.onAuctions(items(1, 1), 1, 0)
    env.BlackMarketWin:hide()
    env.CimmeriaBM.onOpen(4242)
    eq(status(env), 'Press Search to list auctions.', 'open inside the delay')
    eq(env.BlackMarketMod.getAuctionVisibleCount(0), 0, 'store starts clean')
end)

-- A player's click: CEGUI fires EventClicked only on an enabled button.
local function press( env, name )
    if env.windows[name].enabled then
        return click(env, name)
    end
end

scenario('bid-only listing: Min bid, "Bid only", and Buyout answers on the first press (all views)', function()
    -- The seeded Health Slappack: starting bid 30, no buyout, no bid yet.
    local bidOnly = function() return item(1, { currentBid = 0, buyoutPrice = 0, nextMinBidPrice = 30 }) end
    local env = newClient()
    openWithResults(env, { bidOnly(), item(2) }, 2)
    eq(row(env, 'Search', 1, 'CurrentBidText'):getText(), 'Min 30', 'no bid yet: the Bid column shows the minimum')
    eq(row(env, 'Search', 1, 'BuyoutText'):getText(), 'Bid only', 'no buyout: the Buyout column says so')
    eq(row(env, 'Search', 2, 'CurrentBidText'):getText(), '20', 'a standing bid is shown as it is')
    eq(row(env, 'Search', 2, 'BuyoutText'):getText(), '100', 'a buyout price is shown as it is')

    row(env, 'Search', 1):fire('EventMouseButtonDown')
    eq(env.BlackMarket_NewBidText:getText(), '30', 'the bid box holds the minimum')
    local before = callCount(env)
    press(env, 'BlackMarket_SearchBuyoutButton')
    eq(status(env), 'This auction is bid only. Enter at least 30 and press Bid.', 'Buyout on a bid-only row explains itself')
    eq(callCount(env), before, 'nothing sent')
    press(env, 'BlackMarket_SearchBidButton')
    eq(lastCall(env).op, 'bid', 'Bid still works'); eq(lastCall(env).args[2], 30, 'at the minimum')

    -- My Auctions and My Bids use the same row population.
    click(env, 'BlackMarket_TabButton2')
    env.CimmeriaBM.onAuctions({ bidOnly() }, 1, 1)
    eq(row(env, 'MyAuction', 1, 'CurrentBidText'):getText(), 'Min 30', 'My Auctions Bid column')
    eq(row(env, 'MyAuction', 1, 'BuyoutText'):getText(), 'Bid only', 'My Auctions Buyout column')
    click(env, 'BlackMarket_TabButton3')
    env.CimmeriaBM.onAuctions({ bidOnly() }, 1, 2)
    eq(row(env, 'MyBids', 1, 'CurrentBidText'):getText(), 'Min 30', 'My Bids Bid column')
    eq(row(env, 'MyBids', 1, 'BuyoutText'):getText(), 'Bid only', 'My Bids Buyout column')
    -- Watched rows are item definitions, not auctions: no prices at all.
    env.CimmeriaBM.onWatchedItems({ 101 })
    eq(row(env, 'Watched', 1, 'CurrentBidText'):getText(), '', 'Watched Bid column blank')
    eq(row(env, 'Watched', 1, 'BuyoutText'):getText(), '', 'Watched Buyout column blank')
end)

scenario('Create tab: Short/Medium/Long labels fit their slots and do not overlap', function()
    -- "SHORMEDIUM ONG" on the first live run: the stock labels were 32, 60
    -- and 24 px wide, with Medium's box over Short's, so CEGUI clipped them.
    local xml = Stubs.readFile(LAYOUT):gsub('<!%-%-.-%-%->', '')
    local WIDTH = 160 -- the duration container, {{0,285},..,{0,445},..}
    local function span( rect )
        local sx, ox, ex, eo = rect:match('^{{([%d.%-]+),([%d.%-]+)},{[^}]+},{([%d.%-]+),([%d.%-]+)}')
        ok(sx, 'unparsed rect '..tostring(rect))
        return tonumber(sx) * WIDTH + tonumber(ox), tonumber(ex) * WIDTH + tonumber(eo)
    end
    local labels = {}
    for text, rect in xml:gmatch('<Property Name="Text" Value="([^"]*)" />%s*<Property Name="Font" Value="Verdana_6pt" />.-<Property Name="UnifiedAreaRect" Value="([^"]*)" />') do
        local key = text:lower()
        if key == 'short' or key == 'medium' or key == 'long' then
            labels[#labels + 1] = { key = key, text = text, left = select(1, span(rect)), right = select(2, span(rect)) }
        end
    end
    eq(#labels, 3, 'three duration labels')
    for i, l in ipairs(labels) do
        -- About 7 px per character at Verdana 6pt covers capitals too.
        ok(l.right - l.left >= 7 * string.len(l.text), l.text..' label is '..(l.right - l.left)..' px, too narrow')
        ok(l.left >= 0 and l.right <= WIDTH, l.text..' label leaves the container')
        local name = 'BlackMarket_Duration'..l.key:sub(1, 1):upper()..l.key:sub(2)
        local bl, br = span(xml:match('Name="'..name..'Image".-<Property Name="UnifiedAreaRect" Value="([^"]*)"'))
        ok(bl >= l.left and br <= l.right, l.text..' button is not under its label')
        local hl, hr = span(xml:match('Name="'..name..'HighlightImage".-<Property Name="UnifiedAreaRect" Value="([^"]*)"'))
        ok(hl <= bl and hr >= br, l.text..' highlight does not cover its button')
        for j = i + 1, #labels do
            local m = labels[j]
            ok(l.right <= m.left or m.right <= l.left, l.text..' and '..m.text..' labels overlap')
        end
    end
end)

-- The Access-bar suite (access.lua) registers its scenarios here too.
dofile(here..'/access.lua')(here, scenario, eq, ok)

--=============================================================================
local failed = 0
for i, s in ipairs(scenarios) do
    local okRun, err = pcall(s.fn)
    if okRun then
        print('ok   '..s.name)
    else
        failed = failed + 1
        print('FAIL '..s.name..'\n     '..tostring(err))
    end
end
print(string.format('%d scenarios, %d failed', #scenarios, failed))
if failed > 0 then
    os.exit(1)
end
