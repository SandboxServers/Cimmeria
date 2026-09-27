---
name: client-action-bar-is-client-side
description: The SGW action bar (hotbar bindings) is a client-side Lua saved variable; the server cannot read or edit it, and "hotbar" in server code means onKnownAbilitiesUpdate
metadata:
  type: project
---

The client stores action-bar bindings in `GActionProfiles`, a `<CharacterVariable>` declared in `Content/UI/Core/ActionButtons/ActionButtons.toc`. It is written to `Documents/My Games/<profile>/SGWGame/<account>/<shard>-<char>/ActionButtons - Saved Vars.lua` as `buttonInfo[n].actions[layer] = actionId`, and the actionId-to-ability binding is native. No server surface carries it: no def method or property, no DB column, nothing in legacy Python. Every "hotbar" mention in the server (`respawn/resync.rs`, `ability_granted.rs`) means `onKnownAbilitiesUpdate` (client method 101).

**Why:** AT-08 (2026-09-26) was told to "strip refunded ids from the saved hotbar". That turned out to be impossible server-side. The respec strips `sgw_player.abilities` instead, and stale buttons stay until the player clears them.

**How to apply:** any task that says "fix/clear/persist the hotbar" server-side is really about the known-abilities list. Changing the bar itself needs a client Lua patch, which is a maintainer decision. Client Lua lives in the client tree `..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI\`.
