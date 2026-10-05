---
name: client-world-id-and-same-map-load
description: Client RE (2026-10-04, headless Ghidra) - onClientMapLoad skips the UE3 load when mapPath equals the loaded map; getCurrentWorldID comes from setupWorldParameters, not onClientMapLoad's WorldID
metadata:
  type: reference
---

Verified by decompiling SGW.exe headless (PR #1223 review, 2026-10-04).

- **onClientMapLoad** (`FUN_00df27f0`, GameProxyPlayer.cpp): reads areaName/mapPath/WorldID/Location/Direction.
  WorldID is read into a local and then overwritten without being used, so the client discards it.
  It travels to `127.0.0.1/<mapPath>.umap` only when `mapPath` differs from `GameProxyPlayer+0xdc`
  (`FUN_00424590` = wstring::compare), and it updates +0xdc only on that path. +0xdc is reset to `L""`
  only on Disconnect (`FUN_00def710`), so it survives gate travel and GM transfers.
  - Consequence: a cross-world transfer between two worlds with the same client map (SandBox/Harset_CmdCenter,
    DebugArea 1300/Ihpet_Crater_Light 73, two instances of one world via `.gotospace`) does not reload the
    level. Client-side level state (Kismet, map actors) carries over, and no loading screen shows.
  - Both branches GotoState('PlayerWaiting') and subscribe `FUN_00deea80` to Event_World_Loaded
    (`FUN_00df7b10` MemberCallback). `FUN_00deea80` emits Event_NetOut_ClientReady (`FUN_00d93d20`) and
    unsubscribes. World_Loaded comes from `APlayerController.IsWorldLoaded()` -> `FUN_005541a0` (guard
    `DAT_01ee2b6c`). So the handshake most likely completes without a reload. Not yet confirmed live.
- **getCurrentWorldID** (Lua) = `FUN_00ad70a0` -> GameWorldConstants singleton (`FUN_00c71790`) +0x48 =
  `mWorldId`, written only by the setupWorldParameters handler `FUN_00c71a20` (`worldId`).
  Lua consumers: Minimap.lua:44-46 (location text = `getWorldInfo(id).Name`), WorldMap.lua (current
  world, GVisitedWorlds), WorldMapMissions.lua:127, Squad.lua:238, WorldMapPOI.lua:17. Client UI Lua is at
  `..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI\Core\`.
- Shipped CookedWorldInfo `Flags="1"` on _12, _62, _68, _69, _70; 0 on open worlds and on CombatSim _1.
  No client Lua reads WorldInfo.Flags except Social.lua:181. Native readers have not been traced.

Related: [[message-id-00-direction-split]], [[viewport-system]].
