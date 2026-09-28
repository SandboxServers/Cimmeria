-- Stubs of the client's CEGUI window API, the tolua globals and the
-- client-patch DLL's CimmeriaBMNative, enough to load the overlay's
-- BlackMarket.lua in a stock Lua 5.1 and drive it.
--
-- The windows come from the overlay's BlackMarket.layout itself: every named
-- window and every LayoutImport'ed row exists, nothing else does. A name the
-- Lua builds wrongly (U7) or a row the layout never imports (U8) is nil here,
-- as it is in the client.

local Stubs = {}

-- Child windows of the two stock row layouts (not in git: they are unchanged
-- client files, Content/UI/Core/BlackMarket/BlackMarket_*Row.layout).
local ROW_CHILDREN = {
    ['BlackMarket_ItemRow.layout'] = {
        'MainContainer', 'Highlight', 'IconContainer', 'Icon', 'QtyText', 'NameText',
        'TCText', 'TimerImage', 'CurrentBidText', 'BuyoutText', 'SellerText', 'BidCountText',
    },
    ['BlackMarket_YourAuctionRow.layout'] = {
        'MainContainer', 'Highlight', 'IconContainer', 'Icon', 'QtyText', 'NameText',
        'TCText', 'TimerImage', 'CurrentBidText', 'BuyoutText', 'BidderText', 'BidCountText',
    },
}

local function readFile( path )
    local f = assert(io.open(path, 'rb'))
    local s = f:read('*a')
    f:close()
    return s
end
Stubs.readFile = readFile

-- Names of every window the layout creates, comments stripped.
function Stubs.layoutWindowNames( layoutPath )
    local xml = readFile(layoutPath):gsub('<!%-%-.-%-%->', '')
    local names = {}
    for name in xml:gmatch('<Window[^>]-Name="([^"]+)"') do
        names[name] = true
    end
    for prefix, file in xml:gmatch('<LayoutImport Prefix="([^"]+)" Filename="([^"]+)"') do
        local children = assert(ROW_CHILDREN[file], 'unknown row layout '..file)
        for i, child in ipairs(children) do
            names[prefix..child] = true
        end
    end
    return names
end

--=============================================================================
-- Windows
--=============================================================================
local Window = {}

-- Event name fields (row.EventMouseButtonDown, ...) read as their own names.
local function windowIndex( self, key )
    local method = Window[key]
    if method then
        return method
    end
    if type(key) == 'string' and key:sub(1, 5) == 'Event' then
        return key
    end
    return nil
end

function Stubs.newWindow( env, name )
    local w = setmetatable({
        name = name, env = env, text = '', id = 0, visible = true, enabled = true,
        props = {}, subs = {}, active = false,
        pageSize = 0, docSize = 0, scrollPos = 0, items = {},
    }, { __index = windowIndex })
    return w
end

function Window:getName() return self.name end
function Window:show()
    if not self.visible then
        self.visible = true
        self:fire('EventShown')
    end
end
function Window:hide() self.visible = false end
function Window:setVisible( v )
    if v then self:show() else self:hide() end
end
function Window:isVisible() return self.visible end
function Window:setText( t ) self.text = tostring(t) end
function Window:getText() return self.text end
function Window:setID( id ) self.id = id end
function Window:getID() return self.id end
function Window:setEnabled( e ) self.enabled = e and true or false end
function Window:isEnabled() return self.enabled end
function Window:setProperty( k, v ) self.props[k] = v end
function Window:getProperty( k ) return self.props[k] end
function Window:activate() self.active = true end
function Window:isActive() return self.active end
function Window:setSelection( a, b ) self.selection = { a, b } end

function Window:subscribe( event, handlerName )
    self.subs[event] = self.subs[event] or {}
    table.insert(self.subs[event], handlerName)
end
function Window:unsubscribe( event ) self.subs[event] = nil end
function Window:subscriberCount( event ) return #(self.subs[event] or {}) end

-- Scrollbar
function Window:setPageSize( n ) self.pageSize = n end
function Window:getPageSize() return self.pageSize end
function Window:setStepSize( n ) self.stepSize = n end
function Window:setDocumentSize( n ) self.docSize = n end
function Window:getDocumentSize() return self.docSize end
function Window:setOverlapSize( n ) self.overlap = n end
function Window:getScrollPosition() return self.scrollPos end
function Window:setScrollPosition( p )
    p = math.max(0, math.min(p, math.max(self.docSize - self.pageSize, 0)))
    if p ~= self.scrollPos then
        self.scrollPos = p
        self:fire('EventScrollPositionChanged')
    end
end

-- Combobox
function Window:resetList() self.items = {}; self.selected = nil end
function Window:setSortingEnabled( e ) end
function Window:addItem( item ) table.insert(self.items, item) end
function Window:setItemSelectState( item, s ) if s then self.selected = item end end
function Window:getSelectedItem() return self.selected end
function Window:getEditbox() return self end

