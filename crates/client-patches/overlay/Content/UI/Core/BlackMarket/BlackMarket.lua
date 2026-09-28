-- Global table to avoid name clashes
BlackMarketMod = {}

BlackMarketMod.MAX_SEARCH_ROWS = 8
BlackMarketMod.MAX_AUCTION_ROWS = 8
BlackMarketMod.MAX_BID_ROWS = 8
BlackMarketMod.SEARCH_DELAY = 4

BlackMarketMod.selectedSearchAuctionId = 0

BlackMarketMod.savedItemPrices = {}
BlackMarketMod.creationDuration = 5


--=============================================================================
function BlackMarketMod.onBMOpen()
    BlackMarketWin:show()
end

--=============================================================================
function BlackMarketMod.onBMError(this, errorText )
    BlackMarket_ErrorText:setText( errorText )
end

--=============================================================================
function BlackMarketMod.onCloseClicked( this, window )
    BlackMarketWin:hide()
end

--=============================================================================
function BlackMarketMod.onWindowShow( this, window )
    BlackMarketMod.setupWindow()
end

--=============================================================================
function BlackMarketMod.setupWindow()
    BlackMarketMod.updateCash()
    BlackMarketMod.refreshQualityCombo()    
    BlackMarketMod.clearAuctionCreateItem()
    BlackMarketMod.selectCreateDuration( UIAuctionTime.VeryLong )
    
--    BlackMarketMod.clearSearchFilters()
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
    if auctionId > 0 then
        local itemInfo = getAuctionItemInfo( auctionId )
        if itemInfo.auctionId then
            local myCash = getCash()        
            BlackMarket_SearchBidButton:setEnabled( myCash >= itemInfo.nextBidPrice )
            BlackMarket_SearchBuyoutButton:setEnabled( myCash >= itemInfo.buyoutPrice )
        end
    else
        BlackMarket_SearchBidButton:setEnabled( false )
        BlackMarket_SearchBuyoutButton:setEnabled( false )    
    end
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
    if itemId > 0 then
        local itemDefId = getItemDef(itemId)
        local bid = tonumber(BlackMarket_NewAuctionBidText:getText()) or 0
        local buyout = tonumber(BlackMarket_NewAuctionBuyoutText:getText()) or 0
        
        -- Save the desired prices for this item so we can recall it if they put up more of the same
        BlackMarketMod.savedItemPrices[itemDefId] = {}
        BlackMarketMod.savedItemPrices[itemDefId].bid = bid
        BlackMarketMod.savedItemPrices[itemDefId].buyout = buyout
        
        createAuction( itemId, bid, buyout, BlackMarketMod.creationDuration )
    end
end

--=============================================================================
function BlackMarketMod.onCancelClicked( this, window )
    local auctionId = BlackMarketMod.viewData[UIAuctionView.MyAuctions].selectedAuctionId
    if auctionId > 0 then
        cancelAuction( auctionId )
    end    
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
    searchFilter.itemName = BlackMarket_SearchItemText:getText() or ''
    searchFilter.minTC = tonumber(BlackMarket_MinTC:getText()) or 0
    searchFilter.maxTC = tonumber(BlackMarket_MaxTC:getText()) or 0
    searchFilter.quality = 0
    
    if BlackMarket_SearchQualityCombo:getSelectedItem() then
        searchFilter.quality = BlackMarket_SearchQualityCombo:getSelectedItem():getID()
    end
    
    searchFilter.filterFlags = 0

    -- Reset to the first page
    BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage = 0
    BlackMarketMod.viewData[UIAuctionView.SearchResults].searchFilter = searchFilter
    
    BlackMarketMod.performSearch( searchFilter )
end

--=============================================================================
function BlackMarketMod.onSearchRowMouseDown( this, window )
    BlackMarketMod.selectRow( UIAuctionView.SearchResults, this:getID() )
end

--=============================================================================
function BlackMarketMod.onMyAuctionsRowMouseDown( this, window )
    BlackMarketMod.selectRow( UIAuctionView.MyAuctions, this:getID() )
end

--=============================================================================
function BlackMarketMod.onMyBidsRowMouseDown( this, window )
    BlackMarketMod.selectRow( UIAuctionView.MyBids, this:getID() )
end

--=============================================================================
function BlackMarketMod.onScrolled( this, window )
    BlackMarketMod.refreshView( window:getID(), window:getScrollPosition() )
end

