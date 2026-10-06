

--=============================================================================
-- Cimmeria client patch 015-weapon-shot-bar. Cimmeria code, not CME's.
--
-- The action bar is client state only (GActionProfiles), so the server
-- cannot change a button. When the player switches weapons the server swaps
-- the weapon's basic attacks in the known-abilities list, but a button that
-- holds the old weapon's shot keeps it, and the server refuses that shot
-- with the new weapon. This block, appended after 009's, makes such a button
-- follow the weapon: it points the button's action at the shot of the weapon
-- now held.
--
-- "The shot" is the one ranged basic attack in the known list (WeaponShots
-- below; the server keeps exactly one there, the active weapon's). Pistol
-- Shot (592) is a trained ability that a pistol also fires: with a pistol out
-- it is left alone, and it is what a button returns to when the player comes
-- back to a pistol.
--
-- What it never does: add, move, resize or clear a button; touch a button
-- that is not holding a shot; touch a bandolier-bound button (the stock UI
-- already keeps one action per weapon on those); or act when the known list
-- holds no shot or more than one (a blade, an unarmed character, a GM who
-- was given every ability).
--
-- It changes a button the way dropping an ability on it does: the button's
-- action keeps its id and is pointed at the new ability (setActionToAbility).
--
-- It leaves 009's code as it is and adds one rule to its seeding (at the end
-- of this block). 009 subscribes ActionButtonsWin to
-- Events.AbilityUpdate and Events.PropertyUpdated and unsubscribes it when
-- it is done, so this block listens on another window of the module:
-- ActionButton_DragContainer, which the stock layout marks
-- DeafWhenHidden=False (a hidden window without that property gets no
-- events) and which nothing subscribes to these three events. It wraps
-- loadProfile on top of 009's wrapper, calling it first.
--=============================================================================
ActionProfileMod.WeaponShotBar = {
    -- The ranged basic attack a weapon grants while it is the active weapon:
    -- Automatic Weapon, Pistol, HMG, Rifle, LMG, Shotgun, Staff, Ribbon
    -- Device, Dart Pistol, Drone, Zat, Staff.
    WeaponShots = { 559, 579, 580, 581, 582, 583, 584, 712, 1086, 1216, 1414, 2635 },
    PistolShot = 592,       -- trained; needs a pistol
    PistolBasic = 579,      -- the pistol's own basic attack
    WindowName = 'ActionButton_DragContainer',
    ToldKey = 'cimmeriaWeaponShotBar',
    PollSeconds = 1.0,
    WatchSeconds = 20.0,

    isShot = {},            -- id -> true for WeaponShots and PistolShot
    isWeaponShot = {},      -- id -> true for WeaponShots
    busy = false,
    failed = false,
    hooked = false,
    polling = false,
    watchUntil = nil,
    lastPoll = nil,
    listErrorLogged = false,
    windowErrorLogged = false,
}

for _, id in ipairs( ActionProfileMod.WeaponShotBar.WeaponShots ) do
    ActionProfileMod.WeaponShotBar.isShot[id] = true
    ActionProfileMod.WeaponShotBar.isWeaponShot[id] = true
end
ActionProfileMod.WeaponShotBar.isShot[ActionProfileMod.WeaponShotBar.PistolShot] = true

--=============================================================================
function ActionProfileMod.WeaponShotBar.log( text )
    pcall( function() Debug:log( '[cimmeria] weapon shot bar: '..text ) end )
end

--=============================================================================
function ActionProfileMod.WeaponShotBar.now()
    if type(getSystemTime) ~= 'function' then
        return nil
    end
    local ok, t = pcall( getSystemTime )
    if ok and type(t) == 'number' then
        return t
    end
    return nil
end

--=============================================================================
-- The ids in the client's known-abilities cache, or nil when it cannot be
-- read. getAbilityList is a zero-argument native (see 009).
function ActionProfileMod.WeaponShotBar.knownAbilities()
    local WS = ActionProfileMod.WeaponShotBar
    if type(getAbilityList) ~= 'function' then
        return nil
    end
    local ok, list = pcall( getAbilityList )
    if not ok or type(list) ~= 'table' then
        if not WS.listErrorLogged then
            WS.listErrorLogged = true
            WS.log( 'getAbilityList failed: '..tostring(list) )
        end
        return nil
    end
    local known = {}
    for _, entry in pairs(list) do
        local id = entry
        if type(entry) == 'table' then
            id = entry.id
        end
        if type(id) == 'number' then
            known[id] = true
        end
    end
    return known
end

--=============================================================================
-- The active weapon's shot: the only weapon shot in the known list. nil when
-- there is none, or more than one and so no way to tell which is held.
function ActionProfileMod.WeaponShotBar.activeShot( known )
    local WS = ActionProfileMod.WeaponShotBar
    local shot, count = nil, 0
    for _, id in ipairs( WS.WeaponShots ) do
        if known[id] then
            shot = id
            count = count + 1
        end
    end
    if count ~= 1 then
        return nil
    end
    return shot
end

