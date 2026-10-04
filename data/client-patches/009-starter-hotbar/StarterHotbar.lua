

--=============================================================================
-- Cimmeria client patch 009-starter-hotbar. Cimmeria code, not CME's.
--
-- A new character's action bar is client state only (GActionProfiles, a
-- CharacterVariable of the ActionButtons module), so the server cannot put
-- the character's starting abilities on it. The patch appends this block to
-- the stock ActionProfileDefault1.lua, which ActionButtons.toc loads after
-- ActionProfiles.lua. It places the starting abilities the character knows
-- on the empty layer-bound buttons (11-20) of the profile the client creates
-- when the character has no saved UI variables (its first login on this
-- machine), and never touches any other profile.
--
-- It only binds actions to buttons that already exist. It never creates,
-- moves or resizes a button, and it changes no file a UI pack replaces: it
-- wraps ActionProfileMod.refreshProfileTemplateCombo, createProfile and
-- loadProfile, which the stock ActionProfiles.lua and the WQHD v26
-- replacement both define (refreshProfileTemplateCombo is called once, from
-- onModLoaded, in both).
--
-- State, kept in the profile (persisted with GActionProfiles):
--   cimmeriaStarterHotbar  nil        a profile this patch did not create
--                          'pending'  created fresh, nothing placed yet
--                          'seeding'  placed abilities, tops up as more arrive
--                          'done'     finished; never runs again
--   cimmeriaStarterLoad    the module load (login) that created the profile
-- Seeding happens only during the module load that created the profile. At
-- any later load, 'pending' and 'seeding' both become 'done', so nothing is
-- ever added to a bar after the character's first session.
--=============================================================================
ActionProfileMod.StarterHotbar = {
    -- Attacks first, then heals: Pistol Shot, Strike, Heal Focus,
    -- Health Heal, Medical Attention: Recuperation.
    Abilities = { 592, 594, 597, 1646, 1218 },
    FirstButton = 11,
    LastButton = 20,
    StateKey = 'cimmeriaStarterHotbar',
    LoadKey = 'cimmeriaStarterLoad',
    Pending = 'pending',
    Seeding = 'seeding',
    Done = 'done',
    PollSeconds = 1.0,

    loadToken = nil,        -- this module load; see newLoadToken
    loadCount = 0,
    busy = false,
    lastPoll = nil,
    subscribed = false,
    listErrorLogged = false,
}

--=============================================================================
function ActionProfileMod.StarterHotbar.log( text )
    pcall( function() Debug:log( '[cimmeria] starter hotbar: '..text ) end )
end

--=============================================================================
-- A token naming this module load. Stored in the profile it creates, so the
-- "first session" test does not depend on whether the client rebuilds the
-- Lua state between logins. The counter separates loads in one Lua state;
-- the clocks separate Lua states.
function ActionProfileMod.StarterHotbar.newLoadToken()
    local SH = ActionProfileMod.StarterHotbar
    SH.loadCount = SH.loadCount + 1
    local sys, wall = '', ''
    if type(getSystemTime) == 'function' then
        local ok, t = pcall( getSystemTime )
        if ok then sys = tostring(t) end
    end
    if os and type(os.time) == 'function' then
        local ok, t = pcall( os.time )
        if ok then wall = tostring(t) end
    end
    SH.loadToken = wall..'/'..sys..'/'..SH.loadCount
    return SH.loadToken
end

function ActionProfileMod.StarterHotbar.currentLoad()
    local SH = ActionProfileMod.StarterHotbar
    return SH.loadToken or SH.newLoadToken()
end

--=============================================================================
-- The ids in the client's known-abilities cache (filled by
-- onKnownAbilitiesUpdate), or nil when the client has no such list.
-- getAbilityList is a zero-argument native the stock UI never calls: its
-- tolua shim raises if it is given any argument.
function ActionProfileMod.StarterHotbar.knownAbilities()
    local SH = ActionProfileMod.StarterHotbar
    if type(getAbilityList) ~= 'function' then
        return nil
    end
    local ok, list = pcall( getAbilityList )
    if not ok or type(list) ~= 'table' then
        if not SH.listErrorLogged then
            SH.listErrorLogged = true
            SH.log( 'getAbilityList failed: '..tostring(list) )
        end
        return nil
    end
    local known = {}
    local count = 0
    for _, entry in pairs(list) do
        local id = entry
        if type(entry) == 'table' then
            id = entry.id
        end
        if type(id) == 'number' and not known[id] then
            known[id] = true
            count = count + 1
        end
    end
    return known, count
end

