-- Global table to avoid name clashes
BlackMarketMod = {}

-- Cimmeria overlay (BM-05). The stock client shipped this module unfinished:
-- the C++ store behind the four read bindings, the bid wiring and the error
-- text were never written. This version keeps the auction data in Lua, fed by
-- the Cimmeria client-patch DLL through the CimmeriaBM table below, and sends
-- through the DLL's CimmeriaBMNative table. See
-- crates/client-patches/overlay/README.md for the full list of changes.

BlackMarketMod.MAX_SEARCH_ROWS = 8
BlackMarketMod.MAX_AUCTION_ROWS = 8
BlackMarketMod.MAX_BID_ROWS = 8
BlackMarketMod.MAX_WATCHED_ROWS = 8
BlackMarketMod.SEARCH_DELAY = 4
-- Seconds to wait for the server to answer a bid, create, cancel or load.
BlackMarketMod.PENDING_TIMEOUT = 15

-- The Watched tab has no UIAuctionView value; the watch list is deferred (D4).
BlackMarketMod.VIEW_WATCHED = 3

BlackMarketMod.LOG_TAG = '[Cimmeria BM] '
BlackMarketMod.PATCH_MISSING_TEXT = 'The Black Market needs the Cimmeria client patch'

BlackMarketMod.selectedSearchAuctionId = 0

BlackMarketMod.savedItemPrices = {}
BlackMarketMod.creationDuration = 5

BlackMarketMod.searchSuspended = false
BlackMarketMod.searchDelayTime = 0
BlackMarketMod.preRenderSubscribed = false
BlackMarketMod.currentTab = 1

-- In-flight requests, keyed by operation: { id = ..., time = ... }
BlackMarketMod.pending = {}

-- onBMError ids (crates/patch-wire/src/black_market/error.rs, decision D3).
BlackMarketMod.ERROR_TEXT = {
    [0]  = 'The Black Market did not understand that request.',
    [1]  = 'The Black Market is unavailable right now.',
    [2]  = 'You cannot afford that bid.',
    [3]  = 'That auction has ended.',
    [4]  = 'You cannot bid on your own auction.',
    [5]  = 'Your bid is below the minimum bid.',
    [6]  = 'That item cannot be put up for auction.',
    [7]  = 'Only the seller can cancel that auction.',
    [8]  = 'The Black Market had a problem. Nothing was changed.',
    [9]  = 'You must be at an auctioneer to do that.',
    [10] = 'You already have the most auctions allowed (20).',
    [11] = 'The starting bid must be at least 1, and a buyout no lower than the starting bid.',
    [12] = 'Bound items cannot be put up for auction.',
    [13] = 'Your bags are full.',
    [14] = 'The watch list is not available yet.',
}

-- Reasons a CimmeriaBMNative send can return with nil.
BlackMarketMod.SEND_FAILURE_TEXT = {
    offline = 'You are not connected to the server.',
    bad_args = 'That request was not valid.',
    not_main_thread = 'The client patch could not send the request.',
    engine_error = 'The client patch could not send the request.',
}

--=============================================================================
-- Logging, guards and status line
--=============================================================================
function BlackMarketMod.log( text )
    -- The client's own Lua log, which the launcher tails (plan 5.1).
    local line = BlackMarketMod.LOG_TAG..tostring(text)
    local ok = pcall(function() Debug:log( line ) end)
    if not ok then
        pcall(print, line)
    end
end

-- Wrap a handler so an error is logged instead of thrown into the UI.
function BlackMarketMod.guard( name, fn )
    return function(...)
        local result = { pcall(fn, ...) }
        if result[1] then
            return unpack(result, 2, table.maxn(result))
        end
        BlackMarketMod.log( name..' failed: '..tostring(result[2]) )
        return nil
    end
end

function BlackMarketMod.setStatus( text, isError )
    if not BlackMarket_ErrorText then
        return
    end
    BlackMarket_ErrorText:setText( text or '' )
    if isError then
        BlackMarket_ErrorText:setProperty( 'TextColours', 'tl:FFFF6060 tr:FFFF6060 bl:FFFF6060 br:FFFF6060' )
    else
        BlackMarket_ErrorText:setProperty( 'TextColours', 'tl:FFC3F3FF tr:FFC3F3FF bl:FFC3F3FF br:FFC3F3FF' )
    end
end

function BlackMarketMod.errorText( errorId )
    local id = tonumber(errorId)
    return BlackMarketMod.ERROR_TEXT[id] or ('The Black Market refused the request (error '..tostring(errorId)..').')
end

-- CimmeriaBMNative refuses anything but a whole number in INT32 range, so
-- form text is coerced here. nil when the text is not a usable number.
BlackMarketMod.INT32_MAX = 2147483647
function BlackMarketMod.toInt( value )
    local n = tonumber(value)
    if not n or n ~= n then
        return nil
    end
    n = math.floor(n)
    if n < -BlackMarketMod.INT32_MAX - 1 or n > BlackMarketMod.INT32_MAX then
        return nil
    end
    return n
end

-- Strings go out as at most 255 UTF-8 bytes. The host's strings are wide,
-- so this counts characters: 85 fits even at three bytes each.
BlackMarketMod.MAX_NAME_CHARS = 85

function BlackMarketMod.now()
    local ok, t = pcall(worldGetDeltaSeconds)
    if ok and type(t) == 'number' then
        return t
    end
    return 0
end

--=============================================================================
-- Sending, through the client-patch DLL
--=============================================================================
function BlackMarketMod.native()
    local native = rawget(_G, 'CimmeriaBMNative')
    if type(native) == 'table' then
        return native
    end
    return nil
end

function BlackMarketMod.hasPatch()
    return BlackMarketMod.native() ~= nil
end

-- Returns true when the DLL accepted the request. On failure the status line
-- already says why.
function BlackMarketMod.send( op, ... )
    local native = BlackMarketMod.native()
    if not native or type(native[op]) ~= 'function' then
        BlackMarketMod.log( 'send '..op..' skipped: CimmeriaBMNative missing' )
        BlackMarketMod.setStatus( BlackMarketMod.PATCH_MISSING_TEXT, true )
        return false
    end
    local ok, sent, reason = pcall(native[op], ...)
    if not ok then
        reason = tostring(sent)
        sent = nil
    end
    if sent then
        return true
    end
    reason = reason or 'unknown'
    BlackMarketMod.log( 'send '..op..' refused: '..tostring(reason) )
    BlackMarketMod.setStatus( BlackMarketMod.SEND_FAILURE_TEXT[reason] or ('The client patch could not send the request ('..tostring(reason)..').'), true )
    return false
end

function BlackMarketMod.setPending( op, id )
    BlackMarketMod.pending[op] = { id = id, time = BlackMarketMod.now() }
    BlackMarketMod.ensurePreRender()
end

function BlackMarketMod.clearPending( op )
    BlackMarketMod.pending[op] = nil
end

function BlackMarketMod.clearAllPending()
    BlackMarketMod.pending = {}
    BlackMarketMod.viewData[UIAuctionView.SearchResults].request = nil
end

--=============================================================================
-- Lua-side store. Replaces the C++ read bindings getAuctionItemInfo,
-- getAuctionViewItems, getAuctionTotalCount and getAuctionVisibleCount.
--=============================================================================
function BlackMarketMod.resetStore()
    BlackMarketMod.auctions = {}
    BlackMarketMod.views = {}
    BlackMarketMod.views[UIAuctionView.SearchResults] = { ids = {}, total = 0 }
    BlackMarketMod.views[UIAuctionView.MyAuctions] = { ids = {}, total = 0 }
    BlackMarketMod.views[UIAuctionView.MyBids] = { ids = {}, total = 0 }
    BlackMarketMod.watchedItems = {}
end

function BlackMarketMod.getAuctionViewItems( viewType )
    if viewType == BlackMarketMod.VIEW_WATCHED then
        return BlackMarketMod.watchedItems
    end
    local view = BlackMarketMod.views[viewType]
    if view then
        return view.ids
    end
    return {}
end

function BlackMarketMod.getAuctionTotalCount( viewType )
    if viewType == BlackMarketMod.VIEW_WATCHED then
        return #BlackMarketMod.watchedItems
    end
    local view = BlackMarketMod.views[viewType]
    if view then
        return view.total
    end
    return 0
end

function BlackMarketMod.getAuctionVisibleCount( viewType )
    return #BlackMarketMod.getAuctionViewItems( viewType )
end

