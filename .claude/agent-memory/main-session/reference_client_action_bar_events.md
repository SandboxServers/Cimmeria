---
name: client-action-bar-events
description: How the stock ActionButtons Lua hears events and binds buttons - DeafWhenHidden, multi-handler subscribe, bandolier-bound buttons, the weapon-switch event; learned building client patch 015
metadata:
  type: reference
---

Read from the stock client's `Content/UI/Core/ActionButtons/*.lua` and layouts on 2026-10-05 while writing `015-weapon-shot-bar`. Nothing here was observed in a running client.

- **A hidden window is deaf unless its layout says otherwise.** 91 windows in 80 stock layouts set `<Property Name="DeafWhenHidden" Value="False" />`; every hidden window that subscribes to a game event has it (`AbilityWin`, `CharacterWin`, `WorldMapWin`, `ActionButton_DragContainer`). `ActionButtons_ButtonDragContainer` is hidden and does not have it. Pick a listener window by that property, or the handler never runs.
- **`window:subscribe(event, 'Mod.fn')` takes more than one handler per event.** `SelfStatusWin` and `WorldMapWin` each subscribe `Events.ModuleLoaded` twice with different handlers, and the stock UI needs both. `window:unsubscribe(event)` takes no handler, so treat it as removing them all: never share a (window, event) pair with code that unsubscribes. 009 does that on `ActionButtonsWin` for `Events.AbilityUpdate` and `Events.PropertyUpdated`.
- **Whether a handler name is resolved at subscribe time or at dispatch is not known.** To hook a stock handler, wrap a function it calls by table lookup (for example `ActionProfileMod.refreshCurrentProfile`, called from `onContainerSlotActivated`), not the handler itself.
- **The weapon switch has a client event:** `Events.InventoryUpdateContainerActiveSlot(containerId)`, with `Container.Bandolier`. The stock bar, Bandolier, Character and WeaponBar modules all listen to it.
- **Buttons have three bindings** (`ActionProfileMod.ButtonBinding_Never`, `_Bandolier`, `_Layer`). A bandolier-bound button keeps one action per weapon, keyed by `getItemIDForSlot(Container.Bandolier, getActiveSlotForContainer(Container.Bandolier))`, an item instance id, and the stock UI swaps it on the event above. The UI has no native that gives an item's type id, only name, icon, quality and tech competency per slot.
- **Dropping an ability on an occupied button keeps the button's action id** and re-points the action (`ActionProfileMod.receiveDrag` then `ActionButtonMod.receiveDrag`). `setActionToAbility(actionId, abilityId)` on an existing action is therefore the stock way to change what a button does; the profile data (`buttonInfo[n].actions[...]`) does not change.
- `getAbilityInfo(id)` has `isWeaponAbility` and `isDeployAbility` (they pick the button shape). Not checked: what sets them.

Related: [reference_client_ui_lua_overlay_testing.md](reference_client_ui_lua_overlay_testing.md), [reference_client_patch_delivery.md](reference_client_patch_delivery.md), and the rust-gameserver-dev note `client-action-bar-is-client-side.md`.