--=============================================================================
-- Ability ids already bound to any button of the profile, on any layer or
-- weapon, so the patch never adds a second copy of one the player placed.
function ActionProfileMod.StarterHotbar.abilitiesOnBar( profile )
    local onBar = {}
    if type(getActionInfo) ~= 'function' then
        return onBar
    end
    for _, info in pairs( profile.buttonInfo or {} ) do
        if type(info) == 'table' and type(info.actions) == 'table' then
            for _, actionId in pairs( info.actions ) do
                if type(actionId) == 'number' and actionId > 0 then
                    local ok, actionInfo = pcall( getActionInfo, actionId )
                    if ok and type(actionInfo) == 'table' and actionInfo.id and actionInfo.subId
                       and ( ActionType == nil or actionInfo.type == ActionType.Ability ) then
                        onBar[actionInfo.subId] = true
                    end
                end
            end
        end
    end
    return onBar
end

--=============================================================================
-- A layer-bound button of the profile with nothing on it: no action stored
-- for the current layer, and no action bound to the button (a button rebound
-- in the editor keeps its old action until the profile reloads).
function ActionProfileMod.StarterHotbar.isEmptyLayerButton( profile, buttonId )
    local info = profile.buttonInfo and profile.buttonInfo[buttonId]
    if type(info) ~= 'table' or info.binding ~= ActionProfileMod.ButtonBinding_Layer then
        return false
    end
    if not ActionProfileMod.getButtonWindow( buttonId ) then
        return false
    end
    local layer = profile.currentLayer or 1
    local actionId = info.actions and info.actions[layer]
    if actionId and actionId > 0 then
        return false
    end
    local bound = ActionButtonMod.getActionForButton( buttonId )
    if bound and bound > 0 then
        return false
    end
    return true
end

--=============================================================================
-- Bind a new action for abilityId to buttonId, the way dropping the ability
-- from the Ability window does (ActionProfileMod.receiveDrag).
function ActionProfileMod.StarterHotbar.place( buttonId, abilityId )
    local actionId = getUnusedAction()
    if not actionId or actionId < 1 then
        return false
    end
    ActionProfileMod.setButtonCurrentAction( buttonId, actionId )
    setActionToAbility( actionId, abilityId )
    -- Redraw now, so the icon shows without waiting for Events.ActionUpdated.
    ActionButtonMod.bindButtonToAction( buttonId, actionId )
    return true
end

--=============================================================================
-- Place every known starting ability not yet on the bar.
-- Returns placed, missing (not known yet), full (no button or action left).
function ActionProfileMod.StarterHotbar.seed( profile )
    local SH = ActionProfileMod.StarterHotbar
    local known = SH.knownAbilities()
    if not known then
        return 0, #SH.Abilities, false
    end
    local onBar = SH.abilitiesOnBar( profile )
    local placed, missing, full = 0, 0, false
    local buttonId = SH.FirstButton
    for _, abilityId in ipairs( SH.Abilities ) do
        if not onBar[abilityId] then
            if not known[abilityId] then
                missing = missing + 1
            else
                while buttonId <= SH.LastButton and not SH.isEmptyLayerButton( profile, buttonId ) do
                    buttonId = buttonId + 1
                end
                if buttonId > SH.LastButton or not SH.place( buttonId, abilityId ) then
                    full = true
                    break
                end
                onBar[abilityId] = true
                placed = placed + 1
                SH.log( 'placed ability '..abilityId..' on button '..buttonId )
                buttonId = buttonId + 1
            end
        end
    end
    return placed, missing, full
end

--=============================================================================
function ActionProfileMod.StarterHotbar.currentProfile()
    local profile = GActionProfiles and GActionProfiles[GActionCurrentProfileId]
    if type(profile) ~= 'table' then
        return nil
    end
    return profile
end

-- Is the current profile one this module load may still add to?
function ActionProfileMod.StarterHotbar.watching()
    local SH = ActionProfileMod.StarterHotbar
    local profile = SH.currentProfile()
    if not profile then
        return false
    end
    local state = profile[SH.StateKey]
    return ( state == SH.Pending or state == SH.Seeding )
       and profile[SH.LoadKey] == SH.currentLoad()
end

--=============================================================================
-- Listen for late abilities only while there is something to do.
function ActionProfileMod.StarterHotbar.updateSubscriptions()
    local SH = ActionProfileMod.StarterHotbar
    if not ( ActionButtonsWin and Events and Events.AbilityUpdate and Events.PropertyUpdated ) then
        return
    end
    local want = SH.watching()
    if want and not SH.subscribed then
        ActionButtonsWin:subscribe( Events.AbilityUpdate, 'ActionProfileMod.onStarterHotbarAbilityUpdate' )
        ActionButtonsWin:subscribe( Events.PropertyUpdated, 'ActionProfileMod.onStarterHotbarPropertyUpdated' )
        SH.subscribed = true
    elseif not want and SH.subscribed then
        ActionButtonsWin:unsubscribe( Events.AbilityUpdate )
        ActionButtonsWin:unsubscribe( Events.PropertyUpdated )
        SH.subscribed = false
    end