function BlackMarketMod.itemDefInfo( itemDefId )
    local ok, info = pcall(getItemDefInfo, itemDefId)
    if ok and type(info) == 'table' then
        return info
    end
    return {}
end

-- Decision D7: the DLL reads tech competency from the item cache. Blank when
-- it cannot.
function BlackMarketMod.techCompetency( itemDefId )
    local native = BlackMarketMod.native()
    if native and type(native.techCompetency) == 'function' then
        local ok, tc = pcall(native.techCompetency, itemDefId)
        if ok and type(tc) == 'number' then
            return tc
        end
    end
    return ''
end

-- The table the C++ getAuctionItemInfo built (FUN_00ae1ad0); empty on a miss.
function BlackMarketMod.getAuctionItemInfo( auctionId )
    local item = BlackMarketMod.auctions[auctionId]
    if not item then
        return {}
    end
    local def = BlackMarketMod.itemDefInfo( item.itemDefId )
    local info = {}
    info.auctionId = item.sequenceId
    info.itemId = item.itemDefId
    info.name = def.Name or ('Item '..tostring(item.itemDefId))
    info.icon = def.Icon or ''
    info.techCompentancy = BlackMarketMod.techCompetency( item.itemDefId )
    info.timeLeft = item.endTimeValue or UIAuctionTime.VeryLong
    info.charges = item.charges or 0
    info.currentBid = item.currentBid or 0
    info.buyoutPrice = item.buyoutPrice or 0
    info.durability = item.durability or 0
    info.nextBidPrice = item.nextMinBidPrice or 0
    info.sellerName = item.sellerName or ''
    info.stackSize = item.stackSize or 1
    info.bidderName = ''
    info.bidCount = 0
    return info
end

function BlackMarketMod.viewContains( viewType, auctionId )
    for i, id in ipairs(BlackMarketMod.getAuctionViewItems( viewType )) do
        if id == auctionId then
            return i
        end
    end
    return nil
end

-- Drop auctions no view lists any more.
function BlackMarketMod.pruneAuctions()
    local keep = {}
    for viewType, view in pairs(BlackMarketMod.views) do
        for i, id in ipairs(view.ids) do
            keep[id] = true
        end
    end
    for id, item in pairs(BlackMarketMod.auctions) do
        if not keep[id] then
            BlackMarketMod.auctions[id] = nil
        end
    end
end

--=============================================================================
-- Receiving: the DLL calls these plain functions, with no self.
--=============================================================================
CimmeriaBM = {}

function CimmeriaBM.onOpen( entityId )
    BlackMarketMod.entityId = entityId
    BlackMarketMod.onBMOpen()
end

function CimmeriaBM.onError( errorId )
    local text = BlackMarketMod.errorText( errorId )
    BlackMarketMod.log( 'onError id='..tostring(errorId)..' text='..text )
    BlackMarketMod.clearAllPending()
    BlackMarketMod.onBMError( nil, text )
    BlackMarketMod.refreshButtons()
end