-- Can the weapon whose shot is `shot` fire abilityId?
function ActionProfileMod.WeaponShotBar.fits( abilityId, shot )
    local WS = ActionProfileMod.WeaponShotBar
    return abilityId == shot
        or ( shot == WS.PistolBasic and abilityId == WS.PistolShot )
end

-- What a button should hold for that weapon.
function ActionProfileMod.WeaponShotBar.targetFor( known, shot )
    local WS = ActionProfileMod.WeaponShotBar
    if shot == WS.PistolBasic and known[WS.PistolShot] then
        return WS.PistolShot
    end
    return shot
end

--=============================================================================
function ActionProfileMod.WeaponShotBar.currentProfile()
    local profile = GActionProfiles and GActionProfiles[GActionCurrentProfileId]
    if type(profile) ~= 'table' or type(profile.buttonInfo) ~= 'table' then
        return nil
    end
    return profile
end

--=============================================================================
-- Point every action of the current profile that holds a shot the active
-- weapon cannot fire at the shot it can. Returns the number changed.
function ActionProfileMod.WeaponShotBar.follow()
    local WS = ActionProfileMod.WeaponShotBar
    local profile = WS.currentProfile()
    if not profile or type(getActionInfo) ~= 'function' or type(setActionToAbility) ~= 'function' then
        return 0
    end
    local known = WS.knownAbilities()
    if not known then
        return 0
    end
    local shot = WS.activeShot( known )
    if not shot then
        return 0
    end
    local target = WS.targetFor( known, shot )

    -- Actions of bandolier-bound buttons belong to the stock per-weapon swap.
    local perWeapon = {}
    for _, info in pairs( profile.buttonInfo ) do
        if type(info) == 'table' and type(info.actions) == 'table'
           and info.binding == ActionProfileMod.ButtonBinding_Bandolier then
            for _, actionId in pairs( info.actions ) do
                perWeapon[actionId] = true
            end
        end
    end

    local seen = {}
    local changed = 0
    for _, info in pairs( profile.buttonInfo ) do
        if type(info) == 'table' and type(info.actions) == 'table'
           and info.binding ~= ActionProfileMod.ButtonBinding_Bandolier then
            for _, actionId in pairs( info.actions ) do
                if type(actionId) == 'number' and actionId > 0
                   and not seen[actionId] and not perWeapon[actionId] then
                    seen[actionId] = true
                    local ok, actionInfo = pcall( getActionInfo, actionId )
                    if ok and type(actionInfo) == 'table' and actionInfo.id
                       and ( ActionType == nil or actionInfo.type == ActionType.Ability )
                       and WS.isShot[actionInfo.subId]
                       and not WS.fits( actionInfo.subId, shot ) then
                        local old = actionInfo.subId
                        setActionToAbility( actionId, target )
                        -- Redraw now, without waiting for Events.ActionUpdated.
                        if ActionButtonMod and type(ActionButtonMod.onActionUpdated) == 'function' then
                            pcall( ActionButtonMod.onActionUpdated, ActionButtonsWin, actionId )
                        end
                        changed = changed + 1
                        WS.log( 'action '..actionId..': ability '..tostring(old)..' -> '..target )
                    end
                end
            end
        end
    end

    if changed > 0 and profile[WS.ToldKey] == nil then
        profile[WS.ToldKey] = 'told'
        pcall( writeLocalFeedback, 'Your weapon shot button now follows the weapon you hold.' )
    end
    return changed
end

--=============================================================================
function ActionProfileMod.WeaponShotBar.run( reason )
    local WS = ActionProfileMod.WeaponShotBar
    if WS.busy or WS.failed then
        return
    end
    WS.busy = true
    local ok, changed = pcall( WS.follow )
    WS.busy = false
    if not ok then
        -- Never retry a failing swap on every event; the bar stays as it is.
        WS.failed = true
        WS.log( 'error ('..tostring(reason)..'), switched off: '..tostring(changed) )
    elseif changed > 0 then
        WS.log( 'changed '..changed..' ('..tostring(reason)..')' )
    end
end

--=============================================================================
-- The window this block listens on, or nil.
function ActionProfileMod.WeaponShotBar.window()
    local WS = ActionProfileMod.WeaponShotBar
    local win = _G[WS.WindowName]
    if win == nil or type(win.subscribe) ~= 'function' then
        if not WS.windowErrorLogged then
            WS.windowErrorLogged = true
            WS.log( 'no window '..WS.WindowName..'; following on weapon switch and profile load only' )
        end
        return nil
    end
    return win
end

-- Listen for the weapon switch and for the known list changing.
function ActionProfileMod.WeaponShotBar.hook()
    local WS = ActionProfileMod.WeaponShotBar
    if WS.hooked or not ( Events and Events.AbilityUpdate and Events.InventoryUpdateContainerActiveSlot ) then
        return
    end
    local win = WS.window()
    if not win then
        return
    end
    win:subscribe( Events.InventoryUpdateContainerActiveSlot, 'ActionProfileMod.onWeaponShotBarSlotActivated' )
    win:subscribe( Events.AbilityUpdate, 'ActionProfileMod.onWeaponShotBarAbilityUpdate' )
    WS.hooked = true
