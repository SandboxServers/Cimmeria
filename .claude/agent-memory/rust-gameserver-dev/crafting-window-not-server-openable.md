---
name: crafting-window-not-server-openable
description: The stock SGW client cannot open the crafting window from any server message or machine click; only the J action and the access bar show it
metadata:
  type: project
---

The crafting window (`CraftingWin`) opens only through `Actions.ToggleCrafting` (the J binding in `Bindings.toc`) and `AccessMod.onCraftingClicked`. The UI event registry in `SGW.exe` (the UTF-16 `Events.*` name table around file offset `0x15b4118`) has `VendorOpen`, `TrainerOpen`, `BMOpen`, `DHDVisibility` and `VaultVisibility`, but no crafting-open event. `ToggleCrafting` is not in the exe at all. Clicking an `INT_Machine_*` entity (bits 56-60) gives a cursor, a marker body and a minimap icon, but the click is otherwise dead on both ends.

**Why:** in CR-19 (2026-09-27) the owner asked for "right-click a station opens its window". That needs a client patch (cimmeria-client-patches). Seeding the bits and adding a server arm cannot do it.

**How to apply:** before promising any "server opens window X" feature, check the event-name table for an `XOpen`/`XVisibility` event. Evidence and patch options are in `docs/analysis/crafting/worknotes/cr-19.md`. To check a window, scan the exe's UTF-16 strings for its event name (Python over the file bytes; `strings` is not installed in Git Bash). Related: [[client-action-bar-is-client-side]].
