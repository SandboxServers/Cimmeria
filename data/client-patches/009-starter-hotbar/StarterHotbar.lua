

--=============================================================================
-- Cimmeria client patch 009-starter-hotbar. Cimmeria code, not CME's.
--
-- A new character's action bar is client state only (GActionProfiles, a
-- CharacterVariable of the ActionButtons module), so the server cannot put
-- the character's starting abilities on it. The patch appends this block to
-- the stock ActionProfileDefault1.lua, which ActionButtons.toc loads after
-- ActionProfiles.lua. It places the starting abilities the character knows
-- on the empty layer-bound buttons (11-20) of the profile the client creates
-- at the character's first login, and never touches any other profile.
--
-- It only binds actions to buttons that already exist. It never creates,
-- moves or resizes a button, and it changes no file a UI pack replaces: it
-- wraps ActionProfileMod.createProfile and ActionProfileMod.loadProfile, which
-- the stock ActionProfiles.lua and the WQHD v26 replacement both define.
--
-- State, kept in the profile (persisted with GActionProfiles):
--   nil        a profile this patch did not create: never touched
--   'pending'  created fresh at first login; nothing placed yet
--   'seeding'  placed abilities this session; tops up as more arrive
--   'done'     finished; never runs again for this profile
-- 'seeding' turns into 'done' at the next load of the module (the next login),
-- so a starting ability that arrives late in the first session still lands,
-- and nothing is ever added in a later session.
--=============================================================================
ActionProfileMod.StarterHotbar = {
    -- Attacks first, then heals: Pistol Shot, Strike, Heal Focus,
    -- Health Heal, Medical Attention: Recuperation.
    Abilities = { 592, 594, 597, 1646, 1218 },
    FirstButton = 11,
    LastButton = 20,
    StateKey = 'cimmeriaStarterHotbar',
    Pending = 'pending',
    Seeding = 'seeding',
    Done = 'done',
    PollSeconds = 1.0,

    sessionActive = false,  -- this Lua session placed something
    busy = false,
    lastPoll = nil,
}

--=============================================================================
function ActionProfileMod.StarterHotbar.log( text )
    pcall( function() Debug:log( '[cimmeria] starter hotbar: '..text ) end )
end

--=============================================================================
-- The ids in the client's known-abilities cache (filled by
-- onKnownAbilitiesUpdate), or nil when the client has no such list.
-- getAbilityList is a native the stock UI never calls; it ignores its
-- argument and returns the known-ability ids as an array.
function ActionProfileMod.StarterHotbar.knownAbilities()
    if type(getAbilityList) ~= 'function' then
        return nil
    end
    local group = 2
    if UIAbilityGroup and UIAbilityGroup.KnownAbility then
        group = UIAbilityGroup.KnownAbility
    end
    local ok, list = pcall( getAbilityList, group )
    if not ok or type(list) ~= 'table' then
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
-- A layer-bound button of the profile with no action on the current layer.
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
-- Returns placed, missing (not known), full (no button or action left),
-- knownCount (nil when the client has no known-ability list).
function ActionProfileMod.StarterHotbar.seed( profile )
    local SH = ActionProfileMod.StarterHotbar
    local known, knownCount = SH.knownAbilities()
    if not known then
        return 0, #SH.Abilities, false, nil
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
    return placed, missing, full, knownCount
end

--=============================================================================
-- Is the current profile one this session may still add to?
function ActionProfileMod.StarterHotbar.watching()
    local SH = ActionProfileMod.StarterHotbar
    local profile = GActionProfiles and GActionProfiles[GActionCurrentProfileId]
    if type(profile) ~= 'table' then
        return false
    end
    local state = profile[SH.StateKey]
    return state == SH.Pending or ( state == SH.Seeding and SH.sessionActive )
end

--=============================================================================
function ActionProfileMod.StarterHotbar.run( reason )
    local SH = ActionProfileMod.StarterHotbar
    if SH.busy then
        return
    end
    local profile = GActionProfiles and GActionProfiles[GActionCurrentProfileId]
    if type(profile) ~= 'table' then
        return
    end
    local state = profile[SH.StateKey]
    if state == SH.Seeding and not SH.sessionActive then
        profile[SH.StateKey] = SH.Done
        SH.log( 'done: seeded in an earlier session' )
        return
    end
    if not SH.watching() then
        return
    end

    SH.busy = true
    local ok, placed, missing, full, knownCount = pcall( SH.seed, profile )
    SH.busy = false

    if not ok then
        -- Never retry a failing seed on every event; the bar stays as it is.
        profile[SH.StateKey] = SH.Done
        SH.log( 'error ('..tostring(reason)..'): '..tostring(placed) )
        return
    end

    if placed > 0 then
        if state == SH.Pending and type(writeLocalFeedback) == 'function' then
            pcall( writeLocalFeedback, 'Your starting abilities are on your action bar.' )
        end
        profile[SH.StateKey] = SH.Seeding
        SH.sessionActive = true
        SH.log( 'placed '..placed..' ('..tostring(reason)..')' )
    end

    if missing == 0 or full then
        profile[SH.StateKey] = SH.Done
        SH.log( 'done: '..(full and 'no empty button or action left' or 'all starting abilities on the bar') )
    elseif placed == 0 and state == SH.Pending and knownCount and knownCount > 0 then
        -- The known list has arrived and holds none of the starting abilities.
        profile[SH.StateKey] = SH.Done
        SH.log( 'done: no starting ability known' )
    end
end

--=============================================================================
-- Mark the first profile a character gets as fresh. The client creates it
-- when GActionProfiles is empty: a new character, or the stock version-2
-- wipe. A profile made from the editor's New Profile button is never first.
ActionProfileMod.StarterHotbar.stockCreateProfile = ActionProfileMod.createProfile
function ActionProfileMod.createProfile( profileName, templateId )
    local SH = ActionProfileMod.StarterHotbar
    local first = ( GActionProfiles == nil ) or ( next(GActionProfiles) == nil )
    local profileId = SH.stockCreateProfile( profileName, templateId )
    if first and profileId and profileId > 0 and type(GActionProfiles[profileId]) == 'table' then
        GActionProfiles[profileId][SH.StateKey] = SH.Pending
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

-- Fallback trigger, throttled, in case the known list changes without an
-- Events.AbilityUpdate the patch can see.
function ActionProfileMod.onStarterHotbarPropertyUpdated( this )
    local SH = ActionProfileMod.StarterHotbar
    if not SH.watching() then
        return
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

if ActionButtonsWin and Events then
    if Events.AbilityUpdate then
        ActionButtonsWin:subscribe( Events.AbilityUpdate, 'ActionProfileMod.onStarterHotbarAbilityUpdate' )
    end
    if Events.PropertyUpdated then
        ActionButtonsWin:subscribe( Events.PropertyUpdated, 'ActionProfileMod.onStarterHotbarPropertyUpdated' )
    end
end
