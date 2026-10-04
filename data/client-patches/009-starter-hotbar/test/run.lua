-- REPL-style logic UAT for client patch 009-starter-hotbar.
--
--   lua5.1 data/client-patches/009-starter-hotbar/test/run.lua
--   python data/client-patches/009-starter-hotbar/test/run_lupa.py   (Windows)
--
-- Boots the ActionButtons module in a stub client (stubs.lua) with the
-- patch's hook appended to ActionProfileDefault1.lua, then drives first
-- login, late-arriving abilities, relogs and existing profiles. Each
-- scenario runs against every ActionButtons variant available:
--
--   model  a clean-room model of the module (fake_action_buttons.lua); always
--   stock  the stock client scripts; set SGW_UI_DIR to a stock client's
--          Working/SGWGame/Content/UI
--   v26    the stock scripts with the WQHD v26 UI pack's ActionProfiles.lua;
--          set SGW_V26_ACTIONPROFILES to that file as well
--
-- Set STARTER_HOTBAR_REQUIRE_REAL=1 to fail when stock or v26 is skipped.
-- No CME file is read from the repo; the real variants read your client.

local here = ((arg and arg[0]) or ''):match('^(.*)[/\\][^/\\]*$') or '.'
local patchDir = here..'/..'
local Stubs = dofile(here..'/stubs.lua')

local HOOK = assert(Stubs.readFile(patchDir..'/StarterHotbar.lua'), 'StarterHotbar.lua not found')
local STARTERS = { 592, 594, 597, 1646, 1218 }

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
-- Variants: name -> list of { chunkName, text } loaded in .toc order, with
-- the hook appended to the last one (as the patch ships it), or not.
local variants = {}

