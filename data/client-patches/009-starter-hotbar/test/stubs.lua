-- Stub SGW client for the 009-starter-hotbar logic UAT.
--
-- Boots the ActionButtons module the way the client does: run the .toc's
-- scripts in order in one global environment, restore the persisted
-- variables, then fire Events.ModuleLoaded. Windows, CEGUI and the client
-- natives the module touches are stubs that record what happened.
--
-- The native action table (actionId -> abilityId) is treated as persisted
-- across logins. That is an assumption about SGW.exe, not a verified fact.
--
-- The natives the hook calls are tolua++ shims in SGW.exe with strict arity:
-- a missing, extra or wrongly typed argument raises "error in function
-- '<name>'". These stubs raise the same way, with the shapes read from the
-- shim decompiles (headless Ghidra, 2026-10-04):
--   getAbilityList()            0x00aa2740  isnoobj(1)
--   getUnusedAction()           0x00aa9400  isnoobj(1)
--   getSystemTime()             0x00aa1b40  isnoobj(1)
--   getActionInfo(n)            0x00aa9590  isnumber(1) isnoobj(2)
--   setActionToAbility(n, n)    0x00aa9720  isnumber(1) isnumber(2) isnoobj(3)
--   unitsEqual(n, n)            0x00aa0120  isnumber(1) isnumber(2) isnoobj(3)
--   writeLocalFeedback(s)       not decompiled; checked as one string

local Stubs = {}

local function deepcopy( v, seen )
    if type(v) ~= 'table' then
        return v
    end
    seen = seen or {}
    if seen[v] then
        return seen[v]
    end
    local t = {}
    seen[v] = t
    for k, x in pairs(v) do
        t[deepcopy(k, seen)] = deepcopy(x, seen)
    end
    return t
end
Stubs.deepcopy = deepcopy

function Stubs.readFile( path )
    local f = io.open(path, 'rb')
    if not f then
        return nil
    end
    local s = f:read('*a')
    f:close()
    return s
end