--=============================================================================
function BlackMarketMod.selectRow( viewType, auctionId )

    local itemInfo = getAuctionItemInfo( auctionId )
    if itemInfo then
        local currentSelectedAuctionId = BlackMarketMod.viewData[viewType].selectedAuctionId
        if currentSelectedAuctionId ~= auctionId then
            -- Deselect the old row
            if currentSelectedAuctionId > 0 then
                local rowId = BlackMarketMod.viewData[viewType].auctionToRow[currentSelectedAuctionId]
                if rowId then
                    local rowHighlight = _G['BlackMarket_'..BlackMarketMod.viewData[viewType].format..rowId..'Highlight']
                    if rowHighlight then
                        rowHighlight:hide()
                    end
                end
                                    
                BlackMarketMod.viewData[viewType].selectedAuctionId = 0
            end
                
            -- Select the new row
            if auctionId > 0 then
                local rowId = BlackMarketMod.auctionToRow[ auctionId ]
                local rowHighlight = _G['BlackMarket_'..BlackMarketMod.viewData[viewType].format..rowId..'Highlight']
                if rowHighlight then
                    rowHighlight:show()
                end
                
                BlackMarketMod.viewData[viewType].selectedAuctionId = auctionId
                BlackMarketMod.viewData[viewType].selectionCall( itemInfo )
                
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
function BlackMarketMod.refreshSearchView()
    local page = BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage
    local visCount = getAuctionVisibleCount(viewType)
    local totalCount = getAuctionTotalCount(viewType)
    local totalPages = totalCount / visCount
    BlackMarket_SearchPageText:setText( page..'/'..totalPages )
    
    BlackMarketMod.updateSearchNavButtons()    
end

--=============================================================================
function BlackMarketMod.refreshMyAuctionsView()

end

--=============================================================================
function BlackMarketMod.refreshMyBidsView()

end

--=============================================================================
function BlackMarketMod.performSearch( searchFilter )
    -- Save the time they began a search
    BlackMarketMod.searchDelayTime = worldGetDeltaSeconds()
    BlackMarketWin:subscribe( Events.PreRender, 'BlackMarketMod.onPreRender' )
    
    -- Flag the search buttons as suspended
    BlackMarketMod.searchSuspended = true
    BlackMarketMod.updateSearchNavButtons()
    
    searchAuctions( searchFilter.sortId, searchFilter.auctionId, searchFilter.forward, searchFilter.sellerName, 
    searchFilter.bidderName, searchFilter.itemName, searchFilter.minTC, searchFilter.maxTC, searchFilter.quality, searchFilter.filterFlags )
end

--=============================================================================
function BlackMarketMod.onPreRender( window, view )
    -- If enough time has passed, enabled the search buttons again
    BlackMarketMod.searchSuspended = (worldGetDeltaSeconds() - BlackMarketMod.searchDelayTime) < BlackMarketMod.SEARCH_DELAY
    if BlackMarketMod.searchSuspended == false then
        BlackMarketMod.updateSearchNavButtons()
        BlackMarketWin:unsubscribe( Events.PreRender )
    end
end

--=============================================================================
function BlackMarketMod.onBMViewUpdate( this, viewType )
    BlackMarketMod.resetView( viewType )
end

