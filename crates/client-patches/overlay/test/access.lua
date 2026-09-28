-- REPL-style logic UAT for the overlay's Access.lua (the Access bar).
--
-- Loaded by run.lua, which passes its scenario/eq/ok helpers, so CI and
-- run_lupa.py run it with the Black Market scenarios.
--
-- Access.lua is loaded into a sandbox whose windows and modules are stubs
-- that record what they are asked to do. Each scenario clicks a button by
-- calling the handler name Access.lua subscribed to its EventClicked, the
-- way CEGUI would.

return function( here, scenario, eq, ok )

local LUA = here..'/../Content/UI/Core/Access/Access.lua'

--=============================================================================
local function newWindow( name )
    local w = { name = name, visible = false, fronted = 0, subscriptions = {} }
    w.EventClicked = 'EventClicked'
    w.EventCloseClicked = 'EventCloseClicked'
    function w:isVisible() return self.visible end
    function w:setVisible( v ) self.visible = v and true or false end
    function w:show() self.visible = true end
    function w:hide() self.visible = false end
    function w:moveToFront() self.fronted = self.fronted + 1 end
    function w:setProperty() end
    function w:setPosition() end
    function w:getPixelSize() return { width = 32, height = 32 } end
    function w:subscribe( event, handler ) self.subscriptions[event] = handler end
    return w
end

-- A module stub: every function called on it is recorded by name.
local function newModule( calls, modName )
    return setmetatable({}, { __index = function( t, fn )
        local f = function( ... )
            calls[#calls + 1] = modName..'.'..fn
            return ...
        end
        rawset(t, fn, f)
        return f
    end })
end

local function newClient()
    local env = { calls = {} }
    local calls = env.calls
    env.SocialWin = newWindow('SocialWin')
    env.SocialMod = {}
    -- The stock Social.lua body of onToggleSocial (Core/Social/Social.lua:9-12),
    -- which the O key binding (Actions.ToggleSocial) also reaches.
    function env.SocialMod.onToggleSocial( this )
        calls[#calls + 1] = 'SocialMod.onToggleSocial'
        this:setVisible(not this:isVisible())
        this:moveToFront()
    end
    for _, m in ipairs({ 'OptionsMod', 'ActionProfileMod', 'WorldMapMod', 'CraftingMod', 'GateMailMod' }) do
        env[m] = newModule(calls, m)
    end
    env.openWebsite = function( url ) calls[#calls + 1] = 'openWebsite' end
    env.logOff = function() calls[#calls + 1] = 'logOff' end
    env.triggerTutorialDialogId = function() end
    env.UITutorial = setmetatable({}, { __index = function( _, k ) return k end })
    env.Events = setmetatable({}, { __index = function( _, k ) return 'Events.'..k end })
    env.Actions = setmetatable({}, { __index = function( _, k ) return 'Actions.'..k end })
    env.CEGUI = { UVector2 = function() end, UDim = function() end }
    setmetatable(env, { __index = function( t, k )
        if type(k) == 'string' and (k:match('Win$') or k:match('Button$')) then
            local w = newWindow(k)
            rawset(t, k, w)
            return w
        end
        return _G[k]
    end })
    local chunk = assert(loadfile(LUA))
    setfenv(chunk, env)
    chunk()
    return env
end

-- Resolve 'AccessMod.onX' in the sandbox and call it as CEGUI would.
local function click( env, buttonName )
    local button = env[buttonName]
    local handler = button.subscriptions.EventClicked
    ok(handler, buttonName..' has an EventClicked handler')
    local mod, fn = handler:match('^([%w_]+)%.([%w_]+)$')
    local f = env[mod] and env[mod][fn]
    ok(type(f) == 'function', handler..' resolves to a function')
    env.AccessWin:show()
    f(button)
end

local function called( env, name )
    for _, c in ipairs(env.calls) do
        if c == name then return true end
    end
    return false
end

--=============================================================================
scenario('access: the Social button opens the Social window and closes the bar', function()
    local env = newClient()
    click(env, 'Access_SocialButton')
    ok(called(env, 'SocialMod.onToggleSocial'), 'goes through the same toggle as the O key')
    eq(env.SocialWin:isVisible(), true, 'Social window shown')
    ok(env.SocialWin.fronted > 0, 'Social window brought to the front')
    eq(env.AccessWin:isVisible(), false, 'Access bar hidden')
end)

scenario('access: a second Social click closes the Social window', function()
    local env = newClient()
    click(env, 'Access_SocialButton')
    click(env, 'Access_SocialButton')
    eq(env.SocialWin:isVisible(), false, 'Social window toggled off')
end)

-- Every Access-bar button must do something visible on the first press and
-- close the bar, as the stock buttons other than Social always did.
local BUTTONS = {
    { 'Access_SupportButton', 'openWebsite' },
    { 'Access_OptionsButton', 'OptionsMod.onToggleOptions' },
    { 'Access_KeyBindingsButton', 'OptionsMod.onToggleBindings' },
    { 'Access_CustomizeUIButton', 'ActionProfileMod.onToggleEditMode' },
    { 'Access_LogoutButton', 'logOff' },
    { 'Access_CharacterSheetButton', nil, 'CharacterWin' },
    { 'Access_MissionLogButton', nil, 'MissionLogWin' },
    { 'Access_WorldMapButton', 'WorldMapMod.onToggleWorldMap' },
    { 'Access_InventoryButton', nil, 'InventoryWin' },
    { 'Access_AbilitiesButton', nil, 'AbilityWin' },
    { 'Access_CraftingButton', 'CraftingMod.onToggleCrafting' },
    { 'Access_SocialButton', 'SocialMod.onToggleSocial', 'SocialWin' },
    { 'Access_MailButton', 'GateMailMod.onToggleMailbox' },
}

scenario('access: every Access-bar button acts on the first press and closes the bar', function()
    for _, b in ipairs(BUTTONS) do
        local env = newClient()
        click(env, b[1])
        if b[2] then
            ok(called(env, b[2]), b[1]..' calls '..b[2])
        end
        if b[3] then
            eq(env[b[3]]:isVisible(), true, b[1]..' shows '..b[3])
        end
        eq(env.AccessWin:isVisible(), false, b[1]..' hides the Access bar')
    end
end)

end
