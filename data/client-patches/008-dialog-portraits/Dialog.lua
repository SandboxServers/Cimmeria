
DialogMod.dialogDialogId = 0
DialogMod.dialogButtonMap = {}
DialogMod.dialogButtonMap[Dialog.ButtonAcceptType] = Dialog_AcceptButton
--DialogMod.dialogButtonMap[Dialog.ButtonDeclineType] = Dialog_DeclineButton
DialogMod.dialogButtonMap[Dialog.ButtonGeneric1Type] = Dialog_GenericButton1
DialogMod.dialogButtonMap[Dialog.ButtonGeneric2Type] = Dialog_GenericButton2
DialogMod.dialogButtonMap[Dialog.ButtonGeneric3Type] = Dialog_GenericButton3

--=============================================================================
function DialogMod.initDialog( dialogId )
    DialogMod.dialogDialogId = dialogId

    Dialog_ScreenText:setText( getActiveDialogText(DialogMod.dialogDialogId) )
    DialogMod.enableDialogButtons( DialogMod.dialogDialogId, DialogMod.dialogButtonMap )
    
    -- Only show a Decline button if there is an Accept button
    Dialog_DeclineButton:setVisible( Dialog_AcceptButton:isVisible() )
    
    local AcceptVisible = Dialog_AcceptButton:isVisible(true)
    --// Set up the Next/Previous correctly
    if getActiveDialogIndex(DialogMod.dialogDialogId) < getActiveDialogMaxIndex(DialogMod.dialogDialogId) or AcceptVisible then
        Dialog_NextPrevWindow:show()
        Dialog_PrevOnlyWindow:hide()
        if getActiveDialogIndex(DialogMod.dialogDialogId) == getActiveDialogMaxIndex(DialogMod.dialogDialogId) and AcceptVisible then
            Dialog_NextButton:setEnabled(false)
        else
            Dialog_NextButton:setEnabled(true)
        end
    else
        Dialog_NextPrevWindow:hide()
        Dialog_PrevOnlyWindow:show()
    end
    if getActiveDialogIndex(DialogMod.dialogDialogId) == 1 then
        Dialog_PrevButton1:setEnabled(false)
        Dialog_PrevButton2:setEnabled(false)
    else
        Dialog_PrevButton1:setEnabled(true)
        Dialog_PrevButton2:setEnabled(true)
    end
    
    Dialog_NameText:setText( unitName(Unit.Dialog) )
    --// Cimmeria: render the speaker into the portrait disk (CME left this widget unwired).
    --// CoreMaterial_DialogPortrait must be declared in CEGUIData/schemes/TaharezLook.scheme, or
    --// createCharacterPortrait errors. pcall keeps a portrait failure from blocking the window.
    if Dialog_PortraitImage ~= nil then
        local ok = pcall(createCharacterPortrait, "CoreMaterial_DialogPortrait", "Portrait", Unit.Dialog, UIPortraitStyle.Face, UIPortraitUpdate.OneShot, 128, 128)
        if ok then
            pcall(function()
                Dialog_PortraitImage:setProperty("Image", "set:CoreMaterial_DialogPortrait image:full_image")
                Dialog_PortraitImage:show()
            end)
        else
            pcall(function() Dialog_PortraitImage:hide() end)
        end
    end
    --// TODO: Adjust the size of the screen text window depending on how many generic options are shown
    
end

--=============================================================================
function DialogMod.onDialogChoiceClicked( this, window )
    selectActiveDialogChoice(DialogMod.dialogDialogId, this:getID())
    DialogWin:hide()
end

--=============================================================================
function DialogMod.onDialogNextClicked( this, window )
    moveActiveDialogNext(DialogMod.dialogDialogId)
end

--=============================================================================
function DialogMod.onDialogPrevClicked( this, window )
    moveActiveDialogPrevious(DialogMod.dialogDialogId)
end

--=============================================================================
function DialogMod.onDialogDoneClicked( this, window )
    DialogWin:hide()
    discardAvailableDialog(DialogMod.dialogDialogId)
end

--=============================================================================

DialogMod.registerDialogType( Dialog.DialogType, DialogWin, DialogMod.initDialog )
DialogMod.registerDialogType( Dialog.RadioType, DialogWin, DialogMod.initDialog )
DialogMod.registerDialogType( Dialog.RealizationType, DialogWin, DialogMod.initDialog )

--// Register Event Handlers
for key, button in pairs(DialogMod.dialogButtonMap) do
    button:subscribe( button.EventClicked, 'DialogMod.onDialogChoiceClicked' )
end

Dialog_NextButton:subscribe( Dialog_NextButton.EventClicked, 'DialogMod.onDialogNextClicked' )
Dialog_PrevButton1:subscribe( Dialog_PrevButton1.EventClicked, 'DialogMod.onDialogPrevClicked' )
Dialog_PrevButton2:subscribe( Dialog_PrevButton2.EventClicked, 'DialogMod.onDialogPrevClicked' )
Dialog_DoneButton:subscribe( Dialog_DoneButton.EventClicked, 'DialogMod.onDialogDoneClicked' )
Dialog_DeclineButton:subscribe( Dialog_DeclineButton.EventClicked, 'DialogMod.onDialogDoneClicked' )
DialogWin:subscribe( DialogWin.EventCloseClicked, 'DialogMod.onDialogDoneClicked' )