-- Resolve 'BlackMarketMod.onX' in the overlay's environment.
local function resolve( env, handlerName )
    local fn = env
    for part in handlerName:gmatch('[^%.]+') do
        fn = fn[part]
        if fn == nil then
            error('no handler '..handlerName)
        end
    end
    return fn
end

-- A CEGUI event: handler( this, window, ... ). The overlay's handlers read
-- the source window from either argument.
function Window:fire( event, ... )
    local results = {}
    for i, handlerName in ipairs(self.subs[event] or {}) do
        results[#results + 1] = resolve(self.env, handlerName)(self, self, ...)
    end
    return results[1]
end

-- A game event (Events.BMOpen, ...): handler( this, args... ).
function Window:fireGame( event, ... )
    for i, handlerName in ipairs(self.subs[event] or {}) do
        resolve(self.env, handlerName)(self, ...)
    end
end

--=============================================================================
-- The overlay's environment
--=============================================================================
function Stubs.newEnv( opts )
    opts = opts or {}
    local env = {}
    env._G = env
    setmetatable(env, { __index = _G })

    env.windows = {}
    for name in pairs(opts.windowNames) do
        local w = Stubs.newWindow(env, name)
        env.windows[name] = w
        env[name] = w
    end

    env.UIAuctionView = { SearchResults = 0, MyAuctions = 1, MyBids = 2 }
    env.UIAuctionTime = { VeryShort = 1, Short = 2, Medium = 3, Long = 4, VeryLong = 5 }
    env.UIItemQuality = { Normal = 1, Good = 2, Great = 3, Fantastic = 4 }
    env.UIDragType = { Item = 1, Ability = 2 }
    env.Events = setmetatable({}, { __index = function(t, k) return 'Events.'..k end })

    local colour = function(r, g, b, a) return { top_left = { r, g, b, a } } end
    env.CEGUI = {
        colour = colour,
        Key = { Tab = 15 },
        PropertyHelper = { colourToString = function(self, c) return 'FFFFFFFF' end },
        createListboxTextItem = function(text)
            local item = { text = text, id = 0 }
            function item:setSelectionColours() end
            function item:setSelectionBrushImage() end
            function item:setTextColours( c ) self.colours = c end
            function item:getTextColours() return self.colours end
            function item:setID( id ) self.id = id end
            function item:getID() return self.id end
            return item
        end,
    }
    env.localize = function(section, key) return key end

    env.clock = 0
    env.worldGetDeltaSeconds = function() return env.clock end
    env.cash = 1000
    env.getCash = function() return env.cash end

    -- Item definitions: id -> { ID, Name, Icon }
    env.itemDefs = opts.itemDefs or {}
    env.getItemDefInfo = function(id)
        local def = env.itemDefs[id]
        if not def then
            return {}
        end
        return { ID = id, Name = def.Name, Description = '', Icon = def.Icon, Tier = 1 }
    end
    -- Bag slots: container..':'..slot -> { itemId, itemDefId, qty }
    env.bags = {}
    env.getItemIDForSlot = function(c, s) return (env.bags[c..':'..s] or {}).itemId or 0 end
    env.getQuantityForSlot = function(c, s) return (env.bags[c..':'..s] or {}).qty or 0 end
    env.getItemDef = function(itemId)
        for key, slot in pairs(env.bags) do
            if slot.itemId == itemId then
                return slot.itemDefId
            end
        end
        return 0
    end
    env.drag = { nil, nil }
    env.getDragInfo = function() return env.drag[1], env.drag[2] end

    env.logLines = {}
    env.Debug = { log = function(self, line) table.insert(env.logLines, line) end }

    if opts.native ~= false then
        env.CimmeriaBMNative = Stubs.newNative(env)
    end
    return env
end

-- The DLL's send table: records every call; `reply[op]` overrides the result.
function Stubs.newNative( env )
    local native = { version = 'test', calls = {}, reply = {}, tc = {} }
    local function op( name )
        return function(...)
            table.insert(native.calls, { op = name, args = { ... } })
            local r = native.reply[name]
            if r then
                return r[1], r[2]
            end
            return true
        end
    end
    native.search = op('search')
    native.create = op('create')
    native.bid = op('bid')
    native.cancel = op('cancel')
    native.watch = op('watch')
    native.techCompetency = function(itemDefId) return native.tc[itemDefId] end
    return native
end

-- Load the overlay's BlackMarket.lua into `env`.
function Stubs.loadOverlay( env, luaPath )
    local chunk = assert(loadfile(luaPath))
    setfenv(chunk, env)
    chunk()
    return env
end

return Stubs
