-- Global table to avoid name clashes
AccessMod = {}

--The button list is one-per-row for ease in reordering them (and adding new ones where we want)
AccessMod.Buttons = {
    Access_CharacterSheetButton,
    Access_AbilitiesButton,
    Access_WorldMapButton,
    Access_KeyBindingsButton,
    Access_CustomizeUIButton,
    Access_OptionsButton,
    Access_SupportButton,
    Access_LogoutButton,
    Access_MissionLogButton,
    Access_SocialButton,
    Access_MailButton,
    Access_CraftingButton,
    Access_InventoryButton,
}

AccessMod.ButtonSetOne = {
    Radius = 170,
    StartAngle = 90,
    Buttons = AccessMod.Buttons
}

AccessMod.ButtonSetTwo = {
    Radius = 120,
    StartAngle = 90,
    Buttons = {
        Access_CharacterSheetButton,
        Access_AbilitiesButton,
        Access_OptionsButton,
        Access_LogoutButton,
        Access_MissionLogButton,
        Access_MailButton,
    }
}

AccessMod.ButtonStripOne = {
    Access_AbilitiesButton,
    Access_CharacterSheetButton,
    Access_InventoryButton,
    Access_MissionLogButton,
    Access_CustomizeUIButton,
    Access_OptionsButton,
    Access_SocialButton,
    Access_MailButton,
    Access_WorldMapButton,
    Access_SupportButton,
}


function AccessMod.onModuleLoaded( this,name )
    if (name ~= "Access") then return end
end

function AccessMod.RepositionButtonsRadially(ButtonSet)
    local numButtons = #ButtonSet.Buttons
    local numAllButtons = #AccessMod.Buttons
    local angleIncrement = -360.0 / numButtons --Negative so it goes clockwise around the ring.
    local currentAngle = ButtonSet.StartAngle
    local bx,by

    --Hide all buttons
    for i=1,numAllButtons do
        AccessMod.Buttons[i]:hide()
    end

    --Iterate through the buttons and position them all with sin and cos.
    --Also, show the included buttons.
    for i=1,numButtons do
        bx =  math.cos(math.rad(currentAngle)) * ButtonSet.Radius
        by = -math.sin(math.rad(currentAngle)) * ButtonSet.Radius

        ButtonSet.Buttons[i]:setPosition(CEGUI.UVector2(CEGUI.UDim(0,bx),CEGUI.UDim(0,by)))
        ButtonSet.Buttons[i]:show()

        currentAngle = currentAngle + angleIncrement
    end
end

function AccessMod.RepositionButtonsLinearly(ButtonStrip)
    local numButtons = #ButtonStrip
    local buttonWidth = ButtonStrip[1]:getPixelSize().width
    local buttonSpacing = 2

    --Hide all buttons
    for _,b in pairs(AccessMod.Buttons) do
        b:hide()
    end

    --Iterate through the buttons in this strip and position them from left to right.
    local bLeft = -(numButtons * (buttonWidth + buttonSpacing) - buttonSpacing)/ 2
    for i,b in pairs(ButtonStrip) do
        b:setPosition(CEGUI.UVector2(CEGUI.UDim(0.5,bLeft),CEGUI.UDim(0,0)))
        bLeft = bLeft + buttonWidth + buttonSpacing
        b:show()
    end
end

function AccessMod.onToggleVisibility( this )
    --This "AlwaysOnTop" hack is needed to make the tooltips visible when you hover over a button.
    AccessWin:setProperty("AlwaysOnTop","true")
    --AccessMod.RepositionButtons(AccessMod.ButtonSetOne) --Specify which buttonset to display.
    AccessMod.RepositionButtonsLinearly(AccessMod.ButtonStripOne)
    AccessWin:setVisible(not AccessWin:isVisible())
    AccessWin:setProperty("AlwaysOnTop","false")
    if AccessWin:isVisible() then
        triggerTutorialDialogId( UITutorial.AccessBarOpened )
    end
end

--The button-click event handlers.