function CimmeriaBM.onAuctions( items, totalResults, clientKey )
    local viewType = tonumber(clientKey)
    local view = BlackMarketMod.views[viewType]
    if not view then
        BlackMarketMod.log( 'onAuctions: unknown clientKey '..tostring(clientKey) )
        return
    end

    local ids = {}
    for i, item in ipairs(items or {}) do
        if type(item) == 'table' and item.sequenceId then
            BlackMarketMod.auctions[item.sequenceId] = item
            ids[#ids + 1] = item.sequenceId
        end
    end
    view.ids = ids
    view.total = math.max(tonumber(totalResults) or #ids, #ids)
    BlackMarketMod.pruneAuctions()

    local viewData = BlackMarketMod.viewData[viewType]
    if viewData.selectedAuctionId > 0 and not BlackMarketMod.viewContains( viewType, viewData.selectedAuctionId ) then
        viewData.selectedAuctionId = 0
    end

    if viewType == UIAuctionView.SearchResults then
        BlackMarketMod.applySearchPage( #ids )
        BlackMarketMod.clearPending( 'search' )
        if #ids == 0 then
            BlackMarketMod.setStatus( 'No auctions match your search.' )
        else
            BlackMarketMod.setStatus( 'Found '..view.total..' auctions.' )
        end
    elseif viewType == UIAuctionView.MyAuctions then
        BlackMarketMod.clearPending( 'view1' )
        if BlackMarketMod.currentTab == 2 and not BlackMarketMod.pending.create and not BlackMarketMod.pending.cancel then
            if #ids == 0 then
                BlackMarketMod.setStatus( 'You have no active auctions.' )
            else
                BlackMarketMod.setStatus( 'You have '..view.total..' active auctions.' )
            end
        end
    elseif viewType == UIAuctionView.MyBids then
        BlackMarketMod.clearPending( 'view2' )
        if BlackMarketMod.currentTab == 3 then
            if #ids == 0 then
                BlackMarketMod.setStatus( 'You have no active bids.' )
            else
                BlackMarketMod.setStatus( 'You have bids on '..view.total..' auctions.' )
            end
        end
    end

    -- The C++ store would have fired Events.BMViewUpdate here.
    BlackMarketMod.onBMViewUpdate( nil, viewType )
    BlackMarketMod.refreshButtons()
end

function CimmeriaBM.onAuctionRemove( sequenceId )
    for viewType, view in pairs(BlackMarketMod.views) do
        local index = BlackMarketMod.viewContains( viewType, sequenceId )
        if index then
            table.remove(view.ids, index)
            view.total = math.max(view.total - 1, #view.ids)
            local viewData = BlackMarketMod.viewData[viewType]
            if viewData.selectedAuctionId == sequenceId then
                viewData.selectedAuctionId = 0
            end
            BlackMarketMod.rerenderView( viewType )
        end
    end
    BlackMarketMod.auctions[sequenceId] = nil

    local pending = BlackMarketMod.pending
    if pending.cancel and pending.cancel.id == sequenceId then
        BlackMarketMod.clearPending( 'cancel' )
        BlackMarketMod.setStatus( 'Auction cancelled. The item is back in your bags.' )
    elseif pending.bid and pending.bid.id == sequenceId then
        BlackMarketMod.clearPending( 'bid' )
        BlackMarketMod.setStatus( 'You bought the item. It will arrive by mail.' )
    end
    BlackMarketMod.refreshButtons()
end

function CimmeriaBM.onAuctionUpdate( item )
    if type(item) ~= 'table' or not item.sequenceId then
        BlackMarketMod.log( 'onAuctionUpdate: malformed item' )
        return
    end
    local sequenceId = item.sequenceId
    BlackMarketMod.auctions[sequenceId] = item

    local pending = BlackMarketMod.pending
    local listed = false
    for viewType, view in pairs(BlackMarketMod.views) do
        if BlackMarketMod.viewContains( viewType, sequenceId ) then
            listed = true
        end
    end

    if pending.bid and pending.bid.id == sequenceId then
        BlackMarketMod.clearPending( 'bid' )
        BlackMarketMod.addToView( UIAuctionView.MyBids, sequenceId )
        BlackMarketMod.setStatus( 'Bid placed.' )
    elseif not listed and pending.create and pending.create.itemDefId == item.itemDefId then
        -- A new listing: the create was accepted (U12).
        BlackMarketMod.clearPending( 'create' )
        BlackMarketMod.addToView( UIAuctionView.MyAuctions, sequenceId )
        BlackMarketMod.clearAuctionCreateItem()
        BlackMarketMod.setStatus( 'Auction created.' )
    end

    for viewType, view in pairs(BlackMarketMod.views) do
        if BlackMarketMod.viewContains( viewType, sequenceId ) then
            BlackMarketMod.rerenderView( viewType )
        end
    end

    local searchData = BlackMarketMod.viewData[UIAuctionView.SearchResults]
    if searchData.selectedAuctionId == sequenceId then
        BlackMarket_NewBidText:setText( item.nextMinBidPrice or 0 )
    end
    BlackMarketMod.pruneAuctions()
    BlackMarketMod.refreshButtons()
end

function CimmeriaBM.onWatchedItems( itemList )
    local list = {}
    for i, itemDefId in ipairs(itemList or {}) do
        list[#list + 1] = itemDefId
    end
    BlackMarketMod.watchedItems = list
    BlackMarketMod.resetView( BlackMarketMod.VIEW_WATCHED )
end

function BlackMarketMod.addToView( viewType, sequenceId )
    local view = BlackMarketMod.views[viewType]
    if view and not BlackMarketMod.viewContains( viewType, sequenceId ) then
        view.ids[#view.ids + 1] = sequenceId
        view.total = view.total + 1
        BlackMarketMod.rerenderView( viewType )
    end
end

--=============================================================================
function BlackMarketMod.onBMOpen()
    local ok, wasVisible = pcall(function() return BlackMarketWin:isVisible() end)
    BlackMarketWin:show()
    if not ok then
        wasVisible = false
    end
    if wasVisible then
        BlackMarketMod.onWindowShow()
    end
end

--=============================================================================
function BlackMarketMod.onBMError(this, errorText )
    BlackMarketMod.setStatus( errorText, true )
end

--=============================================================================
function BlackMarketMod.onCloseClicked( this, window )
    BlackMarketWin:hide()
end

--=============================================================================
function BlackMarketMod.onWindowShow( this, window )
    BlackMarketMod.setupWindow()
    BlackMarketMod.openSession()
end

--=============================================================================
function BlackMarketMod.setupWindow()
    BlackMarketMod.updateCash()
    BlackMarketMod.refreshQualityCombo()
    BlackMarketMod.clearAuctionCreateItem()
    BlackMarketMod.selectCreateDuration( UIAuctionTime.VeryLong )

--    BlackMarketMod.clearSearchFilters()
end

-- The DLL keeps no state, so every open starts clean and asks for listings.
function BlackMarketMod.openSession()
    BlackMarketMod.resetStore()
    BlackMarketMod.clearAllPending()
    for viewType, viewData in pairs(BlackMarketMod.viewData) do
        viewData.selectedAuctionId = 0
        viewData.currentPage = 0
        viewData.offset = 0
    end
    BlackMarketMod.showTab( 1 )
    BlackMarketMod.resetAllViews()

    if not BlackMarketMod.hasPatch() then
        BlackMarketMod.log( 'window opened without CimmeriaBMNative' )
        BlackMarketMod.setStatus( BlackMarketMod.PATCH_MISSING_TEXT, true )
        return
    end
    if BlackMarketMod.searchSuspended then
        BlackMarketMod.setStatus( 'Press Search to list auctions.' )
    else
        BlackMarketMod.onSearchClicked()
    end
end

function BlackMarketMod.resetAllViews()
    for viewType, viewData in pairs(BlackMarketMod.viewData) do
        BlackMarketMod.resetView( viewType )
    end
end
--=============================================================================
function BlackMarketMod.addSearchQuality( text, colour, id, selected )
    local entry = CEGUI.createListboxTextItem( text )
    entry:setSelectionColours(CEGUI.colour(0.0, 0.0, 1.0, 1.0))
    entry:setSelectionBrushImage("TaharezLook", "ListboxSelectionBrush")
    entry:setTextColours( colour )
    entry:setID( id )
    BlackMarket_SearchQualityCombo:addItem(entry)

    if selected then
        BlackMarket_SearchQualityCombo:setText( text )
        BlackMarket_SearchQualityCombo:setItemSelectState( entry, true )
    end
end

--=============================================================================
function BlackMarketMod.refreshQualityCombo()
    BlackMarket_SearchQualityCombo:resetList()
    BlackMarket_SearchQualityCombo:setText('')
    BlackMarket_SearchQualityCombo:setSortingEnabled(false)
    BlackMarketMod.addSearchQuality( localize('Global', 'ItemQualityNormal'), CEGUI.colour(.7,.7,.7,1), UIItemQuality.Normal, true )   -- White
    BlackMarketMod.addSearchQuality( localize('Global', 'ItemQualityGood'), CEGUI.colour(0,.8,0,1), UIItemQuality.Good )             -- Green
    BlackMarketMod.addSearchQuality( localize('Global', 'ItemQualityGreat'), CEGUI.colour(.2,.2,1,1), UIItemQuality.Great )           -- Blue
    BlackMarketMod.addSearchQuality( localize('Global', 'ItemQualityFantastic'), CEGUI.colour(.5,0,.5,1), UIItemQuality.Fantastic ) -- Purple
end

--=============================================================================
function BlackMarketMod.updateCash()
    local myCash = getCash()
    BlackMarket_MyCashText:setText( myCash )
    BlackMarketMod.updateBidInfo( BlackMarketMod.viewData[UIAuctionView.SearchResults].selectedAuctionId )
end

--=============================================================================
function BlackMarketMod.updateBidInfo( auctionId )
    local bidPending = BlackMarketMod.pending.bid ~= nil
    if auctionId > 0 then
        local itemInfo = BlackMarketMod.getAuctionItemInfo( auctionId )
        if itemInfo.auctionId then
            local myCash = tonumber(getCash()) or 0
            BlackMarket_SearchBidButton:setEnabled( not bidPending and myCash >= itemInfo.nextBidPrice )
            -- A bid-only auction keeps Buyout pressable, so the press can
            -- say why there is nothing to buy out (a disabled button is
            -- silent on the first press).
            BlackMarket_SearchBuyoutButton:setEnabled( not bidPending and (itemInfo.buyoutPrice <= 0 or myCash >= itemInfo.buyoutPrice) )
            return
        end
    end
    BlackMarket_SearchBidButton:setEnabled( false )
    BlackMarket_SearchBuyoutButton:setEnabled( false )
end

--=============================================================================
-- Re-derive every button's enabled state from the selection and what is in flight.
function BlackMarketMod.refreshButtons()
    BlackMarketMod.updateBidInfo( BlackMarketMod.viewData[UIAuctionView.SearchResults].selectedAuctionId )
    BlackMarketMod.updateNewAuctionReady()
    BlackMarket_CancelButton:setEnabled( BlackMarketMod.pending.cancel == nil )
    BlackMarketMod.updateSearchNavButtons()
end

--=============================================================================
function BlackMarketMod.onNewAuctionDragReceived( this, hitWindow, droppedWindow )
    -- see what was dropped on us
    local dragType, dragInfo = getDragInfo()

    -- if it was an item, use it as the component
    if UIDragType.Item == dragType then
        BlackMarketMod.setAuctionCreateItem( dragInfo.container, dragInfo.slot )
        BlackMarket_NewAuctionBidText:activate()
    end
end

--=============================================================================
function BlackMarketMod.clearAuctionCreateItem()
    BlackMarket_NewAuctionImage:setProperty( 'Image', '' )
    BlackMarket_NewAuctionImage:setID( 0 )
    BlackMarket_NewAuctionQtyText:setText( '' )
    BlackMarket_NewAuctionBidText:setText( '' )
    BlackMarket_NewAuctionBuyoutText:setText( '' )
    BlackMarketMod.updateNewAuctionReady()
end

--=============================================================================
function BlackMarketMod.setAuctionCreateItem( container, slot )
    local itemId = getItemIDForSlot(container, slot)
    local itemDefId = getItemDef(itemId)
    local itemInfo = getItemDefInfo( itemDefId )
    if itemInfo.ID then
        BlackMarket_NewAuctionImage:setProperty( 'Image', itemInfo.Icon )
        BlackMarket_NewAuctionImage:setID( itemId )

        local qty = getQuantityForSlot(container, slot)
        if qty > 1 then
            BlackMarket_NewAuctionQtyText:setText( qty )
        else
            BlackMarket_NewAuctionQtyText:setText( '' )
        end

        if BlackMarketMod.savedItemPrices[itemDefId] then
            BlackMarket_NewAuctionBidText:setText( BlackMarketMod.savedItemPrices[itemDefId].bid )
            BlackMarket_NewAuctionBuyoutText:setText( BlackMarketMod.savedItemPrices[itemDefId].buyout )
        else
            BlackMarket_NewAuctionBidText:setText( '' )
            BlackMarket_NewAuctionBuyoutText:setText( '' )
        end

        -- TODO: Item Tooltip here

        BlackMarketMod.updateNewAuctionReady()
    end
end

--=============================================================================
function BlackMarketMod.onCreateClicked( this, window )
    local itemId = BlackMarket_NewAuctionImage:getID()
    if itemId <= 0 then
        BlackMarketMod.setStatus( 'Drag an item from your bags onto this tab first.', true )
        return
    end
    if BlackMarketMod.pending.create then
        BlackMarketMod.setStatus( 'Still creating your last auction...' )
        return
    end
    if not BlackMarketMod.updateNewAuctionReady() then
        BlackMarketMod.setStatus( BlackMarketMod.ERROR_TEXT[11], true )
        return
    end

    local itemDefId = getItemDef(itemId)
    local bid = BlackMarketMod.toInt(BlackMarket_NewAuctionBidText:getText()) or 0
    local buyout = BlackMarketMod.toInt(BlackMarket_NewAuctionBuyoutText:getText()) or 0

    -- Save the desired prices for this item so we can recall it if they put up more of the same
    BlackMarketMod.savedItemPrices[itemDefId] = {}
    BlackMarketMod.savedItemPrices[itemDefId].bid = bid
    BlackMarketMod.savedItemPrices[itemDefId].buyout = buyout

    BlackMarketMod.setPending( 'create', itemId )
    BlackMarketMod.pending.create.itemDefId = itemDefId
    BlackMarketMod.setStatus( 'Creating auction...' )
    BlackMarketMod.updateNewAuctionReady()

    if not BlackMarketMod.send( 'create', itemId, bid, buyout, BlackMarketMod.creationDuration ) then
        BlackMarketMod.clearPending( 'create' )
        BlackMarketMod.updateNewAuctionReady()
    end
end

--=============================================================================
function BlackMarketMod.onCancelClicked( this, window )
    local auctionId = BlackMarketMod.viewData[UIAuctionView.MyAuctions].selectedAuctionId
    if auctionId <= 0 then
        BlackMarketMod.setStatus( 'Select one of your auctions first.', true )
        return
    end
    if BlackMarketMod.pending.cancel then
        BlackMarketMod.setStatus( 'Still cancelling...' )
        return
    end

    BlackMarketMod.setPending( 'cancel', auctionId )
    BlackMarketMod.setStatus( 'Cancelling auction...' )
    BlackMarket_CancelButton:setEnabled( false )

    if not BlackMarketMod.send( 'cancel', auctionId ) then
        BlackMarketMod.clearPending( 'cancel' )
        BlackMarket_CancelButton:setEnabled( true )
    end
end

--=============================================================================
-- Bid and Buyout (U1): the stock layout had the buttons but nothing subscribed.
function BlackMarketMod.placeBid( amount, sentText )
    local auctionId = BlackMarketMod.viewData[UIAuctionView.SearchResults].selectedAuctionId
    if auctionId <= 0 then
        BlackMarketMod.setStatus( 'Select an auction first.', true )
        return
    end
    local itemInfo = BlackMarketMod.getAuctionItemInfo( auctionId )
    if not itemInfo.auctionId then
        BlackMarketMod.setStatus( BlackMarketMod.ERROR_TEXT[3], true )
        return
    end
    if not amount or amount < itemInfo.nextBidPrice then
        BlackMarketMod.setStatus( 'Your bid must be at least '..itemInfo.nextBidPrice..'.', true )
        return
    end

    BlackMarketMod.setPending( 'bid', auctionId )
    BlackMarketMod.setStatus( sentText )
    BlackMarketMod.updateBidInfo( auctionId )

    if not BlackMarketMod.send( 'bid', auctionId, math.floor(amount) ) then
        BlackMarketMod.clearPending( 'bid' )
        BlackMarketMod.updateBidInfo( auctionId )
    end
end

function BlackMarketMod.onBidClicked( this, window )
    BlackMarketMod.placeBid( BlackMarketMod.toInt(BlackMarket_NewBidText:getText()), 'Bid sent...' )
end

function BlackMarketMod.onBuyoutClicked( this, window )
    local auctionId = BlackMarketMod.viewData[UIAuctionView.SearchResults].selectedAuctionId
    local itemInfo = BlackMarketMod.getAuctionItemInfo( auctionId )
    if itemInfo.auctionId and itemInfo.buyoutPrice <= 0 then
        BlackMarketMod.setStatus( 'This auction is bid only. Enter at least '..itemInfo.nextBidPrice..' and press Bid.', true )
        return
    end
    BlackMarketMod.placeBid( itemInfo.buyoutPrice, 'Buyout sent...' )
end

--=============================================================================
function BlackMarketMod.createSearchFilter()
    local searchFilter = {}
    searchFilter.sortId = 0
    searchFilter.auctionId = 0
    searchFilter.forward = true
    searchFilter.sellerName = ""
    searchFilter.bidderName = ""
    searchFilter.itemName = ""
    searchFilter.minTC = 0
    searchFilter.maxTC = 0
    searchFilter.quality = 0
    searchFilter.filterFlags = 0

    return searchFilter
end

--=============================================================================
function BlackMarketMod.onSearchClicked( this, window )
    local searchFilter = BlackMarketMod.createSearchFilter()

    --TODO: Filters based on UI state
    searchFilter.sortId = 0
    searchFilter.auctionId = 0
    searchFilter.forward = true
    searchFilter.sellerName = ''
    searchFilter.bidderName = ''
    searchFilter.itemName = string.sub(BlackMarket_SearchItemText:getText() or '', 1, BlackMarketMod.MAX_NAME_CHARS)
    searchFilter.minTC = math.max(BlackMarketMod.toInt(BlackMarket_MinTC:getText()) or 0, 0)
    searchFilter.maxTC = math.max(BlackMarketMod.toInt(BlackMarket_MaxTC:getText()) or 0, 0)
    searchFilter.quality = 0

    if BlackMarket_SearchQualityCombo:getSelectedItem() then
        searchFilter.quality = BlackMarket_SearchQualityCombo:getSelectedItem():getID()
    end

    searchFilter.filterFlags = 0

    -- Reset to the first page
    BlackMarketMod.viewData[UIAuctionView.SearchResults].searchFilter = searchFilter
    BlackMarketMod.viewData[UIAuctionView.SearchResults].request = { page = 0, reset = true }

    BlackMarketMod.performSearch( searchFilter )
end

--=============================================================================
function BlackMarketMod.onSearchRowMouseDown( this, window )
    BlackMarketMod.selectRow( UIAuctionView.SearchResults, (window or this):getID() )
end

--=============================================================================
function BlackMarketMod.onMyAuctionsRowMouseDown( this, window )
    BlackMarketMod.selectRow( UIAuctionView.MyAuctions, (window or this):getID() )
end

--=============================================================================
function BlackMarketMod.onMyBidsRowMouseDown( this, window )
    BlackMarketMod.selectRow( UIAuctionView.MyBids, (window or this):getID() )
end

--=============================================================================
function BlackMarketMod.onScrolled( this, window )
    BlackMarketMod.refreshView( window:getID(), window:getScrollPosition() )
end

--=============================================================================
function BlackMarketMod.selectRow( viewType, auctionId )
    local viewData = BlackMarketMod.viewData[viewType]
    if not viewData or viewType == BlackMarketMod.VIEW_WATCHED then
        return
    end

    local itemInfo = BlackMarketMod.getAuctionItemInfo( auctionId )
    if itemInfo.auctionId then
        local currentSelectedAuctionId = viewData.selectedAuctionId
        if currentSelectedAuctionId ~= auctionId then
            -- Deselect the old row
            if currentSelectedAuctionId > 0 then
                local rowId = viewData.auctionToRow[currentSelectedAuctionId]
                if rowId then
                    local rowHighlight = _G['BlackMarket_'..viewData.format..rowId..'Highlight']
                    if rowHighlight then
                        rowHighlight:hide()
                    end
                end

                viewData.selectedAuctionId = 0
            end

            -- Select the new row
            if auctionId > 0 then
                local rowId = viewData.auctionToRow[ auctionId ]
                if rowId then
                    local rowHighlight = _G['BlackMarket_'..viewData.format..rowId..'Highlight']
                    if rowHighlight then
                        rowHighlight:show()
                    end
                end

                viewData.selectedAuctionId = auctionId
                viewData.selectionCall( itemInfo )

            end
        end
    end
end

--=============================================================================
function BlackMarketMod.searchSelectionUpdate( itemInfo )
    BlackMarketMod.updateBidInfo( BlackMarketMod.viewData[UIAuctionView.SearchResults].selectedAuctionId )
    BlackMarket_NewBidText:setText( itemInfo.nextBidPrice )
end

--=============================================================================
function BlackMarketMod.myAuctionsSelectionUpdate( itemInfo )

end

--=============================================================================
function BlackMarketMod.myBidsSelectionUpdate( itemInfo )

end

--=============================================================================
function BlackMarketMod.watchedSelectionUpdate( itemInfo )

end

--=============================================================================
-- Paging (U2-U4). The server returns one page per search, cut to fit one
-- message, with the full match count. offset is the index of the page's first
-- row in the whole result.
function BlackMarketMod.applySearchPage( count )
    local viewData = BlackMarketMod.viewData[UIAuctionView.SearchResults]
    local request = viewData.request
    if request then
        if request.reset then
            viewData.offset = 0
        elseif request.forward then
            viewData.offset = request.fromOffset + request.fromCount
        else
            viewData.offset = math.max(request.fromOffset - count, 0)
        end
        viewData.currentPage = request.page
        viewData.request = nil
    end
    if viewData.offset <= 0 then
        viewData.offset = 0
        viewData.currentPage = 0
    end
    viewData.pageSize = math.max(viewData.pageSize or 0, count)
end

function BlackMarketMod.refreshSearchView()
    local viewData = BlackMarketMod.viewData[UIAuctionView.SearchResults]
    local visCount = BlackMarketMod.getAuctionVisibleCount(UIAuctionView.SearchResults)
    local totalCount = BlackMarketMod.getAuctionTotalCount(UIAuctionView.SearchResults)

    if totalCount <= 0 then
        BlackMarket_SearchPageText:setText( '0/0' )
    else
        local page = viewData.currentPage + 1
        local totalPages = math.ceil(totalCount / math.max(viewData.pageSize or visCount, 1))
        if viewData.offset + visCount < totalCount then
            totalPages = math.max(totalPages, page + 1)
        end
        totalPages = math.max(totalPages, page)
        BlackMarket_SearchPageText:setText( page..'/'..totalPages )
    end

    BlackMarketMod.updateSearchNavButtons()
end

--=============================================================================
function BlackMarketMod.refreshMyAuctionsView()

end

--=============================================================================
function BlackMarketMod.refreshMyBidsView()

end

--=============================================================================
function BlackMarketMod.refreshWatchedView()

end

--=============================================================================
function BlackMarketMod.performSearch( searchFilter )
    -- Save the time they began a search
    BlackMarketMod.searchDelayTime = BlackMarketMod.now()

    -- Flag the search buttons as suspended
    BlackMarketMod.searchSuspended = true
    BlackMarketMod.setPending( 'search', 0 )
    BlackMarketMod.setStatus( 'Searching...' )
    BlackMarketMod.updateSearchNavButtons()

    local sent = BlackMarketMod.send( 'search', BlackMarketMod.searchOptions( searchFilter, UIAuctionView.SearchResults ) )
    if not sent then
        BlackMarketMod.searchSuspended = false
        BlackMarketMod.clearPending( 'search' )
        BlackMarketMod.viewData[UIAuctionView.SearchResults].request = nil
        BlackMarketMod.updateSearchNavButtons()
    end
end

-- The table CimmeriaBMNative.search takes: the BMSearch arguments by .def name.
function BlackMarketMod.searchOptions( searchFilter, clientKey )
    local opts = {}
    opts.sortId = searchFilter.sortId
    opts.clientKey = clientKey
    opts.sequenceId = searchFilter.auctionId
    opts.bForward = searchFilter.forward and true or false
    opts.sellerName = searchFilter.sellerName
    opts.bidderName = searchFilter.bidderName
    opts.itemName = searchFilter.itemName
    opts.minTC = searchFilter.minTC
    opts.maxTC = searchFilter.maxTC
    opts.quality = searchFilter.quality
    opts.filterFlags = searchFilter.filterFlags
    return opts
end

-- My Auctions and My Bids (U9): the server fills them from the caller's own
-- player id, so the filter is otherwise empty.
function BlackMarketMod.requestView( viewType )
    if viewType == UIAuctionView.MyAuctions then
        BlackMarketMod.setStatus( 'Loading your auctions...' )
    else
        BlackMarketMod.setStatus( 'Loading your bids...' )
    end
    BlackMarketMod.setPending( 'view'..viewType, 0 )
    if not BlackMarketMod.send( 'search', BlackMarketMod.searchOptions( BlackMarketMod.createSearchFilter(), viewType ) ) then
        BlackMarketMod.clearPending( 'view'..viewType )
    end
end

--=============================================================================
function BlackMarketMod.ensurePreRender()
    if not BlackMarketMod.preRenderSubscribed then
        BlackMarketWin:subscribe( Events.PreRender, 'BlackMarketMod.onPreRender' )
        BlackMarketMod.preRenderSubscribed = true
    end
end

function BlackMarketMod.onPreRender( window, view )
    local now = BlackMarketMod.now()

    -- If enough time has passed, enabled the search buttons again
    if BlackMarketMod.searchSuspended then
        BlackMarketMod.searchSuspended = (now - BlackMarketMod.searchDelayTime) < BlackMarketMod.SEARCH_DELAY
        if BlackMarketMod.searchSuspended == false then
            BlackMarketMod.updateSearchNavButtons()
        end
    end

    -- Give up on requests the server never answered, so no button stays dead.
    local timedOut = false
    for op, entry in pairs(BlackMarketMod.pending) do
        if now - entry.time >= BlackMarketMod.PENDING_TIMEOUT then
            BlackMarketMod.pending[op] = nil
            timedOut = true
            BlackMarketMod.log( 'no reply to '..op )
        end
    end
    if timedOut then
        BlackMarketMod.viewData[UIAuctionView.SearchResults].request = nil
        BlackMarketMod.setStatus( 'The Black Market did not answer. Try again.', true )
        BlackMarketMod.refreshButtons()
    end

    if not BlackMarketMod.searchSuspended and next(BlackMarketMod.pending) == nil then
        BlackMarketWin:unsubscribe( Events.PreRender )
        BlackMarketMod.preRenderSubscribed = false
    end
end

--=============================================================================
function BlackMarketMod.onBMViewUpdate( this, viewType )
    BlackMarketMod.resetView( viewType )
end

--=============================================================================
function BlackMarketMod.resetView( viewType )
    local auctionList = BlackMarketMod.getAuctionViewItems(viewType)
    local scrollBar = BlackMarketMod.viewData[viewType].scrollBar
    local visRows = BlackMarketMod.viewData[viewType].maxVis

    BlackMarketMod.updateScroll( scrollBar, visRows, #auctionList )
    local scrollIndex = scrollBar:getScrollPosition()

    BlackMarketMod.refreshView( viewType, scrollIndex )
end

--=============================================================================
-- Redraw a view after one row changed, keeping the scroll position.
function BlackMarketMod.rerenderView( viewType )
    local auctionList = BlackMarketMod.getAuctionViewItems(viewType)
    local scrollBar = BlackMarketMod.viewData[viewType].scrollBar
    local visRows = BlackMarketMod.viewData[viewType].maxVis
    local scrollIndex = math.floor(scrollBar:getScrollPosition())

    BlackMarketMod.updateScroll( scrollBar, visRows, #auctionList )
    scrollIndex = math.max(math.min(scrollIndex, #auctionList - visRows), 0)
    scrollBar:setScrollPosition( scrollIndex )

    BlackMarketMod.refreshView( viewType, scrollIndex )
end

--=============================================================================
function BlackMarketMod.refreshView( viewType, scrollIndex )
    local auctionList = BlackMarketMod.getAuctionViewItems(viewType)
    local scrollBar = BlackMarketMod.viewData[viewType].scrollBar
    local visRows = BlackMarketMod.viewData[viewType].maxVis

    BlackMarketMod.viewData[viewType].auctionToRow = {}

    -- Call the custom view reset function
    BlackMarketMod.viewData[viewType].refreshCall()

    -- Make sure our scroll index is a whole number
    scrollIndex = math.floor(scrollIndex)

    for i=1, visRows do
        -- Populate the data with the view specific populate call
        BlackMarketMod.populateRow( viewType, i, auctionList[scrollIndex + i] )
    end
end

--=============================================================================
function BlackMarketMod.updateScroll( scrollBar, visRows, totalRows )
    scrollBar:setPageSize( visRows )
    scrollBar:setStepSize( 1 )
    scrollBar:setDocumentSize( totalRows )
    scrollBar:setOverlapSize( 1 )

    if scrollBar:getPageSize() > scrollBar:getDocumentSize() then
        scrollBar:setPageSize( scrollBar:getDocumentSize() )
    end

    scrollBar:setScrollPosition(0)
end

--=============================================================================
function BlackMarketMod.onCashUpdate( this, cash )
    BlackMarketMod.updateCash()
end

--=============================================================================
function BlackMarketMod.populateRow( viewType, rowId, auctionId )
    if not auctionId then
        auctionId = 0
    end

    local viewData = BlackMarketMod.viewData[viewType]
    local rowPrefix = 'BlackMarket_'..viewData.format..rowId
    local rowWin = _G[rowPrefix..'MainContainer']
    if not rowWin then
        return
    end

    local itemInfo
    if viewType == BlackMarketMod.VIEW_WATCHED then
        itemInfo = BlackMarketMod.getWatchedItemInfo( auctionId )
    else
        itemInfo = BlackMarketMod.getAuctionItemInfo( auctionId )
    end

    -- Keep a lookup of AuctionId to Row Id
    if auctionId > 0 then
        viewData.auctionToRow[auctionId] = rowId
    end

    -- Maintain row highlights
    local rowHighlight = _G[rowPrefix..'Highlight']
    if rowHighlight then
        rowHighlight:setVisible( auctionId > 0 and viewData.selectedAuctionId == auctionId )
    end

    if itemInfo.itemId then
        rowWin:show()
        rowWin:setID( auctionId )

        -- Item Icon
        local iconImage = _G[rowPrefix..'Icon']
        if iconImage then
            iconImage:setProperty( 'Image', itemInfo.icon )
        end

        -- Item Quantity
        local qtyText = _G[rowPrefix..'QtyText']
        if qtyText then
            if itemInfo.stackSize > 1 then
                qtyText:setText( itemInfo.stackSize )
            else
                qtyText:setText( '' )
            end
        end

        -- Item Name
        local nameText = _G[rowPrefix..'NameText']
        if nameText then
            nameText:setText( itemInfo.name )
        end

        -- Tech Competancy
        local tcText = _G[rowPrefix..'TCText']
        if tcText then
            tcText:setText( itemInfo.techCompentancy )
        end

        -- Time Remaining (U10: the stock code switched on techCompentancy)
        local timeImage = _G[rowPrefix..'TimerImage']
        if timeImage then
            if itemInfo.timeLeft == UIAuctionTime.VeryShort or itemInfo.timeLeft == UIAuctionTime.Short then
                timeImage:setProperty( 'Image', 'set:CoreWidgets2_2 image:MeasurementBars_Low' )
            elseif itemInfo.timeLeft == UIAuctionTime.Medium then
                timeImage:setProperty( 'Image', 'set:CoreWidgets2_2 image:MeasurementBars_Medium' )
            else
                timeImage:setProperty( 'Image', 'set:CoreWidgets2_2 image:MeasurementBars_High' )
            end
        end

        -- Current Bid (the lowest bid accepted while nobody has bid)
        local curBidText = _G[rowPrefix..'CurrentBidText']
        if curBidText then
            curBidText:setText( BlackMarketMod.bidColumnText( itemInfo ) )
        end

        -- Buyout ("Bid only" when the auction has none)
        local buyoutText = _G[rowPrefix..'BuyoutText']
        if buyoutText then
            buyoutText:setText( BlackMarketMod.buyoutColumnText( itemInfo ) )
        end

        -- Top Bidder
        local bidderText = _G[rowPrefix..'BidderText']
        if bidderText then
            bidderText:setText( itemInfo.bidderName )
        end

        -- Seller
        local sellerText = _G[rowPrefix..'SellerText']
        if sellerText then
            sellerText:setText( itemInfo.sellerName )
        end

        -- Bid Count: not on the wire, so left blank
        local bidCountText = _G[rowPrefix..'BidCountText']
        if bidCountText then
            bidCountText:setText( '' )
        end
    else
        rowWin:setID( 0 )
        rowWin:hide()
    end
end

-- The Bid column: the standing bid, or, before anyone bids, the lowest bid
-- the server accepts (the starting price) as "Min <price>". A 0 read as
-- "free" or "no price" on the first live run.
function BlackMarketMod.bidColumnText( itemInfo )
    if not itemInfo.auctionId then
        return ''
    end
    if (tonumber(itemInfo.currentBid) or 0) > 0 then
        return tostring(itemInfo.currentBid)
    end
    if (tonumber(itemInfo.nextBidPrice) or 0) > 0 then
        return 'Min '..tostring(itemInfo.nextBidPrice)
    end
    return ''
end

-- The Buyout column: the price, or "Bid only" for an auction without one.
function BlackMarketMod.buyoutColumnText( itemInfo )
    if (tonumber(itemInfo.buyoutPrice) or 0) > 0 then
        return tostring(itemInfo.buyoutPrice)
    end
    if itemInfo.auctionId then
        return 'Bid only'
    end
    return ''
end

-- A Watched row shows an item definition, not an auction.
function BlackMarketMod.getWatchedItemInfo( itemDefId )
    if itemDefId <= 0 then
        return {}
    end
    local def = BlackMarketMod.itemDefInfo( itemDefId )
    local info = {}
    info.itemId = itemDefId
    info.name = def.Name or ('Item '..tostring(itemDefId))
    info.icon = def.Icon or ''
    info.techCompentancy = BlackMarketMod.techCompetency( itemDefId )
    info.timeLeft = UIAuctionTime.VeryLong
    info.stackSize = 1
    info.currentBid = ''
    info.buyoutPrice = 0
    info.bidderName = ''
    info.sellerName = ''
    return info
end

--=============================================================================
function BlackMarketMod.initRows( viewType, callback )
    local maxVis = BlackMarketMod.viewData[viewType].maxVis
    local viewFormat = BlackMarketMod.viewData[viewType].format
    for i=1, maxVis do
        local row = _G['BlackMarket_'..viewFormat..i..'MainContainer']
        if row then
            row:setID(i)
            row:subscribe( row.EventMouseButtonDown, callback )
        end
    end
end


--=============================================================================
function BlackMarketMod.selectCreateDuration( duration )
    BlackMarketMod.creationDuration = duration
    BlackMarket_DurationShortHighlightImage:setVisible( duration <= UIAuctionTime.Medium )
    BlackMarket_DurationMediumHighlightImage:setVisible( duration == UIAuctionTime.Long )
    BlackMarket_DurationLongHighlightImage:setVisible( duration == UIAuctionTime.VeryLong )
end

--=============================================================================
function BlackMarketMod.onTimeHighlightMousedown( this, window )
    BlackMarketMod.selectCreateDuration( window:getID() )
end

--=============================================================================
function BlackMarketMod.onTabClicked( this, window )
    BlackMarketMod.showTab( window:getID() )
end

function BlackMarketMod.showTab( tabIndex )
    for i=1, 4 do
        local tab = _G['BlackMarket_Tab'..i]
        if tab then
            tab:setVisible( tabIndex == i )
        end
        local tabButton = _G['BlackMarket_TabButton'..i]
        if tabButton then
            BlackMarketMod.refreshTabButton( tabButton, tabIndex == i )
        end
    end
    BlackMarketMod.currentTab = tabIndex

    BlackMarketMod.setStatus('')

    if not BlackMarketMod.hasPatch() then
        BlackMarketMod.setStatus( BlackMarketMod.PATCH_MISSING_TEXT, true )
        return
    end

    -- Refresh the tab's list each time it opens (U9)
    if tabIndex == 2 then
        BlackMarketMod.requestView( UIAuctionView.MyAuctions )
    elseif tabIndex == 3 then
        BlackMarketMod.requestView( UIAuctionView.MyBids )
    elseif tabIndex == 4 then
        -- Decision D4: the watch list is deferred.
        BlackMarketMod.setStatus( BlackMarketMod.ERROR_TEXT[14] )
    end
end

--=============================================================================
function BlackMarketMod.refreshTabButton( tabButton, enabled )
    if enabled then
        tabButton:setProperty( 'NormalImage', 'set:CoreWidgets_2 image:Tab_Selected' )
        tabButton:setProperty( 'HoverImage', 'set:CoreWidgets_2 image:Tab_Selected' )
    else
        tabButton:setProperty( 'NormalImage', 'set:CoreWidgets_2 image:Tab_Idle' )
        tabButton:setProperty( 'HoverImage', 'set:CoreWidgets_2 image:Tab_Idle' )
    end
end

--=============================================================================
function BlackMarketMod.onSearchComboAccepted( this, window )
    local entry = window:getSelectedItem()
    local colorRect = entry:getTextColours()
    local colorString = CEGUI.PropertyHelper:colourToString(colorRect.top_left)
    window:getEditbox():setProperty('NormalTextColour', colorString )
end

--=============================================================================
function BlackMarketMod.updateSearchNavButtons()
    local viewData = BlackMarketMod.viewData[UIAuctionView.SearchResults]
    local visCount = BlackMarketMod.getAuctionVisibleCount(UIAuctionView.SearchResults)
    local totalCount = BlackMarketMod.getAuctionTotalCount(UIAuctionView.SearchResults)
    local ready = BlackMarketMod.searchSuspended == false and BlackMarketMod.pending.search == nil

    BlackMarket_SearchButton:setEnabled( ready )
    BlackMarket_NextSearchPageButton:setEnabled( ready and totalCount > 0 and viewData.offset + visCount < totalCount )
    BlackMarket_PrevSearchPageButton:setEnabled( ready and totalCount > 0 and viewData.offset > 0 )
end

--=============================================================================
function BlackMarketMod.onSearchNextClicked( this, window )
    local viewData = BlackMarketMod.viewData[UIAuctionView.SearchResults]
    local searchFilter = viewData.searchFilter or BlackMarketMod.createSearchFilter()
    local auctionList = BlackMarketMod.getAuctionViewItems(UIAuctionView.SearchResults)

    -- Find the max auctionId currently being displayed
    local maxId = 0
    for key, auctionId in ipairs(auctionList) do
        if auctionId > maxId then
            maxId = auctionId
        end
    end

    searchFilter.auctionId = maxId
    searchFilter.forward = true
    viewData.searchFilter = searchFilter
    viewData.request = { page = viewData.currentPage + 1, forward = true, fromOffset = viewData.offset, fromCount = #auctionList }

    BlackMarketMod.performSearch( searchFilter )
end

--=============================================================================
function BlackMarketMod.onSearchPrevClicked( this, window )
    local viewData = BlackMarketMod.viewData[UIAuctionView.SearchResults]
    local searchFilter = viewData.searchFilter or BlackMarketMod.createSearchFilter()
    local auctionList = BlackMarketMod.getAuctionViewItems(UIAuctionView.SearchResults)

    -- Find the min auctionId currently being displayed
    local minId = math.huge
    for key, auctionId in ipairs(auctionList) do
        if auctionId < minId then
            minId = auctionId
        end
    end

    if minId == math.huge then
        searchFilter.auctionId = 0
        searchFilter.forward = true
        viewData.request = { page = 0, reset = true }
    else
        searchFilter.auctionId = minId
        searchFilter.forward = false
        viewData.request = { page = math.max(viewData.currentPage - 1, 0), forward = false, fromOffset = viewData.offset, fromCount = #auctionList }
    end
    viewData.searchFilter = searchFilter

    BlackMarketMod.performSearch( searchFilter )
end

--=============================================================================
function BlackMarketMod.onCreateTextChanged( this, window )
    BlackMarketMod.updateNewAuctionReady()
end

--=============================================================================
function BlackMarketMod.updateNewAuctionReady()
    local result = true
    local itemId = BlackMarket_NewAuctionImage:getID()

    if itemId <= 0 then
        result = false
    end

    local bid = BlackMarketMod.toInt(BlackMarket_NewAuctionBidText:getText()) or 0
    local buyoutText = BlackMarket_NewAuctionBuyoutText:getText() or ''
    local buyout = BlackMarketMod.toInt(buyoutText) or 0
    local buyoutBad = (buyoutText ~= '' and not BlackMarketMod.toInt(buyoutText)) or buyout < 0

    if itemId > 0 and bid <= 0 then
        BlackMarket_NewAuctionBidText:setProperty( 'NormalColour', 'FFFF0000' )
        result = false
    else
        BlackMarket_NewAuctionBidText:setProperty( 'NormalColour', 'FFFFFFFF' )
    end

    if buyoutBad or (buyout > 0 and buyout < bid) then
        -- Invalid buyout, too low
        BlackMarket_NewAuctionBuyoutText:setProperty( 'NormalColour', 'FFFF0000' )
        result = false
    else
        BlackMarket_NewAuctionBuyoutText:setProperty( 'NormalColour', 'FFFFFFFF' )
    end

    BlackMarket_CreateButton:setEnabled( result and BlackMarketMod.pending.create == nil )
    return result
end

--=============================================================================
function BlackMarketMod.onCreateKeyPress( this, window, codePoint, scanCode, sysKeys )
    if scanCode == CEGUI.Key.Tab then
        if BlackMarket_NewAuctionBidText:isActive() then
           BlackMarket_NewAuctionBuyoutText:activate()
        else
           BlackMarket_NewAuctionBidText:activate()
        end

        return true
    end

    return false
end

--=============================================================================
function BlackMarketMod.onCreateTextActivated( this, window )
    window:setSelection(0, 99)
end

--=============================================================================
function BlackMarketMod.onCreateTextDeactivated( this, window )
    window:setSelection(0, 0)
end

--=============================================================================
-- Guard every entry point: CEGUI and the DLL call these by name.
BlackMarketMod.GUARDED_HANDLERS = {
    'onBMOpen', 'onBMError', 'onCloseClicked', 'onWindowShow', 'onNewAuctionDragReceived',
    'onCreateClicked', 'onCancelClicked', 'onBidClicked', 'onBuyoutClicked', 'onSearchClicked',
    'onSearchRowMouseDown', 'onMyAuctionsRowMouseDown', 'onMyBidsRowMouseDown', 'onScrolled',
    'onPreRender', 'onBMViewUpdate', 'onCashUpdate', 'onTimeHighlightMousedown', 'onTabClicked',
    'onSearchComboAccepted', 'onSearchNextClicked', 'onSearchPrevClicked', 'onCreateTextChanged',
    'onCreateKeyPress', 'onCreateTextActivated', 'onCreateTextDeactivated',
}
for i, name in ipairs(BlackMarketMod.GUARDED_HANDLERS) do
    BlackMarketMod[name] = BlackMarketMod.guard( 'BlackMarketMod.'..name, BlackMarketMod[name] )
end
for i, name in ipairs({ 'onOpen', 'onError', 'onAuctions', 'onAuctionRemove', 'onAuctionUpdate', 'onWatchedItems' }) do
    CimmeriaBM[name] = BlackMarketMod.guard( 'CimmeriaBM.'..name, CimmeriaBM[name] )
end

--=============================================================================

-- Register for Events
BlackMarketWin:subscribe( Events.BMOpen, 'BlackMarketMod.onBMOpen')
BlackMarketWin:subscribe( Events.BMError, 'BlackMarketMod.onBMError' )
BlackMarketWin:subscribe( Events.BMViewUpdate, 'BlackMarketMod.onBMViewUpdate' )
BlackMarketWin:subscribe( Events.InventoryUpdateCash, 'BlackMarketMod.onCashUpdate' )

BlackMarketWin:subscribe( BlackMarketWin.EventCloseClicked, 'BlackMarketMod.onCloseClicked')
BlackMarketWin:subscribe( BlackMarketWin.EventShown, 'BlackMarketMod.onWindowShow' )
BlackMarket_Tab2:subscribe( BlackMarket_Tab2.EventDragDropItemDropped, 'BlackMarketMod.onNewAuctionDragReceived')

BlackMarket_DurationShortImage:setID(UIAuctionTime.Medium)
BlackMarket_DurationMediumImage:setID(UIAuctionTime.Long)
BlackMarket_DurationLongImage:setID(UIAuctionTime.VeryLong)

BlackMarket_DurationShortImage:subscribe( BlackMarket_DurationShortImage.EventMouseButtonDown, 'BlackMarketMod.onTimeHighlightMousedown' )
BlackMarket_DurationMediumImage:subscribe( BlackMarket_DurationMediumImage.EventMouseButtonDown, 'BlackMarketMod.onTimeHighlightMousedown' )
BlackMarket_DurationLongImage:subscribe( BlackMarket_DurationLongImage.EventMouseButtonDown, 'BlackMarketMod.onTimeHighlightMousedown' )

BlackMarket_SearchScroll:setID( UIAuctionView.SearchResults )
BlackMarket_MyAuctionsScroll:setID( UIAuctionView.MyAuctions )
BlackMarket_MyBidsScroll:setID( UIAuctionView.MyBids )
BlackMarket_WatchedScroll:setID( BlackMarketMod.VIEW_WATCHED )

BlackMarket_SearchButton:subscribe( BlackMarket_SearchButton.EventClicked, 'BlackMarketMod.onSearchClicked' )
BlackMarket_SearchBidButton:subscribe( BlackMarket_SearchBidButton.EventClicked, 'BlackMarketMod.onBidClicked' )
BlackMarket_SearchBuyoutButton:subscribe( BlackMarket_SearchBuyoutButton.EventClicked, 'BlackMarketMod.onBuyoutClicked' )
BlackMarket_CreateButton:subscribe( BlackMarket_CreateButton.EventClicked, 'BlackMarketMod.onCreateClicked' )
BlackMarket_CancelButton:subscribe( BlackMarket_CancelButton.EventClicked, 'BlackMarketMod.onCancelClicked' )
BlackMarket_NextSearchPageButton:subscribe( BlackMarket_NextSearchPageButton.EventClicked, 'BlackMarketMod.onSearchNextClicked' )
BlackMarket_PrevSearchPageButton:subscribe( BlackMarket_PrevSearchPageButton.EventClicked, 'BlackMarketMod.onSearchPrevClicked' )

BlackMarket_NewAuctionBidText:subscribe( BlackMarket_NewAuctionBidText.EventTextChanged, 'BlackMarketMod.onCreateTextChanged' )
BlackMarket_NewAuctionBuyoutText:subscribe( BlackMarket_NewAuctionBuyoutText.EventTextChanged, 'BlackMarketMod.onCreateTextChanged' )

BlackMarket_NewAuctionBidText:subscribe(  BlackMarket_NewAuctionBidText.EventActivated, 'BlackMarketMod.onCreateTextActivated' )
BlackMarket_NewAuctionBuyoutText:subscribe(  BlackMarket_NewAuctionBuyoutText.EventActivated, 'BlackMarketMod.onCreateTextActivated' )

BlackMarket_NewAuctionBidText:subscribe(  BlackMarket_NewAuctionBidText.EventDeactivated, 'BlackMarketMod.onCreateTextDeactivated' )
BlackMarket_NewAuctionBuyoutText:subscribe(  BlackMarket_NewAuctionBuyoutText.EventDeactivated, 'BlackMarketMod.onCreateTextDeactivated' )


BlackMarket_SearchScroll:subscribe( BlackMarket_SearchScroll.EventScrollPositionChanged, 'BlackMarketMod.onScrolled')
BlackMarket_MyAuctionsScroll:subscribe( BlackMarket_MyAuctionsScroll.EventScrollPositionChanged, 'BlackMarketMod.onScrolled')
BlackMarket_MyBidsScroll:subscribe( BlackMarket_MyBidsScroll.EventScrollPositionChanged, 'BlackMarketMod.onScrolled')
BlackMarket_WatchedScroll:subscribe( BlackMarket_WatchedScroll.EventScrollPositionChanged, 'BlackMarketMod.onScrolled')

BlackMarket_SearchQualityCombo:subscribe( BlackMarket_SearchQualityCombo.EventListSelectionAccepted, 'BlackMarketMod.onSearchComboAccepted' )

BlackMarket_NewAuctionBidText:subscribe( BlackMarket_NewAuctionBidText.EventKeyDown, 'BlackMarketMod.onCreateKeyPress' )
BlackMarket_NewAuctionBuyoutText:subscribe( BlackMarket_NewAuctionBuyoutText.EventKeyDown, 'BlackMarketMod.onCreateKeyPress' )


BlackMarket_TabButton1:setID(1)
BlackMarket_TabButton2:setID(2)
BlackMarket_TabButton3:setID(3)
BlackMarket_TabButton4:setID(4)
BlackMarket_Tab1:setID(1)
BlackMarket_Tab2:setID(2)
BlackMarket_Tab3:setID(3)
BlackMarket_Tab4:setID(4)

BlackMarket_TabButton1:subscribe( BlackMarket_TabButton1.EventClicked, 'BlackMarketMod.onTabClicked' )
BlackMarket_TabButton2:subscribe( BlackMarket_TabButton2.EventClicked, 'BlackMarketMod.onTabClicked' )
BlackMarket_TabButton3:subscribe( BlackMarket_TabButton3.EventClicked, 'BlackMarketMod.onTabClicked' )
BlackMarket_TabButton4:subscribe( BlackMarket_TabButton4.EventClicked, 'BlackMarketMod.onTabClicked' )

-- Set up the data for each view.  This drives how the different item lists update
BlackMarketMod.viewData = {}

BlackMarketMod.viewData[UIAuctionView.SearchResults] = {}
BlackMarketMod.viewData[UIAuctionView.SearchResults].refreshCall = BlackMarketMod.refreshSearchView
--BlackMarketMod.viewData[UIAuctionView.SearchResults].populateCall = BlackMarketMod.populateSearchRow
BlackMarketMod.viewData[UIAuctionView.SearchResults].selectionCall = BlackMarketMod.searchSelectionUpdate
BlackMarketMod.viewData[UIAuctionView.SearchResults].maxVis = BlackMarketMod.MAX_SEARCH_ROWS
BlackMarketMod.viewData[UIAuctionView.SearchResults].scrollBar = BlackMarket_SearchScroll
BlackMarketMod.viewData[UIAuctionView.SearchResults].format = "Search"
BlackMarketMod.viewData[UIAuctionView.SearchResults].auctionToRow = {}
BlackMarketMod.viewData[UIAuctionView.SearchResults].selectedAuctionId = 0
BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage = 0
BlackMarketMod.viewData[UIAuctionView.SearchResults].offset = 0

BlackMarketMod.viewData[UIAuctionView.MyAuctions] = {}
BlackMarketMod.viewData[UIAuctionView.MyAuctions].refreshCall = BlackMarketMod.refreshMyAuctionsView
--BlackMarketMod.viewData[UIAuctionView.MyAuctions].populateCall = BlackMarketMod.populateMyAuctionsRow
BlackMarketMod.viewData[UIAuctionView.MyAuctions].selectionCall = BlackMarketMod.myAuctionsSelectionUpdate
BlackMarketMod.viewData[UIAuctionView.MyAuctions].maxVis = BlackMarketMod.MAX_AUCTION_ROWS
BlackMarketMod.viewData[UIAuctionView.MyAuctions].scrollBar = BlackMarket_MyAuctionsScroll
-- U7: the layout's row prefix is MyAuction, not MyAuctions
BlackMarketMod.viewData[UIAuctionView.MyAuctions].format = "MyAuction"
BlackMarketMod.viewData[UIAuctionView.MyAuctions].auctionToRow = {}
BlackMarketMod.viewData[UIAuctionView.MyAuctions].selectedAuctionId = 0
BlackMarketMod.viewData[UIAuctionView.MyAuctions].currentPage = 0
BlackMarketMod.viewData[UIAuctionView.MyAuctions].offset = 0

BlackMarketMod.viewData[UIAuctionView.MyBids] = {}
BlackMarketMod.viewData[UIAuctionView.MyBids].refreshCall = BlackMarketMod.refreshMyBidsView
--BlackMarketMod.viewData[UIAuctionView.MyBids].populateCall = BlackMarketMod.populateMyBidsRow
BlackMarketMod.viewData[UIAuctionView.MyBids].selectionCall = BlackMarketMod.myBidsSelectionUpdate
BlackMarketMod.viewData[UIAuctionView.MyBids].maxVis = BlackMarketMod.MAX_BID_ROWS
BlackMarketMod.viewData[UIAuctionView.MyBids].scrollBar = BlackMarket_MyBidsScroll
BlackMarketMod.viewData[UIAuctionView.MyBids].format = "MyBids"
BlackMarketMod.viewData[UIAuctionView.MyBids].auctionToRow = {}
BlackMarketMod.viewData[UIAuctionView.MyBids].selectedAuctionId = 0
BlackMarketMod.viewData[UIAuctionView.MyBids].currentPage = 0
BlackMarketMod.viewData[UIAuctionView.MyBids].offset = 0

BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED] = {}
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].refreshCall = BlackMarketMod.refreshWatchedView
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].selectionCall = BlackMarketMod.watchedSelectionUpdate
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].maxVis = BlackMarketMod.MAX_WATCHED_ROWS
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].scrollBar = BlackMarket_WatchedScroll
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].format = "Watched"
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].auctionToRow = {}
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].selectedAuctionId = 0
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].currentPage = 0
BlackMarketMod.viewData[BlackMarketMod.VIEW_WATCHED].offset = 0

BlackMarketMod.initRows( UIAuctionView.SearchResults, 'BlackMarketMod.onSearchRowMouseDown' )
BlackMarketMod.initRows( UIAuctionView.MyAuctions, 'BlackMarketMod.onMyAuctionsRowMouseDown' )
BlackMarketMod.initRows( UIAuctionView.MyBids, 'BlackMarketMod.onMyBidsRowMouseDown' )

BlackMarketMod.resetStore()
BlackMarketMod.setupWindow()
BlackMarketMod.resetAllViews()
BlackMarketWin:hide()

do
    local native = BlackMarketMod.native()
    if native then
        BlackMarketMod.log( 'overlay loaded, CimmeriaBMNative version '..tostring(native.version) )
    else
        BlackMarketMod.log( 'overlay loaded, CimmeriaBMNative not present yet' )
    end
end
