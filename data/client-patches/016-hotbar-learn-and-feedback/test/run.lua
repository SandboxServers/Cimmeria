-- REPL-style logic UAT for client patch 016-hotbar-learn-and-feedback.
--
--   lua5.1 data/client-patches/016-hotbar-learn-and-feedback/test/run.lua
--   python data/client-patches/016-hotbar-learn-and-feedback/test/run_lupa.py   (no system lua5.1)
--
-- Boots the ActionButtons module in 009's stub client (its stubs.lua and
-- clean-room model, read from the 009 directory) with 009's hook, 015's
-- block and this patch's block appended to ActionProfileDefault1.lua, as the
-- three patches ship. Sessions are separate boots that carry the saved
-- variables over, as a relog does.
--
--   model  the clean-room model of the module; always
--   stock  the client's own scripts; set SGW_UI_DIR to a client's
--          Working/SGWGame/Content/UI. A stock ActionProfileDefault1.lua gets
--          all three blocks appended; one that already carries 009 (a
--          launcher-installed client) gets 015's and this one.
--
-- Set HOTBAR_LEARN_REQUIRE_REAL=1 to fail when stock is skipped.
-- No CME file is read from the repo; the stock variant reads your client.

local here = ((arg and arg[0]) or ''):match('^(.*)[/\\][^/\\]*$') or '.'
local patchDir = here..'/..'
local starterDir = patchDir..'/../009-starter-hotbar'
local shotDir = patchDir..'/../015-weapon-shot-bar'
local Stubs = dofile(starterDir..'/test/stubs.lua')

local HOOK9 = assert(Stubs.readFile(starterDir..'/StarterHotbar.lua'), '009 StarterHotbar.lua not found')
local HOOK15 = assert(Stubs.readFile(shotDir..'/WeaponShotBar.lua'), '015 WeaponShotBar.lua not found')
local HOOK16 = assert(Stubs.readFile(patchDir..'/HotbarLearnAndFeedback.lua'), 'HotbarLearnAndFeedback.lua not found')

local PISTOL_SHOT, STRIKE, HEAL_FOCUS, HEALTH_HEAL, RECUPERATION, STAFF_SWING = 592, 594, 597, 1646, 1218, 1984
local PISTOL, SMG, RIFLE = 579, 559, 581
local OTHER = 9999
local SLOT_EVENT = 'Events.InventoryUpdateContainerActiveSlot'
local NO_SHOT = 'This weapon has no ranged attack.'

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
-- Variants: name -> list of { chunkName, text } in .toc order. with16 false
-- gives 009 and 015 only, the bar as it is before this patch.
local variants = {}

