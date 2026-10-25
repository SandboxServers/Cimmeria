-- default value for the saved user name
GSavedVar_AccountName = ""
GSavedVar_ShouldSaveAccount = true

local versionInfo = getVersionInfo()

Login_VersionText:setText(versionInfo.FileVersion)

--=============================================================================
if nil == LoginMod.loadServerSystems then
    LoginMod = {}
    function LoginMod.loadServerSystems()
        LoginMod.servers = {}
        LoginMod.servers["Live"] = "172.25.159.239"
    end
end

-- Version of this overlay (017). LoginInternal.lua checks it before it signs
-- the screen in from a launcher sign-in.
LoginMod.cimmeriaOverlayVersion = 1

--=============================================================================
function LoginMod.startup()
    -- set the text to the saved value
    if GSavedVar_ShouldSaveAccount then
        Login_RememberCheck:setSelected(true)
        Login_AccountEdit:setText(GSavedVar_AccountName)
        Login_PasswordEdit:activate()
    else    
        Login_RememberCheck:setSelected(false)
        Login_AccountEdit:setText("")
        Login_AccountEdit:activate()
    end        

    -- Only show this if there is news
    Login_NewsText:hide()

    LoginMod.loadServerSystems()
    
    local firstEntry
    local count = 0
    for key, server in pairs(LoginMod.servers) do
        if firstEntry then
            LoginMod.addServer(key, server)
        else
            -- Save the first one
            firstEntry = LoginMod.addServer(key, server)
        end
        count = count + 1
    end

    if firstEntry then
        Login_ServerTypeCombo:setItemSelectState(firstEntry, true)
        Login_ServerTypeCombo:setText(firstEntry:getText())
    end
    
    if count <= 1 then
        Login_ServerTypeCombo:hide()
    end

    LoginMod.applyCimmeria()
end

--=============================================================================
-- A launcher sign-in (CimmeriaLogin, written for this launch) replaces the
-- name and password boxes with the signed-in name and an Enter button.
function LoginMod.isTicketMode()
    return nil ~= CimmeriaLogin and CimmeriaLogin.version == 1 and CimmeriaLogin.mode == "ticket" and not LoginMod.cimmeriaPasswordMode
end

--=============================================================================
function LoginMod.applyCimmeria()
    if not LoginMod.isTicketMode() then
        return
    end

    Login_AccountEdit:hide()
    Login_PasswordEdit:hide()
    Login_AccountLabel:hide()
    Login_PasswordLabel:hide()
    Login_RememberCheck:hide()
    Login_ServerTypeCombo:hide()

    Login_StatusText:setText("Signed in as " .. CimmeriaLogin.display .. " (" .. CimmeriaLogin.account .. ")")
    Login_StatusText:show()
    Login_LoginButton:setText("Enter")
    Login_PasswordModeButton:show()

    -- a link the launcher did not return has no button
    if nil == CimmeriaLogin.links.website then
        Login_WebsiteButton:hide()
    end
    if nil == CimmeriaLogin.links.account then
        Login_AccountButton:hide()
    end
    if nil == CimmeriaLogin.links.forums then
        Login_ForumsButton:hide()
    end

    if CimmeriaLogin.auto and nil == LoginMod.cimmeriaAutoTried then
        LoginMod.cimmeriaAutoTried = true
        LoginMod.submitTicket()
    end
end

--=============================================================================
-- Signs in with the launcher's ticket. The account name is never saved.
function LoginMod.submitTicket()
    Login_StatusText:setText("Connecting...")
    accountLogin(LoginMod.servers[CimmeriaLogin.server], CimmeriaLogin.account, CimmeriaLogin.ticket)
end

--=============================================================================
function LoginMod.onLoginClicked( this, window )
    if LoginMod.isTicketMode() then
        LoginMod.submitTicket()
        return
    end

    -- get the username and password and server
    local username = Login_AccountEdit:getText()
    local password = Login_PasswordEdit:getText()
    
    -- Determine which system to log into
    local server
    if ( Login_ServerTypeCombo ) then
        local selectedItem = Login_ServerTypeCombo:getSelectedItem()
        if ( selectedItem ) then
            server = LoginMod.servers[selectedItem:getText()]
        end
    end
    
    if  nil == server then
        for key, value in pairs(LoginMod.servers) do
            server = value
        end
    end
    
    -- don't login with empty username or password
    if 0 == #username or 0 == #password or nil == server then
        return
    end

    -- Save the username between sessions
    if ( Login_RememberCheck:isSelected() ) then
        GSavedVar_AccountName = username
        GSavedVar_ShouldSaveAccount = true
    else    
        GSavedVar_AccountName = ""
        GSavedVar_ShouldSaveAccount = false
    end

    Login_PasswordEdit:setText( '' )

    -- try to login
    accountLogin(server, username, password)
end

--=============================================================================
function LoginMod.onQuitClicked( this, window )
    quit()    
end

--=============================================================================
function LoginMod.addServer( name, ipString )
    if ( Login_ServerTypeCombo ) then
        local entry = CEGUI.createListboxTextItem(name)
        entry:setSelectionColours(CEGUI.colour(0.0, 0.0, 1.0, 1.0))
        entry:setSelectionBrushImage("TaharezLook", "ListboxSelectionBrush")
        Login_ServerTypeCombo:addItem(entry)
        return entry
    end        
end