end

--=============================================================================
-- For WatchSeconds after a weapon switch or a profile load, also check on the
-- player's own property updates, in case the known list changes without an
-- Events.AbilityUpdate (the same fallback 009 uses).
function ActionProfileMod.WeaponShotBar.watch()
    local WS = ActionProfileMod.WeaponShotBar
    local now = WS.now()
    if not now or not ( Events and Events.PropertyUpdated ) then
        return
    end
    WS.watchUntil = now + WS.WatchSeconds
    if not WS.polling then
        local win = WS.window()
        if win then
            win:subscribe( Events.PropertyUpdated, 'ActionProfileMod.onWeaponShotBarPropertyUpdated' )
            WS.polling = true
        end
    end
end

function ActionProfileMod.WeaponShotBar.stopWatching()
    local WS = ActionProfileMod.WeaponShotBar
    WS.watchUntil = nil
    if WS.polling then
        WS.polling = false
        local win = WS.window()
        if win then
            win:unsubscribe( Events.PropertyUpdated )
        end
    end
end

--=============================================================================
function ActionProfileMod.onWeaponShotBarSlotActivated( this, containerId )
    if Container and Container.Bandolier and containerId ~= nil and containerId ~= Container.Bandolier then
        return
    end
    pcall( ActionProfileMod.WeaponShotBar.watch )
    pcall( ActionProfileMod.WeaponShotBar.run, 'weapon switched' )
end

function ActionProfileMod.onWeaponShotBarAbilityUpdate( this, groupId, abilityId )
    if groupId ~= nil and UIAbilityGroup and UIAbilityGroup.KnownAbility
       and groupId ~= UIAbilityGroup.KnownAbility then
        return
    end
    pcall( ActionProfileMod.WeaponShotBar.run, 'ability update' )
end

function ActionProfileMod.onWeaponShotBarPropertyUpdated( this, unitId )
    local WS = ActionProfileMod.WeaponShotBar
    local now = WS.now()
    if not now or not WS.watchUntil or now > WS.watchUntil then
        pcall( WS.stopWatching )
        return
    end
    if type(unitsEqual) == 'function' and Unit and Unit.Player and type(unitId) == 'number' then
        local ok, mine = pcall( unitsEqual, Unit.Player, unitId )
        if ok and not mine then
            return
        end
    end
    if WS.lastPoll and now >= WS.lastPoll and now - WS.lastPoll < WS.PollSeconds then
        return
    end
    WS.lastPoll = now
    pcall( WS.run, 'property update' )
end

--=============================================================================
-- The stock handler of the weapon switch refreshes the bandolier-bound
-- buttons through this function. Following from here as well does not depend
-- on this block's own subscription reaching the window.
if type(ActionProfileMod.refreshCurrentProfile) == 'function' then
    ActionProfileMod.WeaponShotBar.previousRefreshCurrentProfile = ActionProfileMod.refreshCurrentProfile
    function ActionProfileMod.refreshCurrentProfile( bindingType )
        ActionProfileMod.WeaponShotBar.previousRefreshCurrentProfile( bindingType )
        if bindingType == ActionProfileMod.ButtonBinding_Bandolier then
            pcall( ActionProfileMod.WeaponShotBar.watch )
            pcall( ActionProfileMod.WeaponShotBar.run, 'bandolier refresh' )
        end
    end
end

--=============================================================================
-- After the profile is loaded (the stock load, then 009's seeding), follow
-- the weapon the character logged in with.
ActionProfileMod.WeaponShotBar.previousLoadProfile = ActionProfileMod.loadProfile
function ActionProfileMod.loadProfile( profileId )
    ActionProfileMod.WeaponShotBar.previousLoadProfile( profileId )
    pcall( ActionProfileMod.WeaponShotBar.hook )
    pcall( ActionProfileMod.WeaponShotBar.watch )
    pcall( ActionProfileMod.WeaponShotBar.run, 'profile loaded' )
end

--=============================================================================
-- 009 seeds a new character's bar during its first session, and for a Human
-- that session never finishes seeding (Health Heal is never known). Without
-- this, 009 would put a second Pistol Shot on the bar as soon as this block
-- had pointed the first one at another weapon's shot. A weapon shot on the
-- bar therefore counts as Pistol Shot being there. 009's own code is not
-- changed; its seed reads the bar through this function.
if ActionProfileMod.StarterHotbar and type(ActionProfileMod.StarterHotbar.abilitiesOnBar) == 'function' then
    ActionProfileMod.WeaponShotBar.starterAbilitiesOnBar = ActionProfileMod.StarterHotbar.abilitiesOnBar
    function ActionProfileMod.StarterHotbar.abilitiesOnBar( profile )
        local WS = ActionProfileMod.WeaponShotBar
        local onBar = WS.starterAbilitiesOnBar( profile )
        for id in pairs( WS.isWeaponShot ) do
            if onBar[id] then
                onBar[WS.PistolShot] = true
            end
        end
        return onBar
    end
end

pcall( ActionProfileMod.WeaponShotBar.hook )
