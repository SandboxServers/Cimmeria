

--=============================================================================
-- Cimmeria client patch 016-hotbar-learn-and-feedback. Cimmeria code, not
-- CME's.
--
-- Appended after 009's and 015's blocks, whose bytes it leaves as they are.
-- It changes three things, all through the tables those blocks expose:
--
-- 1. Starting abilities learned after the first session reach the bar.
--    009 seeds only during the module load that created the profile. Since
--    Class Start v6 the starting abilities are granted later, by content
--    (the CellBlock guard's corpse, the SGC firearm), so a character that
--    logged in once before that point learned them with an empty bar. This
--    block places each starting ability the first time it is known, in any
--    session, on an empty layer-bound button 11-20, as 009 does. It never
--    replaces a button, and it never places an ability again once it has
--    been on the bar: the profile keeps the ids it has dealt with
--    (cimmeriaStarterPlaced), and 009 counts those as on the bar too, so one
--    the player removed stays removed. Only a profile 009 created (it
--    carries 009's mark) is touched.
-- 2. Staff Swing (1984), the Free Jaffa's signature attack, joins the
--    starting abilities, after Strike, for 009's first-session seeding too.
--    Only a Free Jaffa knows it, so 009 does not wait for it to finish.
-- 3. A press on a shot button with no shot to fire says so. With a weapon
--    that has no ranged basic attack (a knife, say) the known list holds no
--    weapon shot, 015 leaves the button on the last weapon's shot, and the
--    client drops the press of an ability it does not know without a word.
--    This block then writes the server's own line for a right-click in that
--    state, "This weapon has no ranged attack.", at most once a second. The
--    button keeps its shot, for when a gun comes back out.
--
-- When it looks: after 015's runner, which 015 calls on every
-- known-abilities update, weapon switch and profile load; and, while a
-- starting ability is still unrecorded and 009 is not seeding, on the
-- player's own property updates at most every PollSeconds, in case a grant
-- reaches the list with no Events.AbilityUpdate (009's fallback, for the
-- whole session). It watches presses through useAction, the native the
-- stock button handler looks up on every press (click or key binding).
--
-- Running this block a second time in one Lua state wraps nothing twice.
--=============================================================================
ActionProfileMod.StarterLearned = {
    StaffSwing = 1984,
    Strike = 594,           -- Staff Swing goes after it in 009's list
    PlacedKey = 'cimmeriaStarterPlaced',
    Placed = 1,             -- was on the bar (placed by 009, by this block or by the player)
    NoRoom = 2,             -- known while buttons 11-20 were all taken; not placed
    PollSeconds = 2.0,

    busy = false,
    failed = false,
    polling = false,
    lastPoll = nil,
}

--=============================================================================
function ActionProfileMod.StarterLearned.log( text )
    pcall( function() Debug:log( '[cimmeria] starter learned: '..text ) end )
end

--=============================================================================
-- Add Staff Swing to 009's list, after Strike, once.
function ActionProfileMod.StarterLearned.addStaffSwing()
    local SH = ActionProfileMod.StarterHotbar
    local SL = ActionProfileMod.StarterLearned
    local list = SH.Abilities
    for _, id in ipairs( list ) do
        if id == SL.StaffSwing then
            return
        end
    end
    local at = #list + 1
    for i, id in ipairs( list ) do
        if id == SL.Strike then
            at = i + 1
        end
    end
    table.insert( list, at, SL.StaffSwing )
end

--=============================================================================
-- The ids this profile has dealt with, created on first use. nil for a
-- profile 009 did not create.
function ActionProfileMod.StarterLearned.placedSet( profile )
    local SH = ActionProfileMod.StarterHotbar
    local SL = ActionProfileMod.StarterLearned
    if type(profile) ~= 'table' or profile[SH.StateKey] == nil then
        return nil
    end
    local set = profile[SL.PlacedKey]
    if type(set) ~= 'table' then
        set = {}
        profile[SL.PlacedKey] = set
    end
    return set
end

--=============================================================================
-- The ability's name for the feedback line.
function ActionProfileMod.StarterLearned.name( abilityId )
    if type(getAbilityInfo) == 'function' then
        local ok, info = pcall( getAbilityInfo, abilityId )
        if ok and type(info) == 'table' and type(info.name) == 'string' and info.name ~= '' then
            return info.name
        end
    end
    return 'A new ability'
end

--=============================================================================
-- Does the current profile still have a starting ability it has not dealt
-- with? Decides whether the property-update fallback is worth keeping.
function ActionProfileMod.StarterLearned.anyUnrecorded()
    local SH = ActionProfileMod.StarterHotbar
    local SL = ActionProfileMod.StarterLearned
    local set = SL.placedSet( SH.currentProfile() )
    if not set then
        return false
    end
    for _, abilityId in ipairs( SH.Abilities ) do
        if set[abilityId] == nil then
            return true
        end
    end
    return false
end

--=============================================================================
-- Record what is on the bar, and, unless 009 is still seeding this session,
-- place each known starting ability this profile has not dealt with yet.
-- Returns the number placed.
function ActionProfileMod.StarterLearned.seed()
    local SH = ActionProfileMod.StarterHotbar
    local SL = ActionProfileMod.StarterLearned
    local profile = SH.currentProfile()
    local set = SL.placedSet( profile )
    if not set then
        return 0
    end
    local known = SH.knownAbilities()
    if not known then
        return 0
    end
    local any = false
    for _, abilityId in ipairs( SH.Abilities ) do
        if set[abilityId] == nil and known[abilityId] then
            any = true
        end
    end
    if not any then
        return 0
    end

    -- The bar as it is, without this block's rule that a recorded id counts
    -- as on the bar.
    local onBar = SH.cimmeriaBarAbilities( profile )
    for _, abilityId in ipairs( SH.Abilities ) do
        if onBar[abilityId] then
            set[abilityId] = SL.Placed
        end
    end
    if SH.watching() then
        -- The first session belongs to 009.
        return 0
    end

    local placed = 0
    local buttonId = SH.FirstButton
    for _, abilityId in ipairs( SH.Abilities ) do
        if set[abilityId] == nil and known[abilityId] then
            while buttonId <= SH.LastButton and not SH.isEmptyLayerButton( profile, buttonId ) do
                buttonId = buttonId + 1
            end
            if buttonId > SH.LastButton then
                set[abilityId] = SL.NoRoom
                SL.log( 'ability '..abilityId..' learned with no empty button 11-'..SH.LastButton..'; not placed' )
            elseif not SH.place( buttonId, abilityId ) then
                -- getUnusedAction gave no action: the client's action table
                -- is full. Not recorded, so a later look tries again.
                SL.log( 'no free action for ability '..abilityId..'; will try again' )
                break
            else
                set[abilityId] = SL.Placed
                placed = placed + 1
                SL.log( 'placed ability '..abilityId..' on button '..buttonId )
                pcall( writeLocalFeedback, SL.name( abilityId )..' is on your action bar.' )
                buttonId = buttonId + 1
            end
        end
    end
    return placed
end

--=============================================================================
-- Keep the property-update fallback while it can still find something. 009
-- owns ActionButtonsWin's subscriptions while it is seeding (and removes all
-- of them when it is done), so this block subscribes only after that.
function ActionProfileMod.StarterLearned.updatePolling()
    local SH = ActionProfileMod.StarterHotbar
    local SL = ActionProfileMod.StarterLearned
    if not ( ActionButtonsWin and Events and Events.PropertyUpdated ) then
        return
    end
    if SH.watching() or SH.subscribed then
        SL.polling = false
        return
    end
    local want = not SL.failed and SL.anyUnrecorded()
    if want and not SL.polling then
        ActionButtonsWin:subscribe( Events.PropertyUpdated, 'ActionProfileMod.onStarterLearnedPropertyUpdated' )
        SL.polling = true
    elseif not want and SL.polling then
        ActionButtonsWin:unsubscribe( Events.PropertyUpdated )
        SL.polling = false
    end
end

--=============================================================================
function ActionProfileMod.StarterLearned.run( reason )
    local SL = ActionProfileMod.StarterLearned
    if SL.busy or SL.failed then
        return
    end
    SL.busy = true
    local ok, placed = pcall( SL.seed )
    SL.busy = false
    if not ok then
        -- Never retry a failing seed on every event; the bar stays as it is.
        SL.failed = true
        SL.log( 'error ('..tostring(reason)..'), switched off: '..tostring(placed) )
    elseif placed > 0 then
        SL.log( 'placed '..placed..' ('..tostring(reason)..')' )
    end
    pcall( SL.updatePolling )
end

-- The player's own updates only, at most once every PollSeconds.
function ActionProfileMod.onStarterLearnedPropertyUpdated( this, unitId )
    local SL = ActionProfileMod.StarterLearned
    if type(unitsEqual) == 'function' and Unit and Unit.Player and type(unitId) == 'number' then
        local ok, mine = pcall( unitsEqual, Unit.Player, unitId )
        if ok and not mine then
            return
        end
    end
    local now = ActionProfileMod.WeaponShotBar.now()
    if now then
        if SL.lastPoll and now >= SL.lastPoll and now - SL.lastPoll < SL.PollSeconds then
            return
        end
        SL.lastPoll = now
    end
    pcall( SL.run, 'property update' )
end

--=============================================================================
-- Wrap 009's and 015's functions once per table: a second run of this block
-- alone finds them already wrapped.
local SH = ActionProfileMod.StarterHotbar
local WS = ActionProfileMod.WeaponShotBar
if not SH.cimmeriaLearnedWrapped then
    SH.cimmeriaLearnedWrapped = true

    -- Every ability 009 or this block places is recorded in the profile at
    -- once, so one the player removes before the next look is not placed
    -- again.
    local previousPlace = SH.place
    function SH.place( buttonId, abilityId )
        local done = previousPlace( buttonId, abilityId )
        if done then
            pcall( function()
                local SL = ActionProfileMod.StarterLearned
                local set = SL.placedSet( ActionProfileMod.StarterHotbar.currentProfile() )
                if set then
                    set[abilityId] = SL.Placed
                end
            end )
        end
        return done
    end

    -- A recorded id counts as on the bar, so 009's seeding never puts back
    -- one the player removed and never waits for it.
    local previousOnBar = SH.abilitiesOnBar
    SH.cimmeriaBarAbilities = previousOnBar
    function SH.abilitiesOnBar( profile )
        local onBar = previousOnBar( profile )
        local set = type(profile) == 'table' and profile[ActionProfileMod.StarterLearned.PlacedKey]
        if type(set) == 'table' then
            for id in pairs( set ) do
                onBar[id] = true
            end
        end
        return onBar
    end

    -- Staff Swing is in the list but only a Free Jaffa knows it: do not let
    -- it keep 009 seeding (and listening) for everyone else's first session.
    local previousSeed = SH.seed
    function SH.seed( profile )
        local placed, missing, full = previousSeed( profile )
        if missing > 0 and not full then
            local SL = ActionProfileMod.StarterLearned
            local known = SH.knownAbilities()
            local onBar = SH.abilitiesOnBar( profile )
            if known and not known[SL.StaffSwing] and not onBar[SL.StaffSwing] then
                missing = missing - 1
            end
        end
        return placed, missing, full
    end
end

-- 015 runs its swap on every known-abilities update, weapon switch and
-- profile load; look after it, so a shot it has just moved counts as Pistol
-- Shot on the bar.
if not WS.cimmeriaLearnedWrapped then
    WS.cimmeriaLearnedWrapped = true
    local previousShotRun = WS.run
    function WS.run( reason )
        previousShotRun( reason )
        pcall( ActionProfileMod.StarterLearned.run, reason )
    end
end

pcall( ActionProfileMod.StarterLearned.addStaffSwing )

--=============================================================================
ActionProfileMod.NoShotFeedback = {
    Text = 'This weapon has no ranged attack.',
    RepeatSeconds = 1.0,

    lastTold = nil,
}

--=============================================================================
function ActionProfileMod.NoShotFeedback.log( text )
    pcall( function() Debug:log( '[cimmeria] no shot feedback: '..text ) end )
end

--=============================================================================
-- Is actionId a shot that cannot fire because the active weapon has none?
-- True when the action holds a weapon shot or Pistol Shot that is not known
-- and the known list holds no weapon shot at all (015's WeaponShots).
function ActionProfileMod.NoShotFeedback.noShotFor( actionId )
    local WS = ActionProfileMod.WeaponShotBar
    if type(actionId) ~= 'number' or actionId <= 0 or type(getActionInfo) ~= 'function' then
        return false
    end
    local ok, info = pcall( getActionInfo, actionId )
    if not ok or type(info) ~= 'table' or not info.id
       or ( ActionType ~= nil and info.type ~= ActionType.Ability )
       or not WS.isShot[info.subId] then
        return false
    end
    local known = WS.knownAbilities()
    if not known or known[info.subId] then
        return false
    end
    for id in pairs( WS.isWeaponShot ) do
        if known[id] then
            return false
        end
    end
    return true
end

function ActionProfileMod.NoShotFeedback.check( actionId )
    local NS = ActionProfileMod.NoShotFeedback
    if not NS.noShotFor( actionId ) then
        return
    end
    local now = ActionProfileMod.WeaponShotBar.now()
    if now and NS.lastTold and now >= NS.lastTold and now - NS.lastTold < NS.RepeatSeconds then
        return
    end
    NS.lastTold = now
    pcall( writeLocalFeedback, NS.Text )
    NS.log( 'action '..tostring(actionId)..' pressed with no weapon shot known' )
end

-- The stock button handler calls the global useAction on every press. The
-- press still goes to the client as before; this only adds the line. The
-- native is kept in an upvalue, and the global names the wrapper, so a
-- second run of this block does not wrap the wrapper.
if type(useAction) == 'function' and useAction ~= CimmeriaNoShotUseAction then
    local previousUseAction = useAction
    function CimmeriaNoShotUseAction( ... )
        pcall( ActionProfileMod.NoShotFeedback.check, (...) )
        return previousUseAction( ... )
    end
    useAction = CimmeriaNoShotUseAction
end