--=============================================================================
function BlackMarketMod.resetView( viewType )
    local auctionList = getAuctionViewItems(viewType)
    local scrollBar = BlackMarketMod.viewData[viewType].scrollBar
    local visRows = BlackMarketMod.viewData[viewType].maxVis
    
    BlackMarketMod.updateScroll( scrollBar, visRows, #auctionList )
    local scrollIndex = scrollBar:getScrollPosition()
    
    BlackMarketMod.refreshView( viewType, scrollIndex )
end

--=============================================================================
function BlackMarketMod.refreshView( viewType, scrollIndex )
    local auctionList = getAuctionViewItems(viewType)
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
    
    local itemInfo = getAuctionItemInfo( auctionId )
    local rowPrefix = 'BlackMarket_'..BlackMarketMod.viewData[viewType].format..rowId
    local rowWin = _G[rowPrefix..'MainContainer']
    
    -- Keep a lookup of AuctionId to Row Id
    BlackMarketMod.viewData[viewType].auctionToRow[auctionId] = rowId

    -- Maintain row highlights    
    local rowHighlight = _G[rowPrefix..'Highlight']
    if rowHighlight then
        rowHighlight:setVisible( BlackMarketMod.viewData[viewType].selectedAuctionId == auctionId )
    end
    
    if itemInfo.itemId and rowWin then
        rowWin:show()
        rowWin:setID( auctionId )
    
        -- Item Icon
        local iconImage = _G[rowPrefix..'Icon']
        if iconImage then
            iconImage:setProperty( 'Image', itemInfo.icon )
        end

        -- Item Quantity
        if itemInfo.stackSize > 1 then
            local qtyText = _G[rowPrefix..'QtyText']
            if qtyText then
                qtyText:setText( itemInfo.stackSize )
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
            
        -- Time Remaining
        local timeImage = _G[rowPrefix..'TimerImage']
        if timeImage then
            if itemInfo.techCompentancy == UIAuctionTime.VeryShort or itemInfo.techCompentancy == UIAuctionTime.Short then
                timeImage:setProperty( 'Image', 'set:CoreWidgets2_2 image:MeasurementBars_Low' )
            elseif itemInfo.techCompentancy == UIAuctionTime.Medium then
                timeImage:setProperty( 'Image', 'set:CoreWidgets2_2 image:MeasurementBars_Medium' )
            else
                timeImage:setProperty( 'Image', 'set:CoreWidgets2_2 image:MeasurementBars_High' )
            end
        end
        
        -- Current Bid
        local curBidText = _G[rowPrefix..'CurrentBidText']
        if curBidText then
            curBidText:setText( itemInfo.currentBid )
        end
              
        -- Buyout
        local buyoutText = _G[rowPrefix..'BuyoutText']
        if buyoutText then
            buyoutText:setText( itemInfo.buyoutPrice )
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

        -- Bid Count
        local bidCountText = _G[rowPrefix..'BidCountText']
        if bidCountText then
            bidCountText:setText( itemInfo.bidCount )
        end        
    else
        rowWin:hide()
    end
end

--=============================================================================
function BlackMarketMod.initRows( viewType, callback )
    maxVis = BlackMarketMod.viewData[UIAuctionView.MyBids].maxVis
    viewFormat = BlackMarketMod.viewData[UIAuctionView.MyBids].format
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
    local tabIndex = window:getID()

    BlackMarket_Tab1:setVisible( tabIndex == BlackMarket_Tab1:getID() )
    BlackMarket_Tab2:setVisible( tabIndex == BlackMarket_Tab2:getID() )
    BlackMarket_Tab3:setVisible( tabIndex == BlackMarket_Tab3:getID() )
    
    BlackMarket_ErrorText:setText('')
    
    BlackMarketMod.refreshTabButton( BlackMarket_TabButton1, tabIndex == BlackMarket_Tab1:getID() )
    BlackMarketMod.refreshTabButton( BlackMarket_TabButton2, tabIndex == BlackMarket_Tab2:getID() )
    BlackMarketMod.refreshTabButton( BlackMarket_TabButton3, tabIndex == BlackMarket_Tab3:getID() )
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
    local currentPage = BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage
    local visCount = getAuctionVisibleCount(UIAuctionView.SearchResults)
    local totalCount = getAuctionTotalCount(UIAuctionView.SearchResults)

    BlackMarket_ErrorText:setText('')

    BlackMarket_SearchButton:setEnabled( BlackMarketMod.searchSuspended == false )
    BlackMarket_NextSearchPageButton:setEnabled( BlackMarketMod.searchSuspended == false and totalCount > 0 and currentPage * visCount >= totalCount)
    BlackMarket_PrevSearchPageButton:setEnabled( BlackMarketMod.searchSuspended == false and totalCount > 0 and currentPage > 0 )
end

--=============================================================================
function BlackMarketMod.onSearchNextClicked( this, window )
    local searchFilter = BlackMarketMod.viewData[UIAuctionView.SearchResults].searchFilter
    local auctionList = getAuctionViewItems(viewType)
    
    -- Find the max auctionId currently being displayed
    local maxId = 0
    for key, auctionId in ipairs(auctionList) do
        if auctionId > maxId then
            maxId = auctionId
        end
    end
    
    searchFilter.auctionId = maxId
    searchFilter.forward = true
    
    BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage = BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage + 1
    BlackMarketMod.performSearch( searchFilter )
    BlackMarketMod.updateSearchNavButtons()
end

--=============================================================================
function BlackMarketMod.onSearchPrevClicked( this, window )
    local searchFilter = BlackMarketMod.viewData[UIAuctionView.SearchResults].searchFilter
    local auctionList = getAuctionViewItems(viewType)
    
    -- Find the min auctionId currently being displayed
    local minId = math.huge
    for key, auctionId in ipairs(auctionList) do
        if auctionId < minId then
            minId = auctionId
        end
    end
    
    searchFilter.auctionId = minId
    searchFilter.forward = false
    
    BlackMarketMod.viewData[UIAuctionView.SearchResults].currentPage = currentPage - 1
    BlackMarketMod.performSearch( searchFilter )
    BlackMarketMod.updateSearchNavButtons()        
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
    
    local bid = tonumber(BlackMarket_NewAuctionBidText:getText()) or 0
    local buyout = tonumber(BlackMarket_NewAuctionBuyoutText:getText()) or 0

    if itemId > 0 and bid <= 0 then
        BlackMarket_NewAuctionBidText:setProperty( 'NormalColour', 'FFFF0000' )
        result = false
    else
        BlackMarket_NewAuctionBidText:setProperty( 'NormalColour', 'FFFFFFFF' )
    end
    
    if buyout > 0 and buyout < bid then
        -- Invalid buyout, too low
        BlackMarket_NewAuctionBuyoutText:setProperty( 'NormalColour', 'FFFF0000' )
        result = false
    else
        BlackMarket_NewAuctionBuyoutText:setProperty( 'NormalColour', 'FFFFFFFF' )
    end
    
    BlackMarket_CreateButton:setEnabled( result )
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

BlackMarket_SearchButton:subscribe( BlackMarket_SearchButton.EventClicked, 'BlackMarketMod.onSearchClicked' )
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

BlackMarket_SearchQualityCombo:subscribe( BlackMarket_SearchQualityCombo.EventListSelectionAccepted, 'BlackMarketMod.onSearchComboAccepted' )

BlackMarket_NewAuctionBidText:subscribe( BlackMarket_NewAuctionBidText.EventKeyDown, 'BlackMarketMod.onCreateKeyPress' )
BlackMarket_NewAuctionBuyoutText:subscribe( BlackMarket_NewAuctionBuyoutText.EventKeyDown, 'BlackMarketMod.onCreateKeyPress' )


BlackMarket_TabButton1:setID(1)
BlackMarket_TabButton2:setID(2)
BlackMarket_TabButton3:setID(3)
BlackMarket_Tab1:setID(1)
BlackMarket_Tab2:setID(2)
BlackMarket_Tab3:setID(3)

BlackMarket_TabButton1:subscribe( BlackMarket_TabButton1.EventClicked, 'BlackMarketMod.onTabClicked' )
BlackMarket_TabButton2:subscribe( BlackMarket_TabButton2.EventClicked, 'BlackMarketMod.onTabClicked' )
BlackMarket_TabButton3:subscribe( BlackMarket_TabButton3.EventClicked, 'BlackMarketMod.onTabClicked' )

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

BlackMarketMod.viewData[UIAuctionView.MyAuctions] = {}
BlackMarketMod.viewData[UIAuctionView.MyAuctions].refreshCall = BlackMarketMod.refreshMyAuctionsView
--BlackMarketMod.viewData[UIAuctionView.MyAuctions].populateCall = BlackMarketMod.populateMyAuctionsRow
BlackMarketMod.viewData[UIAuctionView.MyAuctions].selectionCall = BlackMarketMod.myAuctionsSelectionUpdate
BlackMarketMod.viewData[UIAuctionView.MyAuctions].maxVis = BlackMarketMod.MAX_AUCTION_ROWS
BlackMarketMod.viewData[UIAuctionView.MyAuctions].scrollBar = BlackMarket_MyAuctionsScroll
BlackMarketMod.viewData[UIAuctionView.MyAuctions].format = "MyAuctions"
BlackMarketMod.viewData[UIAuctionView.MyAuctions].auctionToRow = {}
BlackMarketMod.viewData[UIAuctionView.MyAuctions].selectedAuctionId = 0
BlackMarketMod.viewData[UIAuctionView.MyAuctions].currentPage = 0

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

BlackMarketMod.initRows( UIAuctionView.SearchResults, 'BlackMarketMod.onSearchRowMouseDown' )
BlackMarketMod.initRows( UIAuctionView.MyAuctions, 'BlackMarketMod.onMyAuctionsRowMouseDown' )
BlackMarketMod.initRows( UIAuctionView.MyBids, 'BlackMarketMod.onMyBidsRowMouseDown' )

BlackMarketMod.setupWindow()
BlackMarketWin:hide()