end

--=============================================================================
function ActionProfileMod.StarterHotbar.run( reason )
    local SH = ActionProfileMod.StarterHotbar
    if SH.busy then
        return
    end
    local profile = SH.currentProfile()
    if not profile then
        return
    end
    local state = profile[SH.StateKey]
    if ( state == SH.Pending or state == SH.Seeding ) and profile[SH.LoadKey] ~= SH.currentLoad() then
        profile[SH.StateKey] = SH.Done
        SH.log( 'done: the first session ended in state '..state )
    elseif SH.watching() then
        SH.busy = true
        local ok, placed, missing, full = pcall( SH.seed, profile )
        SH.busy = false

        if not ok then
            -- Never retry a failing seed on every event; the bar stays as it is.
            profile[SH.StateKey] = SH.Done
            SH.log( 'error ('..tostring(reason)..'): '..tostring(placed) )
        else
            if placed > 0 then
                if state == SH.Pending then
                    pcall( writeLocalFeedback, 'Your starting abilities are on your action bar.' )
                end
                profile[SH.StateKey] = SH.Seeding
                SH.log( 'placed '..placed..' ('..tostring(reason)..')' )
            end
            if missing == 0 or full then
                profile[SH.StateKey] = SH.Done
                SH.log( 'done: '..(full and 'no empty button or action left' or 'all starting abilities on the bar') )
            end
        end
    end
    SH.updateSubscriptions()
end

--=============================================================================
-- onModLoaded calls refreshProfileTemplateCombo once per module load, after
-- the stock version wipe and before it creates or loads a profile: the point
-- where a new login begins.
ActionProfileMod.StarterHotbar.stockRefreshProfileTemplateCombo = ActionProfileMod.refreshProfileTemplateCombo
function ActionProfileMod.refreshProfileTemplateCombo()
    local SH = ActionProfileMod.StarterHotbar
    SH.newLoadToken()
    SH.lastPoll = nil
    SH.listErrorLogged = false
    return SH.stockRefreshProfileTemplateCombo()
end

--=============================================================================
-- Mark the first profile a character gets as fresh. The client creates it
-- when GActionProfiles is empty: no saved UI variables for this character on
-- this machine, or the stock version-2 wipe. A profile made from the
-- editor's New Profile button is never first.
ActionProfileMod.StarterHotbar.stockCreateProfile = ActionProfileMod.createProfile
function ActionProfileMod.createProfile( profileName, templateId )
    local SH = ActionProfileMod.StarterHotbar
    local first = ( GActionProfiles == nil ) or ( next(GActionProfiles) == nil )
    local profileId = SH.stockCreateProfile( profileName, templateId )
    if first and profileId and profileId > 0 and type(GActionProfiles[profileId]) == 'table' then
        GActionProfiles[profileId][SH.StateKey] = SH.Pending
        GActionProfiles[profileId][SH.LoadKey] = SH.currentLoad()
    end
    return profileId
end

--=============================================================================
-- After the stock load (and any UI pack's layout migration inside it),
-- place what the client already knows.
ActionProfileMod.StarterHotbar.stockLoadProfile = ActionProfileMod.loadProfile
function ActionProfileMod.loadProfile( profileId )
    ActionProfileMod.StarterHotbar.stockLoadProfile( profileId )
    pcall( ActionProfileMod.StarterHotbar.run, 'profile loaded' )
end

--=============================================================================
-- The known abilities usually arrive after the module loads.
function ActionProfileMod.onStarterHotbarAbilityUpdate( this, groupId, abilityId )
    if groupId ~= nil and UIAbilityGroup and UIAbilityGroup.KnownAbility
       and groupId ~= UIAbilityGroup.KnownAbility then
        return
    end
    pcall( ActionProfileMod.StarterHotbar.run, 'ability update' )
end

-- Fallback trigger, the player's own updates only, at most once a second, in
-- case the known list changes without an Events.AbilityUpdate.
function ActionProfileMod.onStarterHotbarPropertyUpdated( this, unitId )
    local SH = ActionProfileMod.StarterHotbar
    if not SH.watching() then
        return
    end
    if type(unitsEqual) == 'function' and Unit and Unit.Player and type(unitId) == 'number' then
        local ok, mine = pcall( unitsEqual, Unit.Player, unitId )
        if ok and not mine then
            return
        end
    end
    if type(getSystemTime) == 'function' then
        local ok, now = pcall( getSystemTime )
        if ok and type(now) == 'number' then
            if SH.lastPoll and now >= SH.lastPoll and now - SH.lastPoll < SH.PollSeconds then
                return
            end
            SH.lastPoll = now
        end
    end
    pcall( SH.run, 'property update' )
end