local MODEL = assert(Stubs.readFile(here..'/fake_action_buttons.lua'))
variants[#variants + 1] = {
    name = 'model',
    scripts = function( withHook )
        return { { 'fake_action_buttons.lua', MODEL..(withHook and HOOK or '') } }
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
    local function real( name, profilesText )
        return {
            name = name,
            scripts = function( withHook )
                return {
                    { 'ActionButtons.lua', buttons },
                    { name..'/ActionProfiles.lua', profilesText },
                    { 'ActionProfileDefault1.lua', default1..(withHook and HOOK or '') },
                }
            end,
        }
    end
    variants[#variants + 1] = real('stock', profiles)
    local v26Path = os.getenv('SGW_V26_ACTIONPROFILES')
    local v26 = v26Path and v26Path ~= '' and Stubs.readFile(v26Path)
    if v26 then
        ok(v26:find('applyWQHDFinalScaleV26', 1, true), 'SGW_V26_ACTIONPROFILES is not the v26 ActionProfiles.lua')
        variants[#variants + 1] = real('v26', v26)
    else
        skipped[#skipped + 1] = 'v26 (set SGW_V26_ACTIONPROFILES)'
    end
else
    skipped[#skipped + 1] = 'stock (set SGW_UI_DIR)'
    skipped[#skipped + 1] = 'v26 (set SGW_UI_DIR and SGW_V26_ACTIONPROFILES)'
end

--=============================================================================
local variant

local function boot( opts, withHook )
    if withHook == nil then
        withHook = true
    end
    local client = Stubs.newClient(opts)
    for _, s in ipairs(variant.scripts(withHook)) do
        client.load(s[2], s[1])
    end
    client.restore()
    client.fire('Events.ModuleLoaded')
    ok(client.dropCallbacks > 0, 'the module finished onModLoaded')
    return client
end

local function state( client )
    return client.profile().cimmeriaStarterHotbar
end

-- Simulate the player dropping an ability on a button (ActionProfileMod.receiveDrag).
local function drag( client, buttonId, abilityId )
    local env = client.env
    local actionId = env.getUnusedAction()
    env.ActionProfileMod.setButtonCurrentAction(buttonId, actionId)
    env.setActionToAbility(actionId, abilityId)
end

local function countActions( client )
    local n = 0
    for _ in pairs(client.actions) do
        n = n + 1
    end
    return n
end

-- Expect buttons first.. to hold the abilities in order, each with its icon
-- and each firing its ability on the first press.
local function expectBar( client, first, abilities, what )
    for i, abilityId in ipairs(abilities) do
        local b = first + i - 1
        eq(client.abilityOnButton(b), abilityId, what..': ability on button '..b)
        eq(client.iconOnButton(b), 'set:Icons image:Ability'..abilityId, what..': icon on button '..b)
        eq(client.press(b), abilityId, what..': first press on button '..b)
    end
end

local function expectEmpty( client, buttons, what )
    for _, b in ipairs(buttons) do
        eq(client.abilityOnButton(b), nil, what..': button '..b..' empty')
    end
end

local function range( a, b )
    local t = {}
    for i = a, b do
        t[#t + 1] = i
    end
    return t
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

local function ourFeedback( client )
    local n = 0
    for _, line in ipairs(client.feedback) do
        if line:find('starting abilities', 1, true) then
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

local function listening( client )
    return client.subscribed('Events.AbilityUpdate') or client.subscribed('Events.PropertyUpdated')
end

--=============================================================================
local scenarios = {}
local function scenario( name, fn )
    scenarios[#scenarios + 1] = { name = name, fn = fn }
end

scenario('the stub natives raise on a wrong argument count, like the tolua shims', function()
    local c = boot({ known = STARTERS })
    local env = c.env
    ok(not pcall(env.getAbilityList, 2), 'getAbilityList(2) raises')
    ok(not pcall(env.getUnusedAction, 1), 'getUnusedAction(1) raises')
    ok(not pcall(env.setActionToAbility, 1), 'setActionToAbility(1) raises')
    ok(not pcall(env.getActionInfo), 'getActionInfo() raises')
    ok(not pcall(env.unitsEqual, env.Unit.Player), 'unitsEqual(1) raises')
    ok(pcall(env.getAbilityList), 'getAbilityList() works')
    say('arity checked: getAbilityList(), getUnusedAction(), setActionToAbility(n,n), getActionInfo(n), unitsEqual(n,n)')
end)

scenario('fresh character, abilities known at load: seeded once, attacks first', function()
    local known = { 9999, 1218, 597, 592, 1646, 594 }
    local c = boot({ known = known })
    expectBar(c, 11, STARTERS, 'first login')
    expectEmpty(c, range(1, 10), 'first login, bandolier buttons')
    expectEmpty(c, range(16, 23), 'first login, other ability not placed')
    eq(#c.feedback, 1, 'one feedback line')
    eq(state(c), 'done', 'state after first login (every starting ability placed)')
    ok(not listening(c), 'no event subscriptions once done')
    say('buttons 11-15: 592 594 597 1646 1218, each with its icon and pressable; feedback "'..c.feedback[1]..'"')

    local actions = countActions(c)
    local c2 = boot({ known = known, saved = c.save() })
    eq(state(c2), 'done', 'state after relog')
    expectBar(c2, 11, STARTERS, 'relog')
    eq(countActions(c2), actions, 'no new actions on relog')
    eq(#c2.feedback, 0, 'no feedback on relog')
    ok(not listening(c2), 'no event subscriptions on relog')
    say('relog: state done, same five buttons, no new actions, no feedback, no subscriptions')
end)

scenario('abilities arrive one by one after the module loads (Events.AbilityUpdate)', function()
    local c = boot({ known = {} })
    expectEmpty(c, range(1, 23), 'before abilities')
    eq(state(c), 'pending', 'state before abilities')
    ok(listening(c), 'subscribed while pending')
    eq(#c.feedback, 0, 'no feedback before abilities')
    c.learn(STARTERS)
    expectBar(c, 11, STARTERS, 'after AbilityUpdate')
    eq(ourFeedback(c), 1, 'one feedback line across five top-ups')
    eq(state(c), 'done', 'done')
    ok(not listening(c), 'unsubscribed once done')
    say('pending until the abilities arrive, one event each; one feedback line; then unsubscribed')
end)

scenario('abilities arrive without an AbilityUpdate (PropertyUpdated fallback)', function()
    local c = boot({ known = {} })
    c.learn(STARTERS, 'property')
    expectBar(c, 11, STARTERS, 'after PropertyUpdated')
    say('seeded from the player PropertyUpdated fallback')
end)

scenario('a non-starting ability arrives first: still seeds when the starters follow', function()
    local c = boot({ known = {} })
    c.learn({ 4242 })
    eq(state(c), 'pending', 'still pending after a non-starter')
    expectEmpty(c, range(1, 23), 'non-starter not placed')
    c.learn(STARTERS)
    expectBar(c, 11, STARTERS, 'after the starters')
    say('4242 first leaves the profile pending; the starters then land on 11-15')
end)

scenario('abilities arrive in pieces: topped up this session, never in the next', function()
    local c = boot({ known = { 597, 592 } })
    expectBar(c, 11, { 592, 597 }, 'partial')
    c.learn({ 594 })
    expectBar(c, 11, { 592, 597, 594 }, 'topped up')
    eq(state(c), 'seeding', 'still seeding this session')
    eq(ourFeedback(c), 1, 'feedback not repeated on the top-up')
    local c2 = boot({ known = { 597, 592, 594 }, saved = c.save() })
    eq(state(c2), 'done', 'done on relog')
    c2.learn({ 1646, 1218 })
    expectEmpty(c2, { 14, 15 }, 'nothing added in a later session')
    eq(ourFeedback(c2), 0, 'no feedback in a later session')
    say('592 597 at load, 594 later the same session; 1646/1218 learned next session stay off the bar')
end)

scenario('nothing known in the first session: pending expires, a later session never seeds', function()
    local c = boot({ known = {} })
    eq(state(c), 'pending', 'pending in the first session')
    -- Weeks of play: the player builds a bar by hand.
    local c2 = boot({ known = {}, saved = c.save() })
    eq(state(c2), 'done', 'pending expired at the next login')
    ok(not listening(c2), 'not listening after expiry')
    drag(c2, 11, 4242)
    drag(c2, 16, 4343)
    local saved = c2.save()
    -- A later patch or server change makes the starters known.
    local c3 = boot({ known = STARTERS, saved = saved })
    c3.learn({ 7001 })
    c3.learn({ 7002 }, 'property')
    local same, where = deepEqual(c3.save().GActionProfiles, saved.GActionProfiles, 'GActionProfiles')
    ok(same, 'customised bar changed at '..tostring(where))
    eq(ourFeedback(c3), 0, 'no feedback')
    say('pending -> done at the second login; the hand-built bar is never seeded later')
end)

scenario('Lua state kept across logout to character select: still first session only', function()
    local c = boot({ known = { 592, 597 } })
    expectBar(c, 11, { 592, 597 }, 'first session')
    eq(state(c), 'seeding', 'seeding in the first session')
    c.relogKeepingLuaState()
    eq(state(c), 'done', 'done at the second login in the same Lua state')
    c.learn({ 594, 1646 })
    expectEmpty(c, { 13, 14 }, 'nothing added after the relog')
    eq(ourFeedback(c), 1, 'no second feedback line')

    local p = boot({ known = {} })
    p.relogKeepingLuaState()
    eq(state(p), 'done', 'pending expired in the same Lua state')
    p.learn(STARTERS)
    expectEmpty(p, range(11, 20), 'pending profile not seeded after the relog')
    say('the login marker is stored in the profile, so a kept Lua state cannot extend the first session')
end)

scenario('the stock version-2 wipe: the recreated profile is seeded', function()
    local pre = boot({ known = STARTERS }, false)
    drag(pre, 12, 4242)
    local c = boot({ known = STARTERS, saved = pre.save(), modVersion = 1 })
    expectBar(c, 11, STARTERS, 'after the wipe')
    eq(ourFeedback(c), 1, 'one starter feedback line')
    eq(state(c), 'done', 'done')
    say('profiles from module version 1 are wiped by the client; the new Default profile is seeded')
end)

scenario('existing customised profile from before the patch: untouched', function()
    local pre = boot({ known = STARTERS }, false)
    drag(pre, 12, 1646)
    drag(pre, 17, 592)
    local saved = pre.save()
    local c = boot({ known = STARTERS, saved = saved })
    c.learn({ 597 })
    local same, where = deepEqual(c.save().GActionProfiles, saved.GActionProfiles, 'GActionProfiles')
    ok(same, 'profile changed at '..tostring(where))
    eq(#c.feedback, 0, 'no feedback')
    eq(c.abilityOnButton(11), nil, 'button 11 stays empty')
    ok(not listening(c), 'not listening on an unmarked profile')
    say('custom bar (12=1646, 17=592) byte-for-byte the same after load and an ability update')
end)

scenario('existing empty profile from before the patch: untouched', function()
    local pre = boot({ known = STARTERS }, false)
    local saved = pre.save()
    local c = boot({ known = STARTERS, saved = saved })
    c.learn({ 597 })
    expectEmpty(c, range(1, 23), 'pre-patch empty profile')
    eq(state(c), nil, 'no state on a profile the patch did not create')
    say('an empty bar created before the patch is not seeded')
end)

scenario('a profile made with the editor New Profile button is not seeded', function()
    local c = boot({ known = STARTERS })
    local env = c.env
    local id = env.ActionProfileMod.createProfile('Mine', env.ActionProfileMod.SafetyTemplate)
    env.ActionProfileMod.loadProfile(id)
    eq(env.GActionCurrentProfileId, id, 'switched to the new profile')
    c.learn({ 4242 })
    expectEmpty(c, range(1, 23), 'new profile')
    eq(state(c), nil, 'no state on the new profile')
    say('second profile '..id..' stays empty')
end)

scenario('the player fills the bar before the abilities arrive: nothing replaced', function()
    local c = boot({ known = {} })
    for b = 11, 20 do
        drag(c, b, 7000 + b)
    end
    c.learn(STARTERS)
    for b = 11, 20 do
        eq(c.abilityOnButton(b), 7000 + b, 'button '..b..' kept')
    end
    eq(state(c), 'done', 'done when no button is free')
    say('all ten layer buttons kept; state done')
end)

scenario('bar fills up while a starter is still missing: done, no further listening', function()
    local c = boot({ known = {} })
    for b = 11, 18 do
        drag(c, b, 7000 + b)
    end
    -- A class without Strike (594): one starter missing ahead of the rest.
    c.learn({ 592, 597, 1646, 1218 })
    eq(c.abilityOnButton(19), 592, 'button 19')
    eq(c.abilityOnButton(20), 597, 'button 20')
    eq(state(c), 'done', 'done when the bar is full, with 594 still unknown')
    ok(not listening(c), 'unsubscribed once full')
    say('592/597 on the last two free buttons; full -> done although 594 never arrived')
end)

scenario('only known-ability updates trigger a seed (UIAbilityGroup filter)', function()
    local c = boot({ known = {} })
    c.learn(STARTERS, 'silent')
    local env = c.env
    for _, id in ipairs(STARTERS) do
        c.fire('Events.AbilityUpdate', env.UIAbilityGroup.Training, id)
    end
    expectEmpty(c, range(11, 20), 'training-group updates ignored')
    c.fire('Events.AbilityUpdate', env.UIAbilityGroup.KnownAbility, 592)
    expectBar(c, 11, STARTERS, 'after a known-ability update')
    say('Training-group AbilityUpdate events are ignored; a KnownAbility one seeds')
end)

scenario('the player already placed a starting ability: no duplicate', function()
    local c = boot({ known = {} })
    drag(c, 13, 592)
    c.learn(STARTERS)
    eq(c.abilityOnButton(11), 594, 'button 11')
    eq(c.abilityOnButton(12), 597, 'button 12')
    eq(c.abilityOnButton(13), 592, 'button 13 kept')
    eq(c.abilityOnButton(14), 1646, 'button 14')
    eq(c.abilityOnButton(15), 1218, 'button 15')
    eq(c.abilityOnButton(16), nil, 'button 16')
    say('592 stays on 13 once; the other four fill 11, 12, 14, 15')
end)

scenario('rebound buttons: only layer-bound buttons 11-20 with nothing on them are used', function()
    local c = boot({ known = {}, weaponItemId = 555 })
    local env = c.env
    local p = c.profile()
    local Layer, Bandolier = env.ActionProfileMod.ButtonBinding_Layer, env.ActionProfileMod.ButtonBinding_Bandolier
    -- Button 3 rebound to the layer bar: outside 11-20, so never used.
    p.buttonInfo[3].binding = Layer
    -- Button 11 rebound to the bandolier: not a layer button.
    p.buttonInfo[11].binding = Bandolier
    -- Button 12 was a bandolier button with a weapon action and is now
    -- layer-bound; it still shows the old action until the profile reloads.
    local old = env.getUnusedAction()
    env.setActionToAbility(old, 7777)
    p.buttonInfo[12].actions = { [555] = old }
    p.buttonInfo[12].binding = Layer
    env.ActionButtonMod.bindButtonToAction(12, old)
    c.learn(STARTERS)
    eq(c.abilityOnButton(3), nil, 'button 3 (layer-bound, below 11) empty')
    eq(c.abilityOnButton(11), nil, 'button 11 (bandolier-bound) empty')
    eq(p.buttonInfo[11].actions[555], nil, 'no weapon-keyed action on 11')
    eq(c.abilityOnButton(12), 7777, 'button 12 keeps its old action')
    expectBar(c, 13, STARTERS, 'starters from 13')
    say('3 and 11 skipped by the binding/range rule, 12 skipped while it still shows an action')
end)

scenario('PropertyUpdated fallback: player only, at most once a second', function()
    local c = boot({ known = {} })
    local base = c.listCalls
    c.advance(5)
    c.propertyUpdate()
    eq(c.listCalls, base + 1, 'first player update polls')
    c.advance(0.5)
    c.propertyUpdate()
    eq(c.listCalls, base + 1, 'second update within 1 s is skipped')
    c.advance(0.6)
    c.propertyUpdate(c.env.Unit.Target)
    eq(c.listCalls, base + 1, "another unit's update is ignored")
    c.propertyUpdate()
    eq(c.listCalls, base + 2, 'player update after 1.1 s polls')
    say('one list read per second of player updates; NPC updates ignored')
end)

scenario('no getAbilityList native: no error, waits', function()
    local c = boot({ known = STARTERS, noAbilityList = true })
    expectEmpty(c, range(1, 23), 'no list')
    eq(state(c), 'pending', 'still pending')
    say('module loads normally; nothing placed')
end)

scenario('getAbilityList raises: logged once, module still loads', function()
    local c = boot({ known = STARTERS, brokenAbilityList = true })
    c.learn({ 4242 })
    c.learn({ 4343 }, 'property')
    expectEmpty(c, range(1, 23), 'nothing placed')
    local n = 0
    for _, line in ipairs(c.log) do
        if line:find('getAbilityList failed', 1, true) then
            n = n + 1
        end
    end
    eq(n, 1, 'one log line for the failing native')
    say('the native error reaches Debug:log once')
end)

scenario('setActionToAbility raises: module still loads, never retried', function()
    local c = boot({ known = STARTERS, failSetAction = true })
    eq(state(c), 'done', 'done after an error')
    ok(logged(c, 'starter hotbar: error'), 'the error is logged')
    say('onModLoaded finished; error logged; state done')
end)

scenario('Events.ActionUpdated arrives late: icons still show', function()
    local c = boot({ known = STARTERS, asyncActionUpdated = true })
    expectBar(c, 11, STARTERS, 'async ActionUpdated')
    say('the hook redraws each button itself')
end)

scenario('button layout is never changed by the hook', function()
    local without = boot({ known = STARTERS }, false).save().GActionProfiles
    local with = boot({ known = STARTERS }).save().GActionProfiles
    for _, p in pairs({ without, with }) do
        for _, info in pairs(p[1].buttonInfo) do
            info.actions = nil
        end
        p[1].cimmeriaStarterHotbar = nil
        p[1].cimmeriaStarterLoad = nil
    end
    local same, where = deepEqual(with, without, 'GActionProfiles')
    ok(same, 'layout differs at '..tostring(where))
    say('positions, anchors, bindings, shapes and dock groups identical with and without the hook')
end)

--=============================================================================
local failed = 0
for _, v in ipairs(variants) do
    variant = v
    print('== variant: '..v.name)
    for _, s in ipairs(scenarios) do
        print('  -- '..s.name)
        local good, err = pcall(s.fn)
        if good then
            print('  PASS')
        else
            failed = failed + 1
            print('  FAIL  '..tostring(err))
        end
    end
end
for _, s in ipairs(skipped) do
    print('== SKIPPED variant: '..s)
end

if failed > 0 then
    print(failed..' scenario(s) failed')
    os.exit(1)
end
if #skipped > 0 and os.getenv('STARTER_HOTBAR_REQUIRE_REAL') == '1' then
    print('real variants skipped and STARTER_HOTBAR_REQUIRE_REAL=1')
    os.exit(1)
end
print('all scenarios passed ('..#variants..' variant(s) x '..#scenarios..' scenarios)')
os.exit(0)
