-- Stubs of the client's CEGUI window API and the client calls that Login.lua
-- makes (accountLogin, PromptMod.showPrompt, the link opener, quit, localize,
-- the saved variables), enough to load the 017 overlay's Login.lua in a stock
-- Lua 5.1 and drive it.
--
-- The windows come from Login.layout itself: every window the layout names
-- exists with its layout Visible and Text, nothing else does. A name the
-- overlay misspells is nil here, as it is in the client. The calls the client
-- receives are recorded in Stubs.calls, Stubs.prompts, Stubs.opened and
-- Stubs.quits for the asserts in run.lua.

local Stubs = {}

local function readFile( path )
    local f = assert(io.open(path, 'rb'))
    local s = f:read('*a')
    f:close()
    return s
end
Stubs.readFile = readFile

-- Name, Visible and Text of each window the layout creates, comments
-- stripped. A property belongs to the window whose open tag came before it;
-- the layout writes a window's properties before its child windows.
local function layoutWindows( path )
    local xml = readFile(path):gsub('<!%-%-.-%-%->', '')
    local list, cur = {}, nil
    for tag, attrs in xml:gmatch('<(/?%a+)([^>]*)>') do
        if tag == 'Window' then
            cur = { name = attrs:match('Name="([^"]+)"'), visible = true, text = '' }
            if cur.name then
                list[#list + 1] = cur
            end
        elseif tag == '/Window' then
            cur = nil
        elseif tag == 'Property' and cur then
            local name = attrs:match('Name="([^"]*)"')
            local value = attrs:match('Value="([^"]*)"')
            if name == 'Visible' then
                cur.visible = (value == 'True')
            elseif name == 'Text' then
                cur.text = value
            end
        end
    end
    return list
end

--=============================================================================
-- Windows
--=============================================================================
local Window = {}

-- Event name fields (Login_LoginButton.EventClicked, ...) read as their own names.
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

local function newWindow( name, visible, text )
    return setmetatable({
        name = name, visible = visible, text = text, enabled = true, active = false,
        selected = false, selectedItem = nil, subs = {}, props = {},
    }, { __index = windowIndex })
end

function Window:getName() return self.name end
function Window:show() self.visible = true end
function Window:hide() self.visible = false end
function Window:setVisible( v ) self.visible = v and true or false end
function Window:isVisible() return self.visible end
function Window:setText( t ) self.text = tostring(t) end
function Window:getText() return self.text end
function Window:setEnabled( e ) self.enabled = e and true or false end
function Window:isEnabled() return self.enabled end
function Window:setProperty( k, v ) self.props[k] = v end
function Window:getProperty( k ) return self.props[k] end
function Window:activate() self.active = true end
function Window:isActive() return self.active end
function Window:setSelection( a, b ) self.selection = { a, b } end
function Window:setCaratIndex( i ) self.carat = i end
function Window:setSelected( s ) self.selected = s and true or false end
function Window:isSelected() return self.selected end

-- Listbox items: CEGUI.createListboxTextItem gives a window whose text is its name.
function Window:setSelectionColours() end
function Window:setSelectionBrushImage() end
function Window:addItem( item ) self.items = (self.items or 0) + 1 end
function Window:setItemSelectState( item, state )
    if state then
        self.selectedItem = item
    end
end
function Window:getSelectedItem() return self.selectedItem end

function Window:subscribe( event, handlerName )
    self.subs[event] = self.subs[event] or {}
    table.insert(self.subs[event], handlerName)
end
function Window:unsubscribe( event ) self.subs[event] = nil end
function Window:subscriberCount( event ) return #(self.subs[event] or {}) end

-- Calls the handlers subscribed to event on w, as CEGUI does: handler(this, window).
function Stubs.fire( w, event )
    for _, handler in ipairs(w.subs[event] or {}) do
        local f = _G
        for part in handler:gmatch('[^.]+') do
            f = f[part]
        end
        f(w, w)
    end
end

-- A clicked button, by its layout name.
function Stubs.press( name )
    Stubs.fire(_G[name], 'EventClicked')
end

--=============================================================================
-- Client globals
--=============================================================================
-- Resets the client to a fresh boot: the layout's windows as globals, the
-- client calls with empty records, no saved variables, no CimmeriaLogin.
function Stubs.reset( layoutPath )
    for _, w in ipairs(layoutWindows(layoutPath)) do
        _G[w.name] = newWindow(w.name, w.visible, w.text)
    end

    Stubs.calls, Stubs.prompts, Stubs.opened, Stubs.quits = {}, {}, {}, 0
    LoginMod = nil
    CimmeriaLogin = nil
    GSavedVar_AccountName = nil
    GSavedVar_ShouldSaveAccount = nil

    Events = { LoginFailed = 'LoginFailed' }
    CEGUI = {
        Key = { Return = 28, Tab = 15 },
        colour = function() return {} end,
        createListboxTextItem = function( name ) return newWindow(name, true, name) end,
    }
    OptionsMod = { onToggleOptions = function() end }
    PromptMod = {
        showPrompt = function( title, text )
            table.insert(Stubs.prompts, text)
        end,
    }
    function getVersionInfo() return { FileVersion = '0.0.0.0' } end
    function localize( _, text ) return text end
    function openWebsite( url ) table.insert(Stubs.opened, url) end
    function quit() Stubs.quits = Stubs.quits + 1 end

    -- The status text is read when the ticket is sent: Login.lua sets
    -- "Connecting..." before it calls accountLogin.
    function accountLogin( server, user, pass )
        table.insert(Stubs.calls, {
            server = server, user = user, pass = pass,
            status = _G.Login_StatusText:getText(),
        })
    end
end

return Stubs