-- A native with tolua's argument check in front of it.
local function native( name, types, fn )
    return function( ... )
        local n = select('#', ...)
        if n ~= #types then
            error("error in function '"..name.."'. "..n.." argument(s) given, "..#types.." expected", 2)
        end
        for i = 1, n do
            local t = type((select(i, ...)))
            if t ~= types[i] then
                error("error in function '"..name.."'. argument #"..i.." is '"..t.."'; '"..types[i].."' expected.", 2)
            end
        end
        return fn(...)
    end
end
Stubs.native = native

-- Time passes between logins: every client starts later than the last.
local clock = 100.0

--=============================================================================
-- Windows. Every method the module calls and the stub does not model is a
-- no-op; Event* fields are the event's name.
local WindowMethods = {}
local WindowMT = {}
WindowMT.__index = function( w, k )
    local m = WindowMethods[k]
    if m then
        return m
    end
    if type(k) == 'string' and k:match('^Event') then
        return k
    end
    return function() end
end

local function newWindow( client, name )
    local w = setmetatable({
        name = name, id = 0, props = {}, subs = {}, visible = true,
        pos = { x = { scale = 0, offset = 0 }, y = { scale = 0, offset = 0 } },
    }, WindowMT)
    client.windows[name] = w
    client.windowOrder[#client.windowOrder + 1] = w
    return w
end

function WindowMethods.getID( w ) return w.id end
function WindowMethods.setID( w, id ) w.id = id end
function WindowMethods.setProperty( w, k, v ) w.props[k] = v end
function WindowMethods.getProperty( w, k ) return w.props[k] end
function WindowMethods.subscribe( w, ev, handler ) w.subs[ev] = handler end
function WindowMethods.unsubscribe( w, ev ) w.subs[ev] = nil end
function WindowMethods.setVisible( w, v ) w.visible = v and true or false end
function WindowMethods.show( w ) w.visible = true end
function WindowMethods.hide( w ) w.visible = false end
function WindowMethods.isVisible( w ) return w.visible end
function WindowMethods.setTooltipText( w, t ) w.tooltip = t end
function WindowMethods.setText( w, t ) w.text = t end
function WindowMethods.getPixelSize( w ) return { width = 36, height = 36 } end
function WindowMethods.setArea( w, pos, size ) w.pos = pos end
function WindowMethods.setPosition( w, pos ) w.pos = pos end
function WindowMethods.getPosition( w ) return w.pos end
function WindowMethods.getChildCount( w ) return 0 end
function WindowMethods.getArea( w )
    return {
        asAbsolute = function()
            local x, y = w.pos.x.offset, w.pos.y.offset
            return { left = x, top = y, right = x + 36, bottom = y + 36 }
        end,
    }
end
function WindowMethods.getSelectedItem( w ) return { getID = function() return 1 end } end

local function isWindowName( k )
    return type(k) == 'string' and ( k:match('Win$') or k:match('_') ) and not k:match('^__')
end

--=============================================================================
-- opts:
--   known          ability ids in the known-abilities cache at module load
--   saved          persisted variables from an earlier session (client.save)
--   modVersion     the saved GActionProfilesModVersion (default 2; 1 triggers
--                  the stock version-2 wipe)
--   noAbilityList  the client has no getAbilityList native
--   brokenAbilityList  getAbilityList raises
--   asyncActionUpdated  setActionToAbility does not fire Events.ActionUpdated
--   failSetAction  setActionToAbility raises a Lua error
--   weaponItemId   the active bandolier weapon's item id
function Stubs.newClient( opts )
    opts = opts or {}
    clock = clock + 1000
    local client = {
        windows = {}, windowOrder = {}, feedback = {}, log = {}, used = {},
        material = {}, dropCallbacks = 0, time = clock, listCalls = 0,
        known = deepcopy(opts.known or {}),
        actions = deepcopy(opts.saved and opts.saved.actions or {}),
    }

    local env = {}
    setmetatable(env, {
        __index = function( t, k )
            local g = _G[k]
            if g ~= nil then
                return g
            end
            if isWindowName(k) then
                return client.windows[k] or newWindow(client, k)
            end
            return nil
        end,
    })
    env._G = env
    client.env = env

    env.Events = setmetatable({}, { __index = function( _, k ) return 'Events.'..k end })
    env.Actions = setmetatable({}, { __index = function( _, k ) return 'Actions.'..k end })

    env.CEGUI = {
        UDim = function( s, o ) return { scale = s, offset = o } end,
        UVector2 = function( x, y ) return { x = x, y = y } end,
        Vector2 = function() return { x = 0, y = 0 } end,
        Rect = function( l, t, r, b ) return { left = l, top = t, right = r, bottom = b } end,
        colour = function() return {} end,
        CoordConverter = {
            windowToScreenX = function() return 0 end,
            windowToScreenY = function() return 0 end,
            screenToWindowX = function() return 0 end,
        },
        createListboxTextItem = function() return setmetatable({ props = {}, subs = {} }, WindowMT) end,
    }

    env.UIAbilityGroup = { None = 0, Training = 1, KnownAbility = 2, Inventory = 3 }
    env.ActionType = { Ability = 1, Item = 2, Macro = 3 }
    env.UIDragType = { Ability = 1, PetAbility = 2, PetCommand = 3, PetStance = 4, Custom = 5, Action = 6, Item = 7, Macro = 8 }
    env.Container = { Bandolier = 3 }
    env.Unit = { Player = 1, Target = 2 }
    env.PromptMod = { showPrompt = function() end, showPromptEdit = function() end }
    env.BackgroundMod = {
        registerDropCallback = function() client.dropCallbacks = client.dropCallbacks + 1 end,
        unregisterDropCallback = function() end,
    }
    env.Debug = {
        log = function( self, text ) client.log[#client.log + 1] = text end,
        warn = function( self, text ) client.log[#client.log + 1] = text end,
    }

    env.getModuleInfo = function() return { version = 2 } end
    env.writeLocalFeedback = native('writeLocalFeedback', { 'string' }, function( text )
        client.feedback[#client.feedback + 1] = text
    end)
    env.localize = function( mod, key ) return key end
    env.autoAnchorWindowExtents = function() end
    env.dragAborted = function() end
    env.getSystemTime = native('getSystemTime', {}, function() return client.time end)
    env.unitsEqual = native('unitsEqual', { 'number', 'number' }, function( a, b ) return a == b end)
    env.getOptionInfo = function() return { value = false } end
    env.getBindingKey = function() return {} end
    env.getImageInfo = function() return {} end
    env.setMaterialTextureProperty = function( mat, prop, value )
        client.material[mat] = client.material[mat] or {}
        client.material[mat][prop] = value
    end
    env.setMaterialVectorProperty = function() end
    env.setMaterialScalarProperty = function() end
    env.getActiveSlotForContainer = function() return 0 end
    env.getItemIDForSlot = function() return opts.weaponItemId end

    env.getAbilityInfo = function( id )
        return { id = id, name = 'Ability '..id, description = '', icon = 'set:Icons image:Ability'..id,
                 isWeaponAbility = false, isDeployAbility = false }
    end
    if not opts.noAbilityList then
        env.getAbilityList = native('getAbilityList', {}, function()
            client.listCalls = client.listCalls + 1
            if opts.brokenAbilityList then
                error('getAbilityList: no player entity')
            end
            return deepcopy(client.known)
        end)
    end

    env.getActionInfo = native('getActionInfo', { 'number' }, function( actionId )
        local abilityId = client.actions[actionId]
        if not abilityId then
            return {}
        end
        return { id = actionId, type = env.ActionType.Ability, subId = abilityId,
                 name = 'Ability '..abilityId, description = '', icon = 'set:Icons image:Ability'..abilityId,
                 quantity = -1, remainingCooldown = 0, totalCooldown = 0 }
    end)
    -- 1..200, or -1 when every action slot is used (0x00ad9560).
    env.getUnusedAction = native('getUnusedAction', {}, function()
        for id = 1, 200 do
            if client.actions[id] == nil then
                return id
            end
        end
        return -1
    end)
    env.setActionToAbility = native('setActionToAbility', { 'number', 'number' }, function( actionId, abilityId )
        if opts.failSetAction then
            error('setActionToAbility failed')
        end
        client.actions[actionId] = abilityId
        if not opts.asyncActionUpdated then
            client.fire('Events.ActionUpdated', actionId)
        end
    end)
    env.clearAction = function( actionId ) client.actions[actionId] = nil end
    env.useAction = function( actionId )
        client.used[#client.used + 1] = client.actions[actionId] or false
    end

    -- Call every window's handler for an event, as the client would.
    function client.fire( ev, ... )
        for _, w in ipairs(client.windowOrder) do
            local handler = w.subs[ev]
            if handler then
                local fn = env
                for part in handler:gmatch('[^.]+') do
                    fn = fn[part]
                end
                fn(w, ...)
            end
        end
    end

    function client.subscribed( ev )
        local w = client.windows.ActionButtonsWin
        return w ~= nil and w.subs[ev] ~= nil
    end

    function client.load( chunkText, chunkName )
        local fn = assert(loadstring(chunkText, '@'..chunkName))
        setfenv(fn, env)
        fn()
    end

    -- Persisted variables restored before Events.ModuleLoaded.
    function client.restore( saved )
        saved = saved or opts.saved
        env.GActionProfilesModVersion = opts.modVersion or 2
        if saved then
            env.GActionProfiles = deepcopy(saved.GActionProfiles)
            env.GActionCurrentProfileId = saved.GActionCurrentProfileId
            env.GActionDragDockGroup = saved.GActionDragDockGroup
        end
    end

    function client.save()
        return {
            GActionProfiles = deepcopy(env.GActionProfiles),
            GActionCurrentProfileId = env.GActionCurrentProfileId,
            GActionDragDockGroup = env.GActionDragDockGroup,
            actions = deepcopy(client.actions),
        }
    end

    -- Log out to character select and back in with the Lua state kept: the
    -- character variables are saved and restored, ModuleLoaded fires again,
    -- and no script runs again.
    function client.relogKeepingLuaState()
        local saved = client.save()
        client.time = client.time + 600
        opts.modVersion = 2
        client.restore(saved)
        client.fire('Events.ModuleLoaded')
    end

    function client.advance( seconds )
        client.time = client.time + seconds
    end

    function client.profile()
        return env.GActionProfiles[env.GActionCurrentProfileId]
    end

    -- The ability on button n's current action, or nil.
    function client.abilityOnButton( n )
        local actionId = env.ActionButtonMod.getActionForButton(n)
        return actionId and client.actions[actionId] or nil
    end

    -- The icon the button's cooldown material shows.
    function client.iconOnButton( n )
        local m = client.material['CoreMaterial_ButtonTimer'..n]
        return m and m.Icon or nil
    end

    -- Click button n, as EventClicked would.
    function client.press( n )
        local w = client.windows[string.format('ActionButtons_%dButton', n)]
        local handler = w.subs[w.EventClicked]
        local fn = env
        for part in handler:gmatch('[^.]+') do
            fn = fn[part]
        end
        fn(w)
        return client.used[#client.used]
    end

    -- The player's own property update (unit defaults to the player).
    function client.propertyUpdate( unitId )
        client.fire('Events.PropertyUpdated', unitId or env.Unit.Player, 1, 1)
    end

    -- The server's onKnownAbilitiesUpdate, as the client surfaces it. By
    -- default the cache grows one id at a time with an Events.AbilityUpdate
    -- after each; 'property' adds them all and fires one player
    -- PropertyUpdated 5 s later; 'silent' only fills the cache.
    function client.learn( ids, how )
        if how == 'property' or how == 'silent' then
            for _, id in ipairs(ids) do
                client.known[#client.known + 1] = id
            end
            if how == 'property' then
                client.advance(5)
                client.propertyUpdate()
            end
        else
            for _, id in ipairs(ids) do
                client.known[#client.known + 1] = id
                client.fire('Events.AbilityUpdate', env.UIAbilityGroup.KnownAbility, id)
            end
        end
    end

    return client
end

return Stubs
