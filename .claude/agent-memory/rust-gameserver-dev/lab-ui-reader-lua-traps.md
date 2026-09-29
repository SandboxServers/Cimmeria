---
name: lab-ui-reader-lua-traps
description: Stock client UI Lua facts behind the cimmeria-lab UI readers (chat wrapper, inventory slots, right-click use, Ctrl-drag split) and how to offline-check reader chunks with lupa
metadata:
  type: reference
---

Facts from the stock UI Lua (`SGWGame/Content/UI/Core/*`) that the `crates/lab/src/supervisor/ui/` tools rely on (2026-09-29):

- `Container`, `Stat`, `UIChannel`, `UISpeakerFlags`, `UICraftType` are native tables: enumerate them with `pairs` at run time, never hard-code ids.
- Slots are 1-based; `InventoryMod.SlotWindows[1..40]`, visible slot = `slot - scroll*10` (All filter), tabs General/Mission/Crafting = Main/Mission/Crafting. Equipment: `CharacterMod.EquippedSlots[containerId]`, `CharacterMod.BandolierSlots[1..5]`.
- A **right-click** (button down + up on the same slot) calls `contextSensitiveUseItem` = use / equip / unequip. No double-click handler on inventory slots.
- Stack split is **Ctrl**-drag (buttonState 9, one item). Shift-drag is a stock `TODO` and does nothing.
- Loot All = `Loot_LootAllButton`; rows `Loot_ItemIcon_1..4` act on **double-click**; 4 per page.
- `window:subscribe(Events.X, 'name')` looks one-handler-per-window-per-event (`unsubscribe(event)` takes no handler). The lab keeps ONE chat capture: the `chat.line` ring in `supervisor/events/lua_rings.rs` (#1100); readers pump it into the event store and read through their own cursor (`client_chat_log` uses `chat_log:<name>`), never a second Lua wrapper or ring.
- Three `NativeLevel` enums exist (world, combat, ui); they share the N1/N2/N3 tier labels, not the type. Supervisor method names collide across tool modules (`player_state`, `press_binding`): prefix new ones.
- Chat: `ChatMod.onMessageReceived(this, speaker, flags, channelId, channelName, text)`; Server channel also opens a prompt; ChannelMap[7] (tell) is nil in some builds, so pcall the mapping.

Offline check: dump every reader chunk to files from a temporary test, then run them in `lupa.lua51` (`pip install lupa`, `LuaRuntime(unpack_returned_tuples=True)`) against a mock of the bindings. Catches Lua syntax/runtime errors before a live client. Worktree Bash refuses heredoc appends and `export X && bash ...`: hardcode the dump dir in the temp test and use the Edit tool.

Related: [[lab-probe-traffic-starves-watchdog]].
