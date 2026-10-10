

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
--    (cimmeriaStarterPlaced), so one the player removed stays removed.
--    Only a profile 009 created (it carries 009's mark) is touched.
-- 2. Staff Swing (1984), the Free Jaffa's signature attack, joins the
--    starting abilities, after Strike, for 009's first-session seeding too.
-- 3. A press on a shot button with no shot to fire says so. With a weapon
--    that has no ranged basic attack (a knife, say) the known list holds no
--    weapon shot, 015 leaves the button on the last weapon's shot, and the
--    client drops the press of an ability it does not know without a word.
--    This block then writes the server's own line for a right-click in that
--    state, "This weapon has no ranged attack.", at most once a second. The
--    button keeps its shot, for when a gun comes back out.
--
-- It subscribes to nothing. It runs after 015's runner, which 015 calls on
-- every known-abilities update, weapon switch and profile load, and it
-- watches presses through useAction, the native the stock button handler
-- looks up on every press (click or key binding).
--=============================================================================
ActionProfileMod.StarterLearned = {
    StaffSwing = 1984,
    Strike = 594,           -- Staff Swing goes after it in 009's list
    PlacedKey = 'cimmeriaStarterPlaced',
    Placed = 1,             -- was on the bar (placed by 009, by this block or by the player)
    NoRoom = 2,             -- known while buttons 11-20 were all taken; not placed

    busy = false,
    failed = false,
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
    local onBar = SH.abilitiesOnBar( profile )
    for _, abilityId in ipairs( SH.Abilities ) do
        if onBar[abilityId] then
            set[abilityId] = SL.Placed
        end
    end
    if SH.watching() then
        -- The first session belongs to 009.
        return 0
    end
    local known = SH.knownAbilities()
    if not known then
        return 0
    end

    local placed = 0
    local buttonId = SH.FirstButton
    for _, abilityId in ipairs( SH.Abilities ) do
        if set[abilityId] == nil and known[abilityId] then
            while buttonId <= SH.LastButton and not SH.isEmptyLayerButton( profile, buttonId ) do
                buttonId = buttonId + 1
            end
            if buttonId > SH.LastButton or not SH.place( buttonId, abilityId ) then
                set[abilityId] = SL.NoRoom
                SL.log( 'ability '..abilityId..' learned with no empty button 11-'..SH.LastButton..'; not placed' )
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
end

--=============================================================================
-- Every ability 009 or this block places is recorded in the profile at once,
-- so one the player removes before the next look is not placed again.
ActionProfileMod.StarterLearned.previousPlace = ActionProfileMod.StarterHotbar.place
function ActionProfileMod.StarterHotbar.place( buttonId, abilityId )
    local SL = ActionProfileMod.StarterLearned
    local done = SL.previousPlace( buttonId, abilityId )
    if done then
        pcall( function()
            local set = SL.placedSet( ActionProfileMod.StarterHotbar.currentProfile() )
            if set then
                set[abilityId] = SL.Placed
            end
        end )
    end
    return done
end

-- 015 runs its swap on every known-abilities update, weapon switch and
-- profile load; look after it, so a shot it has just moved counts as Pistol
-- Shot on the bar.
ActionProfileMod.StarterLearned.previousShotRun = ActionProfileMod.WeaponShotBar.run
function ActionProfileMod.WeaponShotBar.run( reason )
    ActionProfileMod.StarterLearned.previousShotRun( reason )
    pcall( ActionProfileMod.StarterLearned.run, reason )
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
-- and the known list holds no weapon shot at all.
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
-- press still goes to the client as before; this only adds the line.
if type(useAction) == 'function' then
    ActionProfileMod.NoShotFeedback.previousUseAction = useAction
    function useAction( ... )
        pcall( ActionProfileMod.NoShotFeedback.check, (...) )
        return ActionProfileMod.NoShotFeedback.previousUseAction( ... )
    end
end
