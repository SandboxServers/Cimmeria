-- REPL-style logic UAT for client patch 015-weapon-shot-bar.
--
--   lua5.1 data/client-patches/015-weapon-shot-bar/test/run.lua
--   python data/client-patches/015-weapon-shot-bar/test/run_lupa.py   (no system lua5.1)
--
-- Boots the ActionButtons module in 009's stub client (its stubs.lua and
-- clean-room model, read from the 009 directory and not changed) with 009's
-- hook and then this patch's block appended to ActionProfileDefault1.lua, as
-- the two patches ship. It then switches weapons the way the server does: the
-- bandolier's active-slot event, then the known-abilities list losing the old
-- weapon's shot and gaining the new one.
--
--   model  the clean-room model of the module; always
--   stock  the client's own scripts; set SGW_UI_DIR to a client's
--          Working/SGWGame/Content/UI. A stock ActionProfileDefault1.lua gets
--          both hooks appended; one that already carries 009 (a
--          launcher-installed client) gets only this patch's block.
--
-- Set WEAPON_SHOT_BAR_REQUIRE_REAL=1 to fail when stock is skipped.
-- Set WEAPON_SHOT_BAR_APPEND to a later patch's block (016's
-- HotbarLearnAndFeedback.lua) to run these scenarios with that block
-- appended after this one, as a later patch ships it.
-- No CME file is read from the repo; the stock variant reads your client.

local here = ((arg and arg[0]) or ''):match('^(.*)[/\\][^/\\]*$') or '.'
local patchDir = here..'/..'
local starterDir = patchDir..'/../009-starter-hotbar'
local Stubs = dofile(starterDir..'/test/stubs.lua')

local HOOK9 = assert(Stubs.readFile(starterDir..'/StarterHotbar.lua'), '009 StarterHotbar.lua not found')
local HOOK15 = assert(Stubs.readFile(patchDir..'/WeaponShotBar.lua'), 'WeaponShotBar.lua not found')
local appendPath = os.getenv('WEAPON_SHOT_BAR_APPEND')
local APPENDED = ''
if appendPath and appendPath ~= '' then
    APPENDED = assert(Stubs.readFile(appendPath), 'WEAPON_SHOT_BAR_APPEND not found: '..appendPath)
    print('== with '..appendPath..' appended after this block')
end

local PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION = 592, 594, 597, 1218
local PISTOL, SMG, RIFLE, STAFF = 579, 559, 581, 584
local WINDOW = 'ActionButton_DragContainer'
local SLOT_EVENT = 'Events.InventoryUpdateContainerActiveSlot'

--=============================================================================
local function eq( actual, expected, what )
    if actual ~= expected then
        error((what or 'value')..': expected '..tostring(expected)..', got '..tostring(actual), 2)
    end
end
local function ok( cond, what )
    if not cond then
        error(what or 'check failed', 2)
    end
end
local function say( text )
    print('    > '..text)
end

--=============================================================================
-- Variants: name -> list of { chunkName, text } in .toc order.
local variants = {}

local MODEL = assert(Stubs.readFile(starterDir..'/test/fake_action_buttons.lua'))
variants[#variants + 1] = {
    name = 'model',
    scripts = function( with15 )
        return { { 'fake_action_buttons.lua', MODEL..HOOK9..(with15 and HOOK15..APPENDED or '') } }
    end,
}

