---
name: client-lua-cegui-global-subscribe
description: SGW client Lua can subscribe CEGUI global events via window:subscribeGlobal(ns, event, 'Handler.name'); no unsubscribeGlobal; handler args (this, eventWindow, ...); used by the lab click ring (LR-10)
metadata:
  type: reference
---

Found statically for LR-10 (2026-10-11); not yet seen firing in a live client.

- `window:subscribeGlobal(namespace, event, 'Mod.fn')`: three Lua strings (tolua stub, error text `error in function 'subscribeGlobal'`, `docs/reverse-engineering/decompiled/14_standalone_named.c:~202000`). Stock live use: `Inst1ChatWin:subscribeGlobal(CEGUI.RichTextItemEntry.EventNamespace, ...EventLinkClicked, 'ChatMod.onLinkClicked')`.
- Handler gets `(this, eventWindow, ...unpacked event args)`; mouse args are `position, moveDelta, button, sysKeys, wheelChange, clickCount`.
- `CEGUI.Window.EventNamespace` / `EventMouseClick` are bound; literals `"Window"` / `"MouseClick"` work as fallback.
- **No `unsubscribeGlobal` binding**: subscribe once per Lua state (a flag in `_G.CimmeriaLab`), never per pump, or handlers stack.
- `CEGUI.GlobalEventSet` / `subscribeEvent` appear only in the CEGUI sample `demo8.lua`, not the SGW binding.
- Return nothing from the handler so the click is never marked handled.

Detail: `docs/analysis/lab-record/worknotes/LR-10.md`. Code: `crates/lab/src/supervisor/events/lua_rings.rs`.
