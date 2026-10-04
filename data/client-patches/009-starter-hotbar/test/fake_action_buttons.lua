-- A clean-room model of the ActionButtons module's public contract, written
-- for this test. It is not CME code and copies none of it. CI has no client,
-- so CI runs the hook against this model; run.lua also runs it against the
-- real stock and WQHD v26 scripts when their paths are given.
--
-- Modelled: the three button bindings, a two-profile-template table whose
-- template 2 has buttons 1-10 bandolier-bound, 11-20 layer-bound and 21-23
-- never-bound; createProfile, loadProfile, onModLoaded, getButtonInfo,
-- getButtonWindow, setButtonCurrentAction, and the button binding and redraw
-- calls the hook uses.

ActionButtonMod = { buttons = {}, actionMap = {} }

function ActionButtonMod.registerButton( buttonId, buttonWindow, countWindow, keybindWindow, cooldownWindow )
    ActionButtonMod.buttons[buttonId] = {
        buttonId = buttonId, buttonWindow = buttonWindow,
        cooldownImage = 'CoreMaterial_ButtonTimer'..buttonId,
    }
    buttonWindow:setID(buttonId)
    buttonWindow:subscribe(buttonWindow.EventClicked, 'ActionButtonMod.onActionPress')
end

function ActionButtonMod.redraw( buttonInfo, actionId )
    local info = getActionInfo(actionId or 0)
    setMaterialTextureProperty(buttonInfo.cooldownImage, 'Icon', info.id and info.icon or '')
end

function ActionButtonMod.unbindButton( buttonId )
    local b = ActionButtonMod.buttons[buttonId]
    if not b then return end
    if b.actionId then
        ActionButtonMod.actionMap[b.actionId][buttonId] = nil
    end
    b.actionId = nil
    ActionButtonMod.redraw(b, 0)
end

function ActionButtonMod.bindButtonToAction( buttonId, actionId )
    local b = ActionButtonMod.buttons[buttonId]
    if not b then return end
    ActionButtonMod.unbindButton(buttonId)
    b.actionId = actionId
    ActionButtonMod.actionMap[actionId] = ActionButtonMod.actionMap[actionId] or {}
    ActionButtonMod.actionMap[actionId][buttonId] = buttonId
    ActionButtonMod.redraw(b, actionId)
end

function ActionButtonMod.getActionForButton( buttonId )
    local b = ActionButtonMod.buttons[buttonId]
    if not b then return 0 end
    return b.actionId
end

function ActionButtonMod.onActionPress( this )
    local actionId = ActionButtonMod.getActionForButton(this:getID())
    if actionId and actionId ~= 0 then
        useAction(actionId, false)
    end
end

function ActionButtonMod.onActionUpdated( this, actionId )
    for _, buttonId in pairs(ActionButtonMod.actionMap[actionId] or {}) do
        ActionButtonMod.redraw(ActionButtonMod.buttons[buttonId], actionId)
    end
end

ActionProfileMod = {
    MaxButtons = 100, SafetyTemplate = 2, DefaultTemplates = {},
    ButtonBinding_Never = 1, ButtonBinding_Bandolier = 2, ButtonBinding_Layer = 3,
}
GActionProfiles = {}
GActionCurrentProfileId = 1

local function copy( v )
    if type(v) ~= 'table' then return v end
    local t = {}
    for k, x in pairs(v) do t[copy(k)] = copy(x) end
    return t
end

function ActionProfileMod.createProfile( profileName, templateId )
    local profileId = table.maxn(GActionProfiles) + 1
    GActionProfiles[profileId] = copy(ActionProfileMod.DefaultTemplates[templateId])
    if not GActionProfiles[profileId] then return -1 end
    GActionProfiles[profileId].nameText = profileName
    return profileId
end

function ActionProfileMod.getButtonWindow( buttonId )
    local b = ActionButtonMod.buttons[buttonId]
    return b and b.buttonWindow or nil