local skipped = {}
local uiDir = os.getenv('SGW_UI_DIR')
if uiDir and uiDir ~= '' then
    local ab = uiDir..'/Core/ActionButtons/'
    local buttons = Stubs.readFile(ab..'ActionButtons.lua')
    local profiles = Stubs.readFile(ab..'ActionProfiles.lua')
    local default1 = Stubs.readFile(ab..'ActionProfileDefault1.lua')
    assert(buttons and profiles and default1, 'SGW_UI_DIR has no Core/ActionButtons scripts: '..uiDir)
    -- The file this patch's delta starts from: the stock file followed by 009.
    local base = default1
    if default1:find('ActionProfileMod.StarterHotbar', 1, true) then
        ok(default1:sub(-#HOOK9) == HOOK9, 'ActionProfileDefault1.lua carries a 009 block that is not the committed one')
    else
        base = default1..HOOK9
    end
    variants[#variants + 1] = {
        name = 'stock',
        scripts = function( with15 )
            return {
                { 'ActionButtons.lua', buttons },
                { 'ActionProfiles.lua', profiles },
                { 'ActionProfileDefault1.lua', base..(with15 and HOOK15..APPENDED or '') },
            }
        end,
    }
    -- The window the block listens on must hear events while hidden.
    local dragLayout = Stubs.readFile(ab..'ActionButtonDrag.layout')
    if dragLayout then
        local decl = dragLayout:match('Name="'..WINDOW..'">(.-)<Window') or ''
        ok(decl:find('Name="DeafWhenHidden" Value="False"', 1, true),
           WINDOW..' is not DeafWhenHidden=False in ActionButtonDrag.layout')
        print('== '..WINDOW..' is DeafWhenHidden=False in the client layout')
    end
else
    skipped[#skipped + 1] = 'stock (set SGW_UI_DIR)'
end

--=============================================================================
local variant

local function boot( opts, with15 )
    if with15 == nil then
        with15 = true
    end
    local client = Stubs.newClient(opts)
    for _, s in ipairs(variant.scripts(with15)) do
        client.load(s[2], s[1])
    end
    client.restore()
    client.fire('Events.ModuleLoaded')
    ok(client.dropCallbacks > 0, 'the module finished onModLoaded')
    return client
end

-- Simulate the player dropping an ability on a button (ActionProfileMod.receiveDrag).
local function drag( client, buttonId, abilityId )
    local env = client.env
    local actionId = env.ActionButtonMod.getActionForButton(buttonId)
    if not actionId or actionId < 1 then
        actionId = env.getUnusedAction()
        env.ActionProfileMod.setButtonCurrentAction(buttonId, actionId)
    end
    env.setActionToAbility(actionId, abilityId)
    return actionId
end

local function forget( client, abilityId )
    for i = #client.known, 1, -1 do
        if client.known[i] == abilityId then
            table.remove(client.known, i)
        end
    end
end

-- The server's weapon switch: the active slot changes, then
-- onKnownAbilitiesUpdate drops the old weapon's shot and adds the new one's.
-- how: nil      one Events.AbilityUpdate per change
--      'silent' the list changes with no event
local function switchWeapon( client, oldShot, newShot, how )
    local env = client.env
    client.fire(SLOT_EVENT, env.Container.Bandolier)
    if oldShot then
        forget(client, oldShot)
        if how ~= 'silent' then
            client.fire('Events.AbilityUpdate', env.UIAbilityGroup.KnownAbility, oldShot)
        end
    end
    if newShot then
        client.learn({ newShot }, how == 'silent' and 'silent' or nil)
    end
end

local function listens( client, ev )
    local w = client.windows[WINDOW]
    return w ~= nil and w.subs[ev] ~= nil
end

local function ourFeedback( client )
    local n = 0
    for _, line in ipairs(client.feedback) do
        if line:find('follows the weapon', 1, true) then
            n = n + 1
        end
    end
    return n
end

local function logged( client, text )
    for _, line in ipairs(client.log) do
        if line:find(text, 1, true) then
            return true
        end
    end
    return false
end

local function expectButton( client, buttonId, abilityId, what )
    eq(client.abilityOnButton(buttonId), abilityId, what..': ability on button '..buttonId)
    if abilityId then
        eq(client.iconOnButton(buttonId), 'set:Icons image:Ability'..abilityId, what..': icon on button '..buttonId)
        eq(client.press(buttonId), abilityId, what..': first press on button '..buttonId)
    end
end

local function deepEqual( a, b, path )
    if type(a) ~= type(b) then
        return false, path
    end
    if type(a) ~= 'table' then
        return a == b, path
    end
    for k, v in pairs(a) do
        local same, where = deepEqual(v, b[k], path..'.'..tostring(k))
        if not same then
            return false, where
        end
    end
    for k in pairs(b) do
        if a[k] == nil then
            return false, path..'.'..tostring(k)
        end
    end
    return true
end

-- A character past its first session: Pistol Shot, Strike and Heal Focus on
-- buttons 11-13 (as 009 left them), 009 done, holding a pistol.
local function veteran( extra )
    local first = boot({ known = { PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION, PISTOL } })
    local saved = first.save()
    local opts = { known = { PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION, PISTOL }, saved = saved }
    for k, v in pairs(extra or {}) do
        opts[k] = v
    end
    return boot(opts), saved
end

--=============================================================================
local scenarios = {}
local function scenario( name, fn )
    scenarios[#scenarios + 1] = { name = name, fn = fn }
end

scenario('009 still seeds a new character with this block appended', function()
    local c = boot({ known = { 9999, RECUPERATION, HEAL_FOCUS, PISTOL_SHOT, 1646, STRIKE, PISTOL } })
    for i, id in ipairs({ PISTOL_SHOT, STRIKE, HEAL_FOCUS, 1646, RECUPERATION }) do
        expectButton(c, 10 + i, id, '009 seeding')
    end
    eq(c.profile().cimmeriaStarterHotbar, 'done', '009 finished')
    ok(not c.subscribed('Events.AbilityUpdate'), '009 unsubscribed ActionButtonsWin')
    ok(listens(c, 'Events.AbilityUpdate'), 'this block still listens for the known list')
    ok(listens(c, SLOT_EVENT), 'this block still listens for the weapon switch')
    eq(ourFeedback(c), 0, 'nothing was swapped: a pistol fires Pistol Shot')
    say('five starters on 11-15, 009 done and unsubscribed, this block still listening on '..WINDOW)
end)

scenario('pistol to SMG: the Pistol Shot button becomes the SMG shot', function()
    local c = veteran()
    expectButton(c, 11, PISTOL_SHOT, 'before')
    switchWeapon(c, PISTOL, SMG)
    expectButton(c, 11, SMG, 'after the switch')
    expectButton(c, 12, STRIKE, 'after the switch')
    expectButton(c, 13, HEAL_FOCUS, 'after the switch')
    eq(ourFeedback(c), 1, 'one line the first time')
    ok(logged(c, 'ability 592 -> 559'), 'the swap is logged')
    say('button 11: 592 -> 559 with its icon, fires 559 on the first press; 12 and 13 untouched')
end)

scenario('back to a pistol: the button returns to Pistol Shot, or to the pistol basic attack', function()
    local c = veteran()
    switchWeapon(c, PISTOL, SMG)
    switchWeapon(c, SMG, PISTOL)
    expectButton(c, 11, PISTOL_SHOT, 'back on a pistol')
    eq(ourFeedback(c), 1, 'the line is not repeated')

    -- A character that never learned Pistol Shot.
    local d = veteran()
    forget(d, PISTOL_SHOT)
    switchWeapon(d, PISTOL, SMG)
    switchWeapon(d, SMG, PISTOL)
    expectButton(d, 11, PISTOL, 'back on a pistol without Pistol Shot')
    say('592 known: 559 -> 592; 592 unknown: 559 -> 579')
end)

scenario('the slot event alone changes nothing; the known-list update does', function()
    local c = veteran()
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    expectButton(c, 11, PISTOL_SHOT, 'slot event, list still names the pistol')
    forget(c, PISTOL)
    c.fire('Events.AbilityUpdate', c.env.UIAbilityGroup.KnownAbility, PISTOL)
    expectButton(c, 11, PISTOL_SHOT, 'no weapon shot in the list')
    c.learn({ SMG })
    expectButton(c, 11, SMG, 'the new shot arrived')
    say('stale list: no change; list with no shot: no change; list with the new shot: 592 -> 559')
end)

scenario('both shots in the list for a moment: nothing until one is left', function()
    local c = veteran()
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    c.learn({ RIFLE })
    expectButton(c, 11, PISTOL_SHOT, 'two weapon shots known')
    forget(c, PISTOL)
    c.fire('Events.AbilityUpdate', c.env.UIAbilityGroup.KnownAbility, PISTOL)
    expectButton(c, 11, RIFLE, 'one weapon shot known')
    say('579 and 581 known: no change; only 581 known: 592 -> 581')
end)

scenario('an ability update of another group is ignored', function()
    local c = veteran()
    forget(c, PISTOL)
    c.known[#c.known + 1] = SMG
    c.fire('Events.AbilityUpdate', c.env.UIAbilityGroup.Training, SMG)
    expectButton(c, 11, PISTOL_SHOT, 'a trainer-list update')
    c.fire('Events.AbilityUpdate', c.env.UIAbilityGroup.KnownAbility, SMG)
    expectButton(c, 11, SMG, 'a known-list update')
    say('group Training: no change; group KnownAbility: 592 -> 559')
end)

scenario('the list changes with no event: caught on a property update, for 20 s', function()
    local c = veteran()
    switchWeapon(c, PISTOL, SMG, 'silent')
    expectButton(c, 11, PISTOL_SHOT, 'no event yet')
    ok(listens(c, 'Events.PropertyUpdated'), 'watching after the weapon switch')
    c.advance(2)
    c.propertyUpdate(c.env.Unit.Target)
    expectButton(c, 11, PISTOL_SHOT, 'another unit is not a reason to look')
    c.propertyUpdate()
    expectButton(c, 11, SMG, 'the player\'s own update')

    -- Throttled to one look a second.
    local calls = c.listCalls
    c.propertyUpdate()
    c.propertyUpdate()
    eq(c.listCalls, calls, 'no second look within a second')

    -- After the watch ends the block stops listening to property updates.
    local d = veteran()
    d.fire(SLOT_EVENT, d.env.Container.Bandolier)
    d.advance(25)
    forget(d, PISTOL)
    d.known[#d.known + 1] = SMG
    d.propertyUpdate()
    expectButton(d, 11, PISTOL_SHOT, 'after the watch')
    ok(not listens(d, 'Events.PropertyUpdated'), 'unsubscribed after the watch')
    d.fire('Events.AbilityUpdate', d.env.UIAbilityGroup.KnownAbility, SMG)
    expectButton(d, 11, SMG, 'an ability update still works')
    say('silent list change: swapped on the player\'s property update within 20 s, once a second at most; later only on an event')
end)

scenario('no weapon shot (a blade, bare hands): the bar is left alone', function()
    local c = veteran()
    switchWeapon(c, PISTOL, nil)
    expectButton(c, 11, PISTOL_SHOT, 'no shot known')
    eq(ourFeedback(c), 0, 'no line')
    switchWeapon(c, nil, STAFF)
    expectButton(c, 11, STAFF, 'then a staff')
    say('no weapon shot: no change; staff: 592 -> 584')
end)

scenario('only shots move; a shot the weapon can fire stays', function()
    local c = veteran()
    drag(c, 14, PISTOL)            -- the pistol's basic attack, placed by the player
    drag(c, 15, RECUPERATION)
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    expectButton(c, 11, PISTOL_SHOT, 'pistol out')
    expectButton(c, 14, PISTOL, 'pistol out')
    switchWeapon(c, PISTOL, RIFLE)
    expectButton(c, 11, RIFLE, 'rifle out')
    expectButton(c, 14, RIFLE, 'rifle out')
    expectButton(c, 12, STRIKE, 'rifle out')
    expectButton(c, 13, HEAL_FOCUS, 'rifle out')
    expectButton(c, 15, RECUPERATION, 'rifle out')
    say('pistol: 592 and 579 both stay; rifle: both -> 581; Strike, Heal Focus, Recuperation never move')
end)

scenario('bandolier-bound buttons are left to the stock swap; never-bound and other layers follow', function()
    local c = veteran({ weaponItemId = 7001 })
    local env = c.env
    local mod = env.ActionProfileMod
    -- Buttons 16 and 17 are empty layer-bound buttons in every variant.
    c.profile().buttonInfo[16].binding = mod.ButtonBinding_Bandolier
    c.profile().buttonInfo[17].binding = mod.ButtonBinding_Never
    local bound = drag(c, 16, PISTOL_SHOT)
    drag(c, 17, PISTOL_SHOT)
    c.profile().currentLayer = 2
    mod.loadProfile(env.GActionCurrentProfileId)
    local layer2 = drag(c, 12, PISTOL_SHOT)     -- layer 2 of a layer-bound button
    c.profile().currentLayer = 1
    mod.loadProfile(env.GActionCurrentProfileId)

    switchWeapon(c, PISTOL, SMG)
    eq(c.actions[bound], PISTOL_SHOT, 'the bandolier-bound action')
    expectButton(c, 17, SMG, 'never-bound')
    eq(c.actions[layer2], SMG, 'the layer-2 action, not displayed')
    expectButton(c, 11, SMG, 'layer 1')
    expectButton(c, 12, STRIKE, 'layer 1')
    say('button 16 (bandolier-bound) keeps 592; button 17 (never-bound) and the hidden layer-2 action -> 559')
end)

scenario('logging in with a rifle out: followed when the profile loads, or when the list arrives', function()
    local _, saved = veteran()
    local c = boot({ known = { PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION, RIFLE }, saved = saved })
    expectButton(c, 11, RIFLE, 'known at load')

    local d = boot({ known = {}, saved = saved })
    expectButton(d, 11, PISTOL_SHOT, 'nothing known yet')
    d.learn({ PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION, RIFLE })
    expectButton(d, 11, RIFLE, 'the list arrived')
    say('list known at load: 592 -> 581 on profile load; list arriving later: on its ability updates')
end)

scenario('a relog keeps working and does not repeat the line', function()
    local c = veteran()
    switchWeapon(c, PISTOL, SMG)
    c.relogKeepingLuaState()
    expectButton(c, 11, SMG, 'after the relog')
    switchWeapon(c, SMG, PISTOL)
    expectButton(c, 11, PISTOL_SHOT, 'after the relog')
    switchWeapon(c, PISTOL, STAFF)
    expectButton(c, 11, STAFF, 'after the relog')
    eq(ourFeedback(c), 1, 'one line for the character')
    say('same Lua state, module loaded again: still follows, one feedback line in total')
end)

scenario('a new character switching weapons in its first session gets no second Pistol Shot from 009', function()
    -- A Human never knows Health Heal (1646), so 009 keeps seeding all session.
    local c = boot({ known = { PISTOL_SHOT, STRIKE, PISTOL } })
    eq(c.profile().cimmeriaStarterHotbar, 'seeding', '009 still seeding')
    expectButton(c, 11, PISTOL_SHOT, 'seeded')
    expectButton(c, 12, STRIKE, 'seeded')

    switchWeapon(c, PISTOL, SMG)
    expectButton(c, 11, SMG, 'first SMG')
    c.learn({ HEAL_FOCUS, RECUPERATION })       -- 009 tops the bar up
    expectButton(c, 13, HEAL_FOCUS, '009 top-up')
    expectButton(c, 14, RECUPERATION, '009 top-up')
    eq(c.abilityOnButton(15), nil, 'no second Pistol Shot')
    local shots = 0
    for b = 1, 100 do
        local id = c.abilityOnButton(b)
        if id == PISTOL_SHOT or id == SMG then
            shots = shots + 1
        end
    end
    eq(shots, 1, 'one shot button on the bar')

    switchWeapon(c, SMG, PISTOL)
    expectButton(c, 11, PISTOL_SHOT, 'back on the pistol')
    say('009 seeding: 592 -> 559 on button 11, later starters land on 13 and 14, and 592 is not seeded again')
end)

scenario('009 seeds Pistol Shot as before when no weapon shot is on the bar', function()
    local c = boot({ known = { STRIKE, SMG } })
    expectButton(c, 11, STRIKE, 'seeded')
    c.learn({ PISTOL_SHOT })
    local placed = c.abilityOnButton(12)
    ok(placed == PISTOL_SHOT or placed == SMG, '009 placed Pistol Shot on button 12, got '..tostring(placed))
    -- Which block hears the update first decides whether it is followed at
    -- once; the next look settles it either way.
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    expectButton(c, 12, SMG, 'followed')
    say('Strike on 11; Pistol Shot learned with an SMG out: 009 places it on 12, then 592 -> 559')
end)

scenario('the profile data and the button layout are never changed', function()
    local a = veteran()
    local before = a.save()
    switchWeapon(a, PISTOL, SMG)
    local after = a.save()
    after.GActionProfiles[after.GActionCurrentProfileId].cimmeriaWeaponShotBar = nil
    local same, where = deepEqual(before.GActionProfiles, after.GActionProfiles, 'GActionProfiles')
    ok(same, 'the profile changed at '..tostring(where))
    local n1, n2 = 0, 0
    for _ in pairs(before.actions) do n1 = n1 + 1 end
    for _ in pairs(after.actions) do n2 = n2 + 1 end
    eq(n2, n1, 'no action was created or cleared')
    say('GActionProfiles identical before and after a swap apart from the told mark; same number of actions')
end)

scenario('a failing setActionToAbility switches the block off without an error', function()
    local _, saved = veteran()
    local c = boot({ known = { PISTOL_SHOT, STRIKE, RIFLE }, saved = saved, failSetAction = true })
    expectButton(c, 11, PISTOL_SHOT, 'the swap failed')
    ok(logged(c, 'switched off'), 'the failure is logged')
    local calls = c.listCalls
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    c.fire('Events.AbilityUpdate', c.env.UIAbilityGroup.KnownAbility, RIFLE)
    -- An appended block may read the list on these events; the swap itself
    -- must still not be retried.
    if APPENDED == '' then
        eq(c.listCalls, calls, 'no retry on later events')
    end
    local errors = 0
    for _, line in ipairs(c.log) do
        if line:find('weapon shot bar: error', 1, true) then
            errors = errors + 1
        end
    end
    eq(errors, 1, 'the failing swap is not retried')
    expectButton(c, 12, STRIKE, 'the bar still works')
    say('setActionToAbility raises: logged once, no retry, the bar is as it was')
end)

scenario('a client with no getAbilityList, or one that raises: the block does nothing', function()
    local _, saved = veteran()
    local c = boot({ saved = saved, noAbilityList = true })
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    expectButton(c, 11, PISTOL_SHOT, 'no getAbilityList')
    local d = boot({ saved = saved, brokenAbilityList = true })
    d.fire(SLOT_EVENT, d.env.Container.Bandolier)
    d.fire('Events.AbilityUpdate', d.env.UIAbilityGroup.KnownAbility, RIFLE)
    expectButton(d, 11, PISTOL_SHOT, 'getAbilityList raises')
    ok(logged(d, 'getAbilityList failed'), 'logged')
    say('no list, or a list that raises: the bar is untouched')
end)

scenario('without this block nothing follows (the scenarios above test the block)', function()
    local first = boot({ known = { PISTOL_SHOT, STRIKE, PISTOL } }, false)
    switchWeapon(first, PISTOL, SMG)
    eq(first.abilityOnButton(11), PISTOL_SHOT, '009 alone')
    ok(not listens(first, SLOT_EVENT), '009 alone does not listen on '..WINDOW)
    say('009 alone: 592 stays on button 11 after a switch to an SMG')
end)

--=============================================================================
local failures = 0
for _, v in ipairs(variants) do
    variant = v
    print('== variant: '..v.name)
    for _, s in ipairs(scenarios) do
        print('  -- '..s.name)
        local passed, err = pcall(s.fn)
        if passed then
            print('  PASS')
        else
            failures = failures + 1
            print('  FAIL: '..tostring(err))
        end
    end
end
for _, name in ipairs(skipped) do
    print('== SKIPPED variant: '..name)
end
if #skipped > 0 and os.getenv('WEAPON_SHOT_BAR_REQUIRE_REAL') == '1' then
    print('FAILED: WEAPON_SHOT_BAR_REQUIRE_REAL=1 and a real variant was skipped')
    os.exit(1)
end
if failures > 0 then
    print(failures..' scenario(s) FAILED')
    os.exit(1)
end
print('all scenarios passed ('..#variants..' variant(s) x '..#scenarios..' scenarios)')