function AccessMod.onSupportClicked( this )
    openWebsite("http://stargate-union.com/sgw/StargateWorldsGuide.html")
    AccessWin:hide()
end

function AccessMod.onOptionsClicked( this )
    OptionsMod.onToggleOptions()
    AccessWin:hide()
end

function AccessMod.onKeyBindingsClicked( this )
    OptionsMod.onToggleBindings()
    AccessWin:hide()
end

function AccessMod.onCustomizeUIClicked( this )
    ActionProfileMod.onToggleEditMode()
    AccessWin:hide()
end

function AccessMod.onLogoutClicked( this )
    logOff()
    AccessWin:hide()
end

function AccessMod.onCharacterSheetClicked( this )
    CharacterWin:moveToFront()
    CharacterWin:setVisible(not CharacterWin:isVisible())
    AccessWin:hide()
end

function AccessMod.onMissionLogClicked( this )
    MissionLogWin:moveToFront()
    MissionLogWin:setVisible(not MissionLogWin:isVisible())
    AccessWin:hide()
end

function AccessMod.onWorldMapClicked( this )
    WorldMapWin:moveToFront()
    WorldMapMod.onToggleWorldMap(WorldMapWin)
    AccessWin:hide()
end

function AccessMod.onInventoryClicked( this )
    InventoryWin:moveToFront()
    InventoryWin:setVisible(not InventoryWin:isVisible())
    AccessWin:hide()
end

function AccessMod.onAbilitiesClicked( this )
    AbilityWin:moveToFront()
    AbilityWin:setVisible(not AbilityWin:isVisible())
    AccessWin:hide()
end

function AccessMod.onCraftingClicked( this )
    CraftingMod.onToggleCrafting(CraftingWin)
    AccessWin:hide()
end

function AccessMod.onSocialClicked( this )
    -- Shipped as an empty TODO. Open the Social window the way its O key
    -- binding does (Actions.ToggleSocial), like the Crafting button above.
    SocialMod.onToggleSocial(SocialWin)
    AccessWin:hide()
end

function AccessMod.onMailClicked( this )
    GateMailMod.onToggleMailbox()
    AccessWin:hide()
end


--=============================================================================
-- Register Event Handlers
AccessWin:subscribe(Events.ModuleLoaded,'AccessMod.onModuleLoaded')
AccessWin:subscribe(Actions.ToggleAccessWindow,'AccessMod.onToggleVisibility')

Access_SupportButton:subscribe(Access_SupportButton.EventClicked, 'AccessMod.onSupportClicked')
Access_OptionsButton:subscribe(Access_OptionsButton.EventClicked, 'AccessMod.onOptionsClicked')
Access_KeyBindingsButton:subscribe(Access_KeyBindingsButton.EventClicked, 'AccessMod.onKeyBindingsClicked')
Access_CustomizeUIButton:subscribe(Access_CustomizeUIButton.EventClicked, 'AccessMod.onCustomizeUIClicked')
Access_LogoutButton:subscribe(Access_LogoutButton.EventClicked,'AccessMod.onLogoutClicked')
Access_CharacterSheetButton:subscribe(Access_CharacterSheetButton.EventClicked,'AccessMod.onCharacterSheetClicked')
Access_MissionLogButton:subscribe(Access_MissionLogButton.EventClicked,'AccessMod.onMissionLogClicked')
Access_WorldMapButton:subscribe(Access_WorldMapButton.EventClicked,'AccessMod.onWorldMapClicked')
Access_InventoryButton:subscribe(Access_InventoryButton.EventClicked,'AccessMod.onInventoryClicked')
Access_AbilitiesButton:subscribe(Access_AbilitiesButton.EventClicked,'AccessMod.onAbilitiesClicked')
Access_CraftingButton:subscribe(Access_CraftingButton.EventClicked,'AccessMod.onCraftingClicked')
Access_SocialButton:subscribe(Access_SocialButton.EventClicked,'AccessMod.onSocialClicked')
Access_MailButton:subscribe(Access_MailButton.EventClicked,'AccessMod.onMailClicked')