end

function ActionProfileMod.getButtonInfo( buttonId )
    local p = GActionProfiles[GActionCurrentProfileId]
    return p and p.buttonInfo[buttonId] or nil
end

function ActionProfileMod.actionFor( profile, info )
    if info.binding == ActionProfileMod.ButtonBinding_Never then
        return info.actions[1]
    elseif info.binding == ActionProfileMod.ButtonBinding_Layer then
        return info.actions[profile.currentLayer]
    end
    return info.actions[getItemIDForSlot(Container.Bandolier, getActiveSlotForContainer(Container.Bandolier))]
end

function ActionProfileMod.loadProfile( profileId )
    local p = GActionProfiles[profileId]
    if p == nil then
        p = GActionProfiles[1]
        if p == nil then
            p = GActionProfiles[ActionProfileMod.createProfile('Default', ActionProfileMod.SafetyTemplate)]
        end
    end
    GActionCurrentProfileId = profileId
    p.currentLayer = p.currentLayer or 1
    for buttonId in pairs(ActionButtonMod.buttons) do
        local info = p.buttonInfo[buttonId]
        local actionId = info and ActionProfileMod.actionFor(p, info)
        if actionId and actionId > 0 then
            ActionButtonMod.bindButtonToAction(buttonId, actionId)
        else
            ActionButtonMod.unbindButton(buttonId)
        end
    end
end

function ActionProfileMod.setButtonCurrentAction( buttonId, actionId )
    local info = ActionProfileMod.getButtonInfo(buttonId)
    if not info then return end
    local p = GActionProfiles[GActionCurrentProfileId]
    if info.binding == ActionProfileMod.ButtonBinding_Never then
        info.actions[1] = actionId
    elseif info.binding == ActionProfileMod.ButtonBinding_Layer then
        info.actions[p.currentLayer] = actionId
    else
        info.actions[getItemIDForSlot(Container.Bandolier, getActiveSlotForContainer(Container.Bandolier))] = actionId
    end
    ActionButtonMod.bindButtonToAction(buttonId, actionId)
end

function ActionProfileMod.onModLoaded( window )
    for i = 1, ActionProfileMod.MaxButtons do
        ActionButtonMod.registerButton(i, _G[string.format('ActionButtons_%dButton', i)])
    end
    if not GActionProfiles[GActionCurrentProfileId] then
        GActionCurrentProfileId = ActionProfileMod.createProfile('Default', ActionProfileMod.SafetyTemplate)
    end
    ActionProfileMod.loadProfile(GActionCurrentProfileId)
    BackgroundMod.registerDropCallback(UIDragType.Ability, function() end)
end

function ActionProfileMod.onActionUpdated( this, actionId )
    ActionButtonMod.onActionUpdated(this, actionId)
end

ActionButtonsWin:subscribe(Events.ModuleLoaded, 'ActionProfileMod.onModLoaded')
ActionButtonsWin:subscribe(Events.ActionUpdated, 'ActionProfileMod.onActionUpdated')

-- The templates (the real client defines these in ActionProfileDefault1.lua).
ActionProfileMod.DefaultTemplates[1] = { currentLayer = 1, nameText = 'Blank', buttonInfo = {}, dockGroups = {} }
local t = { currentLayer = 1, nameText = 'Default 1', buttonInfo = {}, dockGroups = { {}, {}, {} } }
for i = 1, 23 do
    local binding, group = ActionProfileMod.ButtonBinding_Bandolier, 1
    if i > 20 then
        binding, group = ActionProfileMod.ButtonBinding_Never, 3
    elseif i > 10 then
        binding, group = ActionProfileMod.ButtonBinding_Layer, 2
    end
    t.buttonInfo[i] = { binding = binding, posX = 36 * i, posY = 700, actions = {}, dockGroup = group }
    t.dockGroups[group][i] = true
end
ActionProfileMod.DefaultTemplates[2] = t
