-- Logic UAT for client patch 017-launcher-sign-in: the overlay Login.lua
-- against the launcher's LoginInternal.lua, in Lua 5.1.
--
--   lua5.1 data/client-patches/017-launcher-sign-in/test/run.lua
--   python data/client-patches/017-launcher-sign-in/test/run_lupa.py   (no system lua5.1)
--
-- Each case boots a fresh client: stubs.lua's windows from Login.layout, a
-- LoginInternal.lua, the overlay Login.lua, then LoginMod.startup(). The
-- old-screen case also needs SGW_UI_DIR, a client's Working/SGWGame/Content/UI,
-- and runs the stock Startup/Login/Login.lua from it; without it the case is
-- skipped with a note. CI runs this file directly with Ubuntu's lua5.1.

local here = ((arg and arg[0]) or ''):match('^(.*)[/\\][^/\\]*$') or '.'
local patchDir = here..'/..'
local Stubs = dofile(here..'/stubs.lua')

local LAYOUT = patchDir..'/Login.layout'
local LOGIN = assert(Stubs.readFile(patchDir..'/Login.lua'), 'Login.lua not found')
local GOLDEN = assert(Stubs.readFile(here..'/LoginInternal.ticket.lua'), 'LoginInternal.ticket.lua not found')

local SERVER = 'http://127.0.0.1:50123'
local ACCOUNT = 'Player_1234'
local TICKET = 'AbCdEf0123456789AbCdEf0123456789'
local STOCK_SITE = 'http://projectgiza.com'
local STOCK_ACCOUNT_PAGE = 'http://projectgiza.com/forums/viewtopic.php?f=4&t=51'

-- What login_servers::render writes for one server (CRLF).
local STOCK_INTERNAL = 'LoginMod = {}\r\n'..
    'function LoginMod.loadServerSystems()\r\n'..
    '    LoginMod.servers = {}\r\n'..
    '    LoginMod.servers["Cimmeria"] = "'..SERVER..'"\r\n'..
    'end\r\n'

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
local function shown( name )
    ok(_G[name]:isVisible(), name..' is hidden')
end
local function hidden( name )
    ok(not _G[name]:isVisible(), name..' is visible')
end
local function plain( src, from, to )
    local i, j = src:find(from, 1, true)
    ok(i, 'golden text not found: '..from)
    return src:sub(1, i - 1)..to..src:sub(j + 1)
end

-- A fresh client: LoginInternal.lua, then the overlay Login.lua, then startup().
local function boot( internal, login )
    Stubs.reset(LAYOUT)
    assert(loadstring(internal, 'LoginInternal.lua'))()
    assert(loadstring(login or LOGIN, 'Login.lua'))()
    LoginMod.startup()
end

--=============================================================================
local cases = {}

