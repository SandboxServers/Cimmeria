-- Stub SGW client for the 009-starter-hotbar logic UAT.
--
-- Boots the ActionButtons module the way the client does: run the .toc's
-- scripts in order in one global environment, restore the persisted
-- variables, then fire Events.ModuleLoaded. Windows, CEGUI and the client
-- natives the module touches are stubs that record what happened.
--
-- The native action table (actionId -> abilityId) is treated as persisted
-- across logins. That is an assumption about SGW.exe, not a verified fact.

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
--   saved          persisted variables from an earlier session (Stubs.save)
--   noAbilityList  the client has no getAbilityList native
--   asyncActionUpdated  setActionToAbility does not fire Events.ActionUpdated
--   failSetAction  setActionToAbility raises a Lua error
function Stubs.newClient( opts )
    opts = opts or {}
    local client = {
        windows = {}, windowOrder = {}, feedback = {}, log = {}, used = {},
        material = {}, dropCallbacks = 0, time = 100.0,
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
    env.writeLocalFeedback = function( text ) client.feedback[#client.feedback + 1] = text end
    env.localize = function( mod, key ) return key end
    env.autoAnchorWindowExtents = function() end
    env.dragAborted = function() end
    env.getSystemTime = function() return client.time end
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
        env.getAbilityList = function( group )
            return deepcopy(client.known)
        end
    end

    env.getActionInfo = function( actionId )
        local abilityId = client.actions[actionId]
        if not abilityId then
            return {}
        end
        return { id = actionId, type = env.ActionType.Ability, subId = abilityId,
                 name = 'Ability '..abilityId, description = '', icon = 'set:Icons image:Ability'..abilityId,
                 quantity = -1, remainingCooldown = 0, totalCooldown = 0 }
    end
    env.getUnusedAction = function()
        for id = 1, 200 do
            if client.actions[id] == nil then
                return id
            end
        end
        return 0
    end
    env.setActionToAbility = function( actionId, abilityId )
        if opts.failSetAction then
            error('setActionToAbility failed')
        end
        client.actions[actionId] = abilityId
        if not opts.asyncActionUpdated then
            client.fire('Events.ActionUpdated', actionId)
        end
    end
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

    function client.load( chunkText, chunkName )
        local fn = assert(loadstring(chunkText, '@'..chunkName))
        setfenv(fn, env)
        fn()
    end

    -- Persisted variables restored before Events.ModuleLoaded.
    function client.restore()
        local saved = opts.saved
        env.GActionProfilesModVersion = 2
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

    -- The server's onKnownAbilitiesUpdate, as the client surfaces it.
    function client.learn( ids, how )
        for _, id in ipairs(ids) do
            client.known[#client.known + 1] = id
        end
        if how == 'property' then
            client.time = client.time + 5
            client.fire('Events.PropertyUpdated', 1, 1, 1)
        else
            for _, id in ipairs(ids) do
                client.fire('Events.AbilityUpdate', env.UIAbilityGroup.KnownAbility, id)
            end
        end
    end

    return client
end

return Stubs