local MODEL = assert(Stubs.readFile(starterDir..'/test/fake_action_buttons.lua'))
variants[#variants + 1] = {
    name = 'model',
    scripts = function( with16 )
        return { { 'fake_action_buttons.lua', MODEL..HOOK9..HOOK15..(with16 and HOOK16 or '') } }
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
    local base = default1
    if default1:find('ActionProfileMod.WeaponShotBar', 1, true) then
        ok(default1:sub(-#(HOOK9..HOOK15)) == HOOK9..HOOK15, 'ActionProfileDefault1.lua carries 009 and 015 blocks that are not the committed ones')
    elseif default1:find('ActionProfileMod.StarterHotbar', 1, true) then
        ok(default1:sub(-#HOOK9) == HOOK9, 'ActionProfileDefault1.lua carries a 009 block that is not the committed one')
        base = default1..HOOK15
    else
        base = default1..HOOK9..HOOK15
    end
    variants[#variants + 1] = {
        name = 'stock',
        scripts = function( with16 )
            return {
                { 'ActionButtons.lua', buttons },
                { 'ActionProfiles.lua', profiles },
                { 'ActionProfileDefault1.lua', base..(with16 and HOOK16 or '') },
            }
        end,
    }
else
    skipped[#skipped + 1] = 'stock (set SGW_UI_DIR)'
end

--=============================================================================
local variant

local function boot( opts, with16 )
    if with16 == nil then
        with16 = true
    end
    local client = Stubs.newClient(opts)
    for _, s in ipairs(variant.scripts(with16)) do
        client.load(s[2], s[1])
    end
    client.restore()
    client.fire('Events.ModuleLoaded')
    ok(client.dropCallbacks > 0, 'the module finished onModLoaded')
    return client
end

-- The next session of the same character, on the same machine.
local function relog( client, opts, with16 )
    opts = opts or {}
    opts.saved = client.save()
    return boot(opts, with16)
end

-- The player drops an ability on a button (ActionProfileMod.receiveDrag).
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

-- The player drags a button's action off the bar.
local function remove( client, buttonId )
    local env = client.env
    local actionId = env.ActionButtonMod.getActionForButton(buttonId)
    local info = client.profile().buttonInfo[buttonId]
    info.actions[client.profile().currentLayer or 1] = nil
    env.ActionButtonMod.unbindButton(buttonId)
    if actionId then
        env.clearAction(actionId)
    end
end

local function forget( client, abilityId )
    for i = #client.known, 1, -1 do
        if client.known[i] == abilityId then
            table.remove(client.known, i)
        end
    end
end

-- The server's weapon switch (see 015's UAT).
local function switchWeapon( client, oldShot, newShot )
    local env = client.env
    client.fire(SLOT_EVENT, env.Container.Bandolier)
    if oldShot then
        forget(client, oldShot)
        client.fire('Events.AbilityUpdate', env.UIAbilityGroup.KnownAbility, oldShot)
    end
    if newShot then
        client.learn({ newShot })
    end
end

local function feedbackCount( client, text )
    local n = 0
    for _, line in ipairs(client.feedback) do
        if line:find(text, 1, true) then
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
    end
end

local function expectBar( client, first, abilities, what )
    for i, id in ipairs(abilities) do
        expectButton(client, first + i - 1, id, what)
    end
end

local function expectEmpty( client, a, b, what )
    for n = a, b do
        eq(client.abilityOnButton(n), nil, what..': button '..n..' empty')
    end
end

-- How many buttons 1-100 hold abilityId.
local function onBarCount( client, abilityId )
    local n = 0
    for b = 1, 100 do
        if client.abilityOnButton(b) == abilityId then
            n = n + 1
        end
    end
    return n
end

-- A character whose first session ended with nothing known, as an SGC or
-- CellBlock character's first session does since Class Start v6.
local function secondSession( opts, with16 )
    local first = boot({ known = {} }, with16)
    eq(first.profile().cimmeriaStarterHotbar, 'pending', 'first session: nothing to place')
    opts = opts or {}
    opts.known = opts.known or {}
    return relog(first, opts, with16)
end

--=============================================================================
local scenarios = {}
local function scenario( name, fn )
    scenarios[#scenarios + 1] = { name = name, fn = fn }
end

scenario('before this patch: the F8 bug (starters learned in a later session leave the bar empty)', function()
    local c = secondSession(nil, false)
    eq(c.profile().cimmeriaStarterHotbar, 'done', '009 done at the second load')
    c.learn({ PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION })
    expectEmpty(c, 11, 20, '009 and 015 only')
    say('009 + 015 only: second session, four starters learned, buttons 11-20 stay empty')
end)

scenario('F8: starters learned in a later session go on the first empty buttons', function()
    local c = secondSession()
    eq(c.profile().cimmeriaStarterHotbar, 'done', '009 done at the second load')
    c.learn({ OTHER, RECUPERATION, PISTOL_SHOT, STRIKE, HEAL_FOCUS })
    -- In the order they are learned: each update places what it can.
    expectBar(c, 11, { RECUPERATION, PISTOL_SHOT, STRIKE, HEAL_FOCUS }, 'learned one by one')
    expectEmpty(c, 15, 20, 'nothing else')
    eq(c.press(12), PISTOL_SHOT, 'first press fires Pistol Shot')
    eq(feedbackCount(c, ' is on your action bar.'), 4, 'one line per placed ability')
    eq(feedbackCount(c, 'Ability 592 is on your action bar.'), 1, 'the line names the ability')
    ok(logged(c, 'placed ability 592 on button 12'), 'logged')

    -- All at once (one update for the whole list): the starting order.
    local d = secondSession()
    d.learn({ RECUPERATION, HEAL_FOCUS, PISTOL_SHOT, STRIKE }, 'silent')
    d.fire('Events.AbilityUpdate', d.env.UIAbilityGroup.KnownAbility, STRIKE)
    expectBar(d, 11, { PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION }, 'learned at once')
    say('second session, learned one by one: 1218, 592, 594, 597 on 11-14 with one line each; learned at once: 592, 594, 597, 1218')
end)

scenario('F8: starters known at the load of a later session are placed at once', function()
    local c = secondSession({ known = { STRIKE, PISTOL_SHOT, PISTOL } })
    expectBar(c, 11, { PISTOL_SHOT, STRIKE }, 'known at load')
    say('known at the second load: 592 and 594 on 11-12 when the profile loads')
end)

scenario('F8: a player\'s button is never replaced', function()
    local c = secondSession()
    drag(c, 11, OTHER)
    drag(c, 13, OTHER + 1)
    c.learn({ PISTOL_SHOT, STRIKE })
    expectButton(c, 11, OTHER, 'the player\'s button')
    expectButton(c, 12, PISTOL_SHOT, 'the first empty one')
    expectButton(c, 13, OTHER + 1, 'the player\'s button')
    expectButton(c, 14, STRIKE, 'the next empty one')
    say('player buttons on 11 and 13: 592 -> 12, 594 -> 14')
end)

scenario('F8: no empty button: not placed, and not later either', function()
    local c = secondSession()
    for b = 11, 20 do
        drag(c, b, OTHER + b)
    end
    c.learn({ PISTOL_SHOT })
    eq(onBarCount(c, PISTOL_SHOT), 0, 'no room')
    ok(logged(c, 'ability 592 learned with no empty button'), 'logged')
    eq(c.profile().cimmeriaStarterPlaced[PISTOL_SHOT], 2, 'recorded as dealt with (no room)')
    remove(c, 15)
    c.learn({ OTHER })
    eq(onBarCount(c, PISTOL_SHOT), 0, 'a freed button is left to the player')
    local d = relog(c)
    eq(onBarCount(d, PISTOL_SHOT), 0, 'nor at the next login')
    say('buttons 11-20 full when 592 is learned: not placed; freeing 15 later does not bring it back')
end)

scenario('F8: one the player removed is never placed again', function()
    local c = secondSession()
    c.learn({ PISTOL_SHOT, STRIKE })
    expectBar(c, 11, { PISTOL_SHOT, STRIKE }, 'placed')
    remove(c, 11)
    c.learn({ HEAL_FOCUS })
    expectButton(c, 11, HEAL_FOCUS, 'the freed button takes the next new one')
    eq(onBarCount(c, PISTOL_SHOT), 0, 'Pistol Shot not back this session')
    local d = relog(c)
    eq(onBarCount(d, PISTOL_SHOT), 0, 'nor at the next login')
    d.learn({ RECUPERATION })
    expectButton(d, 13, RECUPERATION, 'new ones still arrive')
    eq(onBarCount(d, PISTOL_SHOT), 0, 'nor on a later update')

    -- Removed in the first session, where 009 placed it.
    local e = boot({ known = { PISTOL_SHOT, STRIKE } })
    expectBar(e, 11, { PISTOL_SHOT, STRIKE }, '009 seeding')
    remove(e, 11)
    local f = relog(e)
    f.learn({ HEAL_FOCUS })
    eq(onBarCount(f, PISTOL_SHOT), 0, '009\'s placement is remembered')
    expectButton(f, 11, HEAL_FOCUS, 'into the freed button')
    say('592 removed after it was placed (by this block or by 009): never placed again, here or after a relog')
end)

scenario('F5: a Free Jaffa gets Staff Swing in the first session, after the attacks', function()
    local c = boot({ known = { RECUPERATION, HEAL_FOCUS, STAFF_SWING } })
    expectBar(c, 11, { STAFF_SWING, HEAL_FOCUS, RECUPERATION }, 'Free Jaffa first session')
    eq(feedbackCount(c, 'Your starting abilities are on your action bar.'), 1, '009\'s line')
    eq(c.press(11), STAFF_SWING, 'first press fires Staff Swing')

    -- Learned after the first session, like the other starters.
    local d = secondSession()
    d.learn({ STAFF_SWING })
    expectButton(d, 11, STAFF_SWING, 'later session')

    -- The order of 009's list, Staff Swing after Strike, once.
    local list = c.env.ActionProfileMod.StarterHotbar.Abilities
    eq(table.concat(list, ','), '592,594,1984,597,1646,1218', '009\'s list')
    say('known 597, 1218, 1984: 1984 on 11, 597 on 12, 1218 on 13; 009 list 592,594,1984,597,1646,1218')
end)

scenario('first session still 009\'s: same bar and line as before, no early takeover', function()
    local c = boot({ known = { OTHER, RECUPERATION, HEAL_FOCUS, PISTOL_SHOT, HEALTH_HEAL, STRIKE, PISTOL } })
    expectBar(c, 11, { PISTOL_SHOT, STRIKE, HEAL_FOCUS, HEALTH_HEAL, RECUPERATION }, '009 seeding')
    eq(feedbackCount(c, 'Your starting abilities are on your action bar.'), 1, '009\'s line once')
    eq(feedbackCount(c, ' is on your action bar.'), 0, 'no line of this block in the first session')
    local placed = c.profile().cimmeriaStarterPlaced
    for _, id in ipairs({ PISTOL_SHOT, STRIKE, HEAL_FOCUS, HEALTH_HEAL, RECUPERATION }) do
        eq(placed[id], 1, 'recorded '..id)
    end
    say('009 seeds 11-15 and writes its line; every placement is recorded for later sessions')
end)

scenario('a profile 009 did not create is never touched', function()
    -- A profile that existed before 009 (no mark), with an empty bar.
    local c = boot({ known = {} })
    local saved = c.save()
    local p = saved.GActionProfiles[saved.GActionCurrentProfileId]
    p.cimmeriaStarterHotbar = nil
    p.cimmeriaStarterLoad = nil
    p.cimmeriaStarterPlaced = nil
    local d = boot({ known = { PISTOL_SHOT }, saved = saved })
    d.learn({ STRIKE })
    expectEmpty(d, 11, 20, 'unmarked profile')
    eq(d.profile().cimmeriaStarterPlaced, nil, 'no tracking added')
    say('unmarked profile: buttons 11-20 stay empty, no key added')
end)

scenario('a profile from before this patch: known starters missing from the bar are placed once', function()
    -- Two sessions with 009 and 015 only: the F8 state, an empty bar.
    local before = secondSession(nil, false)
    before.learn({ PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION, PISTOL })
    drag(before, 11, HEAL_FOCUS)
    -- Then the launcher applies 016.
    local c = relog(before, { known = { PISTOL_SHOT, STRIKE, HEAL_FOCUS, RECUPERATION, PISTOL } })
    expectBar(c, 11, { HEAL_FOCUS, PISTOL_SHOT, STRIKE, RECUPERATION }, 'first load with 016')
    eq(onBarCount(c, HEAL_FOCUS), 1, 'the player\'s Heal Focus is not doubled')
    say('empty bar from the F8 bug plus a player Heal Focus on 11: 592, 594, 1218 on 12-14 at the first load with 016')
end)

scenario('a shot the bar already holds counts as Pistol Shot (015): no second shot button', function()
    local c = secondSession({ known = { SMG } })
    c.learn({ PISTOL_SHOT })
    local shots = onBarCount(c, PISTOL_SHOT) + onBarCount(c, SMG)
    eq(shots, 1, 'one shot button')
    c.fire(SLOT_EVENT, c.env.Container.Bandolier)
    expectButton(c, 11, SMG, 'followed by 015')
    c.learn({ STRIKE })
    switchWeapon(c, SMG, PISTOL)
    expectButton(c, 11, PISTOL_SHOT, 'back on a pistol')
    eq(onBarCount(c, PISTOL_SHOT) + onBarCount(c, SMG), 1, 'still one shot button')
    say('592 learned with an SMG out: placed once, 015 moves it to 559 and back; never a second one')
end)

scenario('F6: a shot button with a knife out says so, once a second', function()
    local c = secondSession({ known = { PISTOL_SHOT, STRIKE, PISTOL } })
    switchWeapon(c, PISTOL, SMG)
    expectButton(c, 11, SMG, 'SMG out')
    eq(c.press(11), SMG, 'the SMG shot fires')
    eq(feedbackCount(c, NO_SHOT), 0, 'no line with a gun out')

    switchWeapon(c, SMG, nil)            -- Combat Knife: no ranged basic attack
    expectButton(c, 11, SMG, 'the button keeps its last shot')
    eq(c.press(11), SMG, 'the press still goes to the client')
    eq(feedbackCount(c, NO_SHOT), 1, 'one line on the first press')
    ok(logged(c, 'pressed with no weapon shot known'), 'logged')
    c.press(11)
    c.advance(0.5)
    c.press(11)
    eq(feedbackCount(c, NO_SHOT), 1, 'repeats within a second are not repeated')
    c.advance(1)
    c.press(11)
    eq(feedbackCount(c, NO_SHOT), 2, 'a press a second later gets the line again')

    eq(c.press(12), STRIKE, 'Strike fires')
    c.advance(5)
    c.press(12)
    eq(feedbackCount(c, NO_SHOT), 2, 'never for a non-shot button')

    switchWeapon(c, nil, RIFLE)
    expectButton(c, 11, RIFLE, 'a rifle out')
    c.advance(5)
    c.press(11)
    eq(feedbackCount(c, NO_SHOT), 2, 'no line with a gun out')
    say('knife out, button 11 on 559: line on the first press, not again within 1 s, again after; Strike and a rifle never')
end)

scenario('F6: Pistol Shot still known with a knife out goes to the server, no line here', function()
    local c = secondSession({ known = { PISTOL_SHOT, STRIKE, PISTOL } })
    switchWeapon(c, PISTOL, nil)
    expectButton(c, 11, PISTOL_SHOT, 'Pistol Shot stays')
    eq(c.press(11), PISTOL_SHOT, 'pressed')
    eq(feedbackCount(c, NO_SHOT), 0, 'known ability: the server answers (CS-07)')

    -- A GM who knows every shot: the client sends them all.
    local d = secondSession({ known = { STRIKE, SMG, RIFLE } })
    drag(d, 15, PISTOL)
    d.press(15)
    eq(feedbackCount(d, NO_SHOT), 0, 'another weapon shot is known')
    say('592 known: no line (the server answers); a shot not known while another is: no line')
end)

scenario('F6: works by key binding as well as by click (both reach useAction)', function()
    local c = secondSession({ known = { PISTOL_SHOT, STRIKE, PISTOL } })
    switchWeapon(c, PISTOL, SMG)
    switchWeapon(c, SMG, nil)
    local env = c.env
    -- The stock module subscribes the key binding to the same handler; call
    -- the handler the way a key press does, with the button window.
    local w = c.windows[string.format('ActionButtons_%dButton', 11)]
    env.ActionButtonMod.onActionPress(w)
    eq(feedbackCount(c, NO_SHOT), 1, 'line on a key press')
    say('ActionButtonMod.onActionPress from a key binding: the line shows')
end)

scenario('a failing setActionToAbility switches the block off without an error', function()
    local c = secondSession({ failSetAction = true })
    c.learn({ PISTOL_SHOT })
    ok(logged(c, 'starter learned: error'), 'logged')
    local calls = c.listCalls
    c.learn({ STRIKE })
    expectEmpty(c, 11, 20, 'nothing placed')
    ok(c.listCalls - calls <= 2, 'no retry loop')
    say('setActionToAbility raises: logged once, switched off, the bar is as it was')
end)

scenario('no getAbilityList, or one that raises: nothing placed, no line, no error', function()
    local c = secondSession({ noAbilityList = true })
    c.learn({ PISTOL_SHOT })
    expectEmpty(c, 11, 20, 'no getAbilityList')
    drag(c, 11, SMG)
    c.press(11)
    eq(feedbackCount(c, NO_SHOT), 0, 'no list, no line')
    local d = secondSession({ brokenAbilityList = true })
    d.learn({ PISTOL_SHOT })
    expectEmpty(d, 11, 20, 'getAbilityList raises')
    say('no list: the bar is untouched and presses pass through')
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
if #skipped > 0 and os.getenv('HOTBAR_LEARN_REQUIRE_REAL') == '1' then
    print('FAILED: HOTBAR_LEARN_REQUIRE_REAL=1 and a real variant was skipped')
    os.exit(1)
end
if failures > 0 then
    print(failures..' scenario(s) FAILED')
    os.exit(1)
end
print('all scenarios passed ('..#variants..' variant(s) x '..#scenarios..' scenarios)')