--=============================================================================
function LoginMod.onEditKeyPress( this, window, codePoint, scanCode, sysKeys )
    if scanCode == CEGUI.Key.Return then
        LoginMod.onLoginClicked( this, nil )
    elseif scanCode == CEGUI.Key.Tab then
        if Login_AccountEdit:isActive() then
           Login_PasswordEdit:activate()
        else
           Login_AccountEdit:activate()
        end
    else    
        return false        
    end
end

--=============================================================================
function LoginMod.onEditActivate(this, window, otherWindow)
    -- Select all of the text and put the cursor at the end of it
    window:setSelection(0, 99)
    window:setCaratIndex(99)
end

--=============================================================================
function LoginMod.onEditDeactivate(this, window, otherWindow)
    -- Select all of the text and put the cursor at the end of it
    window:setSelection(0, 0)
end

--=============================================================================
function LoginMod.onLoginFailure( this, window, errorStr, errorNum )
    if LoginMod.isTicketMode() then
        Login_LoginButton:setText("Retry")
        if errorNum and CimmeriaLogin.errors[errorNum] then
            Login_StatusText:setText(CimmeriaLogin.errors[errorNum])
        else
            Login_StatusText:setText("Login failed (" .. tostring(errorNum) .. "). Keep the launcher open, then press Retry.")
        end
        return
    end

    -- the launcher's text for this error replaces the server's, when it has one
    if errorNum and CimmeriaLogin and CimmeriaLogin.errors[errorNum] then
        errorStr = CimmeriaLogin.errors[errorNum]
    end

    if errorNum then	
        PromptMod.showPrompt( localize("Login","Login Failure"), errorNum.." - "..errorStr, localize("Global","Ok"), nil, nil, nil, true )
    else
        PromptMod.showPrompt( localize("Login","Login Failure"), errorStr, localize("Global","Ok"), nil, nil, nil, true )
    end	
end

--=============================================================================
function LoginMod.onWebsiteButtonClicked( this, window )
    LoginMod.openLink("website", "http://projectgiza.com")
end

--=============================================================================
function LoginMod.onManageAccountButtonClicked( this, window )
    LoginMod.openLink("account", "http://projectgiza.com/forums/viewtopic.php?f=4&t=51")
end

--=============================================================================
function LoginMod.onForumButtonClicked( this, window )
    LoginMod.openLink("forums", "http://projectgiza.com")
end

--=============================================================================
-- Opens the launcher's link when there is a launcher sign-in, else the stock URL.
function LoginMod.openLink( name, stockUrl )
    local url = stockUrl
    if CimmeriaLogin then
        url = CimmeriaLogin.links[name]
    end
    if url then
        openWebsite(url)
    end
end

--=============================================================================
-- Leaves ticket mode for this session and shows the stock boxes again.
function LoginMod.onPasswordModeClicked( this, window )
    LoginMod.cimmeriaPasswordMode = true

    Login_AccountLabel:show()
    Login_PasswordLabel:show()
    Login_AccountEdit:show()
    Login_PasswordEdit:show()
    Login_RememberCheck:show()

    local count = 0
    for _ in pairs(LoginMod.servers) do
        count = count + 1
    end
    if count > 1 then
        Login_ServerTypeCombo:show()
    end

    Login_AccountEdit:setText("")
    Login_PasswordEdit:setText("")
    Login_LoginButton:setText("Login")
    Login_StatusText:hide()
    Login_PasswordModeButton:hide()
end

--=============================================================================
function LoginMod.onOptionsClicked( this, window )
	OptionsMod.onToggleOptions()
end

--=============================================================================
--LoginWin:subscribe( LoginWin.EventShown, 'LoginMod.onLoginShow')
Login_LoginButton:subscribe(Login_LoginButton.EventClicked, 'LoginMod.onLoginClicked')
Login_QuitButton:subscribe(Login_QuitButton.EventClicked, 'LoginMod.onQuitClicked')
Login_OptionsButton:subscribe(Login_OptionsButton.EventClicked, 'LoginMod.onOptionsClicked')

Login_AccountEdit:subscribe( Login_AccountEdit.EventKeyDown, 'LoginMod.onEditKeyPress' )
Login_AccountEdit:subscribe( Login_AccountEdit.EventActivated, 'LoginMod.onEditActivate' )
Login_AccountEdit:subscribe( Login_AccountEdit.EventDeactivated, 'LoginMod.onEditDeactivate' )

Login_PasswordEdit:subscribe( Login_PasswordEdit.EventKeyDown, 'LoginMod.onEditKeyPress' )
Login_PasswordEdit:subscribe( Login_PasswordEdit.EventActivated, 'LoginMod.onEditActivate' )
Login_PasswordEdit:subscribe( Login_PasswordEdit.EventDeactivated, 'LoginMod.onEditDeactivate' )

LoginWin:subscribe(Events.LoginFailed, 'LoginMod.onLoginFailure')

Login_WebsiteButton:subscribe(Login_WebsiteButton.EventClicked, 'LoginMod.onWebsiteButtonClicked')
Login_AccountButton:subscribe(Login_AccountButton.EventClicked, 'LoginMod.onManageAccountButtonClicked')
Login_ForumsButton:subscribe(Login_ForumsButton.EventClicked, 'LoginMod.onForumButtonClicked')

Login_PasswordModeButton:subscribe(Login_PasswordModeButton.EventClicked, 'LoginMod.onPasswordModeClicked')

LoginWin:hide()