cases[#cases + 1] = { name = 'ticket, manual', run = function()
    boot(GOLDEN)
    for _, name in ipairs({ 'Login_AccountEdit', 'Login_PasswordEdit', 'Login_AccountLabel',
                            'Login_PasswordLabel', 'Login_RememberCheck', 'Login_ServerTypeCombo' }) do
        hidden(name)
    end
    shown('Login_StatusText')
    eq(Login_StatusText:getText(), 'Signed in as Player ('..ACCOUNT..')', 'status text')
    eq(Login_LoginButton:getText(), 'Enter', 'login button')
    eq(LoginMod.cimmeriaOverlayVersion, 1, 'overlay version')
    eq(Login_AccountEdit:getText(), '', 'account box not pre-filled by the overlay')
    eq(Login_PasswordEdit:getText(), '', 'password box not pre-filled by the overlay')
    shown('Login_PasswordModeButton')
    hidden('Login_WebsiteButton')
    hidden('Login_AccountButton')
    hidden('Login_ForumsButton')

    Stubs.press('Login_LoginButton')
    eq(#Stubs.calls, 1, 'accountLogin calls')
    local c = Stubs.calls[1]
    eq(c.server, SERVER, 'server')
    eq(c.user, ACCOUNT, 'account')
    eq(c.pass, TICKET, 'ticket')
    eq(c.status, 'Connecting...', 'status when accountLogin ran')
    eq(GSavedVar_AccountName, '', 'saved account name')
end }

cases[#cases + 1] = { name = 'ticket, auto', run = function()
    boot(plain(GOLDEN, 'CimmeriaLogin.auto = false', 'CimmeriaLogin.auto = true'))
    eq(#Stubs.calls, 1, 'accountLogin calls after startup')
    eq(Stubs.calls[1].user, ACCOUNT, 'auto account')
    LoginMod.startup()
    eq(#Stubs.calls, 1, 'accountLogin calls after a second startup')
end }

cases[#cases + 1] = { name = 'failure', run = function()
    boot(GOLDEN)
    LoginMod.onLoginFailure(nil, nil, 'x', 4)
    eq(Login_StatusText:getText(), 'Your sign-in for this launch is no longer valid. '..
        'Close the game and press Play in the launcher.', 'status for 4')
    eq(Login_LoginButton:getText(), 'Retry', 'button for 4')

    LoginMod.onLoginFailure(nil, nil, 'x', 13)
    ok(Login_StatusText:getText():find('13', 1, true), 'generic status names the code')
    eq(Login_LoginButton:getText(), 'Retry', 'button for 13')
    eq(#Stubs.prompts, 0, 'prompts in ticket mode')
end }

cases[#cases + 1] = { name = 'use password', run = function()
    boot(GOLDEN)
    Login_AccountEdit:setText('stale-name')
    Login_PasswordEdit:setText('stale-pw')
    Stubs.press('Login_PasswordModeButton')
    for _, name in ipairs({ 'Login_AccountEdit', 'Login_PasswordEdit', 'Login_AccountLabel',
                            'Login_PasswordLabel', 'Login_RememberCheck' }) do
        shown(name)
    end
    eq(Login_AccountEdit:getText(), '', 'account box after the switch')
    eq(Login_PasswordEdit:getText(), '', 'password box after the switch')
    hidden('Login_StatusText')
    hidden('Login_PasswordModeButton')
    eq(Login_LoginButton:getText(), 'Login', 'login button')

    Login_AccountEdit:setText('Player_1234')
    Login_PasswordEdit:setText('typed-pw')
    Stubs.press('Login_LoginButton')
    eq(#Stubs.calls, 1, 'accountLogin calls')
    eq(Stubs.calls[1].server, SERVER, 'server')
    eq(Stubs.calls[1].user, ACCOUNT, 'typed account')
    eq(Stubs.calls[1].pass, 'typed-pw', 'typed password')
end }

cases[#cases + 1] = { name = 'stock', run = function()
    boot(STOCK_INTERNAL)
    eq(CimmeriaLogin, nil, 'CimmeriaLogin')
    for _, name in ipairs({ 'Login_AccountEdit', 'Login_PasswordEdit', 'Login_AccountLabel',
                            'Login_PasswordLabel', 'Login_RememberCheck' }) do
        shown(name)
    end
    hidden('Login_StatusText')
    hidden('Login_PasswordModeButton')
    eq(Login_LoginButton:getText(), 'Login', 'login button')

    Login_AccountEdit:setText(ACCOUNT)
    Login_PasswordEdit:setText('typed-pw')
    Stubs.press('Login_LoginButton')
    eq(#Stubs.calls, 1, 'accountLogin calls')
    eq(Stubs.calls[1].server, SERVER, 'stock server')
    eq(Stubs.calls[1].pass, 'typed-pw', 'stock password')

    Stubs.press('Login_WebsiteButton')
    Stubs.press('Login_AccountButton')
    Stubs.press('Login_ForumsButton')
    eq(Stubs.opened[1], STOCK_SITE, 'stock website')
    eq(Stubs.opened[2], STOCK_ACCOUNT_PAGE, 'stock account page')
    eq(Stubs.opened[3], STOCK_SITE, 'stock forums')
end }

cases[#cases + 1] = { name = 'failure, prompt outside ticket mode', run = function()
    boot(STOCK_INTERNAL)
    LoginMod.onLoginFailure(nil, nil, 'bad login', 13)
    eq(Stubs.prompts[1], '13 - bad login', 'stock prompt')

    boot(GOLDEN)
    Stubs.press('Login_PasswordModeButton')
    LoginMod.onLoginFailure(nil, nil, 'server text', 5)
    eq(Stubs.prompts[1], '5 - This account is suspended. Contact the server staff.', 'launcher text replaces the server text')
    eq(Login_LoginButton:getText(), 'Login', 'button stays Login')
end }

cases[#cases + 1] = { name = 'links', run = function()
    boot(plain(GOLDEN, 'CimmeriaLogin.links = {}',
        'CimmeriaLogin.links = {}\r\n    CimmeriaLogin.links.website = "https://example.org/site"'))
    shown('Login_WebsiteButton')
    hidden('Login_AccountButton')
    hidden('Login_ForumsButton')
    Stubs.press('Login_WebsiteButton')
    eq(#Stubs.opened, 1, 'opened links')
    eq(Stubs.opened[1], 'https://example.org/site', 'website link')
end }

cases[#cases + 1] = { name = 'old screen fallback', run = function()
    local uiDir = os.getenv('SGW_UI_DIR')
    if not uiDir or uiDir == '' then
        return 'skipped (SGW_UI_DIR is unset)'
    end
    local stock = assert(Stubs.readFile(uiDir..'/Startup/Login/Login.lua'),
        'SGW_UI_DIR has no Startup/Login/Login.lua: '..uiDir)
    boot(GOLDEN, stock)
    eq(Login_AccountEdit:getText(), ACCOUNT, 'account box on the old screen')
    eq(Login_PasswordEdit:getText(), TICKET, 'ticket box on the old screen')
end }

--=============================================================================
local failed = false
for _, case in ipairs(cases) do
    local okRun, result = pcall(case.run)
    if not okRun then
        print('FAIL '..case.name..': '..tostring(result))
        failed = true
        break
    end
    print('ok   '..case.name..(result and ' - '..result or ''))
end
if failed then
    os.exit(1)
end
print('017 login screen logic UAT: '..#cases..' cases')
