

DialogMod.blurbDialogId = 0
DialogMod.blurbButtonMap = {}
DialogMod.blurbButtonMap[Dialog.ButtonMoreInfoType] = Blurb_MoreInfoButton
DialogMod.blurbButtonMap[Dialog.ButtonAcceptType] = Blurb_AcceptButton
--DialogMod.blurbButtonMap[Dialog.ButtonDeclineType] = Blurb_DeclineButton

--=============================================================================
function DialogMod.initBlurb( dialogId )
    DialogMod.blurbDialogId = dialogId

    Blurb_ScreenText:setText( getActiveDialogText(dialogId) )
    DialogMod.enableDialogButtons( dialogId, DialogMod.blurbButtonMap )
    
    -- Only show a Decline button if there is an Accept button
    Blurb_DeclineButton:setVisible( Blurb_AcceptButton:isVisible() )
    
    Blurb_NameText:setText( unitName(Unit.Dialog) )
    --// Cimmeria: render the speaker into the portrait disk (CME left this widget unwired).
    --// CoreMaterial_BlurbPortrait must be declared in CEGUIData/schemes/TaharezLook.scheme, or
    --// createCharacterPortrait errors. pcall keeps a portrait failure from blocking the window.
    if Blurb_PortraitImage ~= nil then
        local ok = pcall(createCharacterPortrait, "CoreMaterial_BlurbPortrait", "Portrait", Unit.Dialog, UIPortraitStyle.Face, UIPortraitUpdate.OneShot, 128, 128)
        if ok then
            pcall(function()
                Blurb_PortraitImage:setProperty("Image", "set:CoreMaterial_BlurbPortrait image:full_image")
                Blurb_PortraitImage:show()
            end)
        else
            pcall(function() Blurb_PortraitImage:hide() end)
        end
    end
end

--=============================================================================
function DialogMod.onBlurbChoiceClicked( this, window )
    selectActiveDialogChoice(DialogMod.blurbDialogId, this:getID())
    BlurbWin:hide()
end

--=============================================================================
function DialogMod.onBlurbClosed( this, window )
    BlurbWin:hide()
    discardAvailableDialog(DialogMod.blurbDialogId)
end

--=============================================================================
DialogMod.registerDialogType( Dialog.BlurbType, BlurbWin, DialogMod.initBlurb )

-- TEMP HACK TEMP HACK
DialogMod.registerDialogType( 0, BlurbWin, DialogMod.initBlurb )

--// Register Event Handlers
for key, button in pairs(DialogMod.blurbButtonMap) do
    button:subscribe( button.EventClicked, 'DialogMod.onBlurbChoiceClicked' )
end

BlurbWin:subscribe(BlurbWin.EventCloseClicked, 'DialogMod.onBlurbClosed')
BlurbWin:subscribe(Blurb_DeclineButton.EventClicked, 'DialogMod.onBlurbClosed')
