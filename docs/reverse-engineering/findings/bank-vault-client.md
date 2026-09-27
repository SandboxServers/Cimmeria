---
title: "Bank and Vault — Client Evidence (BV-E1)"
type: reference
audience: BV-02/BV-03/BV-05 implementers, the Bank and Vault campaign coordinator
last_updated: 2026-09-27
---

# Bank and Vault — Client Evidence (BV-E1)

> **Last updated**: 2026-09-27
> **Source**: SGW.exe read-only Ghidra pass (image base `0x00400000`, no renames or comments written this pass), the uncooked client UI module at `…\SGWGame\Content\UI\Core\{Vault,Dialog}\` (`Vault.lua` read in full, 986 lines), and the existing `dialog-controller-wire-flow.md` / `docs/content/dialog-ui-client-contract.md` findings, cross-checked against `docs/gameplay/inventory-system.md` and the Rust dialog-dispatch code.
> **Confidence**: HIGH for everything sourced from recovered Lua, seed schema and doc cross-reference; MEDIUM/LOW items are fenced off per question.
> **Companion docs**: [work-packets.md](../../analysis/bank-vault/work-packets.md) § BV-E1, [audit.md](../../analysis/bank-vault/audit.md) rows A-02/A-09/A-12, [dialog-controller-wire-flow.md](dialog-controller-wire-flow.md), [dialog-ui-client-contract.md](../../content/dialog-ui-client-contract.md), [inventory-system.md](../../gameplay/inventory-system.md)

## Summary

This is a read-only follow-up to the prior "Banker / Vault Open Path" pass (`docs/analysis/bank-vault` ledger cites it as `kg-2026-09-27/bank-re-findings.md`; that pass answered "does a Banker need special client code" — no). This pass answers five narrower questions the BV-E1 packet asks: bag-info timing for container 17, live resize behaviour, whether a Banker can offer a purchasable dialog button through the existing dialog system, the `onVaultOpen` `Position` argument, and an `isBankingOverride` sweep.

The headline finding is Q3: **yes**, a Banker can offer an "Expand vault" choice today, through the existing dialog seed tables and the existing `dialogButtonChoice`/content-engine wiring, with no new wire method and no client patch — provided the offered dialog carries exactly one clickable button (see below for why that qualifier matters).

## Q1 — Does the client need a fresh `onBagInfo` for container 17, or is the world-entry declaration enough? How does `getContainerSize` get its value?

**The world-entry declaration is enough. `onVaultOpen` triggers no bag-info request of its own.**

- `VaultMod.ValidateScrollbar` (`Vault.lua:298-304`) computes `maxSize = getContainerSize(Container.Vault) / 10`. `getContainerSize` is a native Lua binding (no Lua definition anywhere in the tree) that reads the client's cached per-container size state — the same cache every other container (`INV_Main`, etc.) is read from.
- `Vault.lua:985`, `VaultMod.UpdateAllItems( nil )`, runs unconditionally at module load (i.e., at UI construction / world entry), and it calls `ValidateScrollbar`. This proves the window is fully functional from whatever bag-size cache already exists at load time, before any Banker interaction happens.
- `VaultMod.onVaultVisibility` (`Vault.lua:9-16`), the handler for `onVaultOpen`'s native `Event_UI_VaultVisibility`, does exactly `VaultWin:moveToFront(); VaultWin:setVisible(true)` — no bag-size or bag-content request of any kind.
- Ghidra: `search_functions_enhanced` for `onBagInfo` and for `BagInfo` (both regex, case-sensitive over the full function name table) return **zero matches**. This is a meaningful negative: `onVaultOpen` and `onDialogDisplay` both have dedicated `Event_NetIn_*` classes with a named `register_NetIn_*` stub (e.g. `register_NetIn_onVaultOpen` @ `0x00d7e560`) plus a `TypedEmitInfo` vtable-install function (`CME_EventSignal_..._onVaultOpen___TypedEmitInfo__vfunc_0` @ `0x00d7e640`). `onBagInfo` has no such named event class anywhere in the binary's symbol table, consistent with it being an `ARRAY<BagInfo>` payload delivered through the universal RPC dispatcher (`0x00c6fc40`, per `findings/README.md`'s Architecture Note) straight into a generic per-container-size cache, rather than a bespoke CME UI event.

**Conclusion:** `onBagInfo` declaring container 17 at world entry (and on resync) is sufficient. `onVaultOpen` (client method 106) needs no accompanying `onBagInfo` send — Cimmeria's existing world-entry `onBagInfo` path (`crates/wire/src/mercury/world_data/map_loaded.rs`) already covers this, once BV-01 fixes `bag_max_slots`/`BAG_SIZES` to declare 17 (A-20/A-21 in the audit).

Confidence: HIGH (Lua evidence, doc cross-reference). MEDIUM on "no dedicated native `onBagInfo` handler exists at all" — absence of a *named* Ghidra symbol doesn't prove absence of an unnamed `FUN_` handler, only that it isn't implemented as a distinctly-named CME event the way `onVaultOpen`/`onDialogDisplay` are.

## Q2 — Does a mid-session `onBagInfo` with a larger size for 17 resize an open vault window, or only take effect on next open? Is there a Lua event on bag-info change?

**Yes — live resize, through a genuine native UI event, no reopen needed.**

- `Vault.lua:833`: `VaultWin:subscribe( Events.InventoryUpdateContainerSize, 'VaultMod.onResizeContainer' )`.
- `VaultMod.onResizeContainer` (`Vault.lua:419-423`):

  ```lua
  function VaultMod.onResizeContainer( this, containerId )
      if containerId == Container.Vault then
          VaultMod.ValidateScrollbar( this ) -- Takes care of size and makes sure current pos is valid.
      end
  end
  ```

  `ValidateScrollbar` re-reads `getContainerSize(Container.Vault)` and resets the scrollbar's `DocumentSize` (`Vault.lua:299-303`), so an already-open Vault window's scroll range grows live.
- `Events.InventoryUpdateContainerSize` is a genuine native CME UI event, not a Lua-only convention: Ghidra string search confirms the plain-text name at `0x019b4d78`, the RTTI type-descriptor name `.?AUEvent_UI_InventoryUpdateContainerSize@@` at `0x01e0b118`, and a `GameEventHandler<Event_UI_InventoryUpdateContainerSize, SGWScriptedWindow>` class at `0x01e1add8` — the same subscription mechanism `VaultVisibility` uses.
- Vault.lua subscribes to exactly four inventory-cache events and no others: `InventoryUpdateSlot`, `InventoryHideSlot`, `InventoryClear` (`Vault.lua:830-832`), and `InventoryUpdateContainerSize` (`:833`). There is no separate "bag info changed" event distinct from this one.
- **What raises it natively was not conclusively traced this pass.** The plain string at `0x019b4d78` is referenced only from a large (~69 KB decompiled) Lua-property-registration function at `0x00cc33f0` — the same shape as the `Dialog` constants table in `dialog-controller-wire-flow.md` ("not defined in any `.lua` file... registered natively"), i.e. a name-table registration, not the emit call site. The RTTI type-descriptor address had no direct code cross-references found.
- **Best-supported inference (MEDIUM confidence, architectural, not directly observed):** this event fires from the generic container-size cache-write path itself, the same layer that produces `InventoryUpdateSlot`/`InventoryHideSlot`/`InventoryClear` for content mutations — i.e., whenever the client's cached size for any container changes (from any `onBagInfo` that changes that container's declared size), the corresponding UI event fires and any open window for that container updates in the same tick. This is consistent with `onResizeContainer` being a generic, container-agnostic pattern (it checks `containerId == Container.Vault` itself, implying the same event fires for every container's resize, not just the Vault's).

**Conclusion for BV-05:** re-sending `onBagInfo` for container 17 with the new `bank_slots` value after a successful Expand purchase should resize an already-open Vault window's scrollbar immediately — no client patch, no reopen required. This is itself useful "visible feedback on the press" (see Q3).

## Q3 — Can a Banker offer an "Expand vault" dialog button through the existing dialog system? (Most important)

**Yes, using the existing dialog seed tables and the existing `dialogButtonChoice` path, with no new wire method — but only if the offered dialog carries exactly one clickable button.** Recommended flow: the Banker interaction opens the vault immediately *and* separately offers the expand dialog in the same response; it does not need to be a two-step "open vault / open dialog" choice.

### Wire shapes already in place

- **`onDialogDisplay`** — client method 105, `entities/defs/SGWPlayer.def:1150-1156`. Wire: `INT32 EntityId, INT32 DialogID, INT32 MissionFlags, UINT8 IsImmediate, INT32 aMissionId` (17 bytes). Cimmeria's single choke point is `send_dialog_display(player_id, npc_entity_id, dialog_id, tx, space_mgr)` in `crates/cell-content/src/cell/interactions/dialog.rs` — every dialog-display path (interact-open, monologue, chain actions, offer-mission) already routes through it, and it unconditionally records the id in the player's `offered_dialog_ids` set before sending, which is exactly the precondition `dialogButtonChoice` checks.
- **`dialogButtonChoice`** — cell method 75, `<Exposed/>`, `entities/defs/SGWPlayer.def:621-625`: `INT32 DialogId, INT32 ButtonId`. Server handling is `crates/cell-methods/src/cell/cell_methods/player/interaction/dialog.rs::handle_dialog_button_choice`: it verifies the dialog was actually offered to *this* player (`take_offered_dialog`, the #479/CAT-J-01 server-authority gate — a forged or replayed choice for an undelivered `dialog_id` is rejected and logged), consumes the offer (one-shot), then calls `crate::cell::content::fire_dialog_choice(entity_id, player_id, dialog_id, button_id, engine, tx, space_mgr)`.
- **Content-engine matching** (`crates/content-engine/src/triggers/matching.rs`): `Trigger::OnDialogChoice { dialog_id }` matches **only on `dialog_id`**, never `button_id`, even though `fire_dialog_choice` receives `button_id` as a parameter. This is the same limitation `docs/content/dialog-ui-client-contract.md` documents as blocked on decision DU-06 ("no authorable `button_id` condition yet").

### Why the "exactly one button" qualifier makes DU-06 irrelevant here

Because content-engine can only key off `dialog_id`, a dialog with two distinguishable choices ("Open vault" vs. "Expand vault") cannot be told apart by a chain today. But a dialog with **one** clickable button sidesteps this entirely: any reply that reaches the server for that `dialog_id` unambiguously *is* that one choice (closing the dialog with no buttons drawn sends nothing at all when a button exists — see the close-semantics rule below — so there is no other traffic on that `dialog_id` to confuse it with). No wire or content-engine change is needed for a single-button Expand offer.

### Seed shape (evidence, not a design decision)

- `db/resources/Dialogs/Tables/dialogs.sql`: `dialogs(dialog_id, dialog_flags, event_set_id, ui_screen_type, tags[], accepts_mission_id, name)`.
- `db/resources/Dialogs/Tables/dialog_screens.sql`: `dialog_screens(dialog_id, screen_id, text, speaker_id, index)`.
- `db/resources/Dialogs/Tables/dialog_screen_buttons.sql`: `dialog_screen_buttons(screen_button_id, button_id, screen_id, button_type, text)`.
- `button_id` (the value that goes on the wire as `dialogButtonChoice`'s `ButtonId`) is an independently-sequenced arbitrary integer (`dialog_screen_buttons_button_id_seq`), **not** the same thing as `button_type` (the client-drawn button kind — Accept/Decline/Generic1-3). Existing content happens to reuse `button_id = 8` for many Accept buttons (seen repeatedly in the colo telemetry `dialog-controller-wire-flow.md` cites), but that is convention, not a constraint — a new Expand button gets its own fresh `button_id`.
- These dialog id/screen id/button id sequences are separate from, and not covered by, the bank-reserved seed ranges in `work-packets.md` (`entity_templates` 370-389, `spawnlist` 470-489) — BV-05 needs its own allocation from the `dialogs_dialog_id_seq` family, which is a note for that packet, not decided here.

### Which `button_type` to use, and why

Per `dialog-ui-client-contract.md`: `button_type = 2` (Accept) is drawn as a fixed-art image button on both `DialogWin` and `BlurbWin` — **the authored `text` is never shown**, only the fixed "Accept" art. Only `button_type` 4/5/6 (Generic1-3) are `TextButton_2` widgets that render the authored `text`, and only `DialogWin` (i.e. `ui_screen_type = 2`, `DUIST_DefaultDialog`) draws Generic buttons at all (`BlurbWin` only draws More Info and Accept). So the Expand offer must be `ui_screen_type = 2` with a single `button_type = 4` ("Generic 1") button whose `text` carries the actual offer, e.g. `"Expand your vault by 10 slots for 100 naquadah?"` — this can be authored directly into `dialog_screens.text` too, so the price is visible to the player without any new wire field.

A single-screen, single-button dialog also automatically satisfies the client contract's Hard Rule 1 ("a dialog that keys a chain must have a button on its final screen") — there is only one screen, and it has the button.

### How the server learns which button was pressed

Exactly the existing path: click → `selectActiveDialogChoice(dialogId, buttonIndex)` (Lua) → native resolves the screen's cooked `Buttons[buttonIndex-1].ButtonID` → `dialogButtonChoice(dialogId, ButtonId)` sent to the server → `handle_dialog_button_choice` validates the offer and calls `fire_dialog_choice(..., dialog_id, button_id, ...)`. A content-engine chain with `Trigger::OnDialogChoice { dialog_id: <expand-dialog-id> }` fires on that one possible outcome. Given the "one statement or one transaction, atomic and replay-safe" contract in `work-packets.md`, and following the vendor precedent (`crates/base-methods/.../vendor/purchase/` is a dedicated bounded module, not a generic content-engine action), the recommendation is a dedicated handler for this specific `dialog_id`/`button_id` pair rather than a generic chain action — this is an implementation suggestion for BV-05, not something this RE pass can settle.

### Can a Banker open the vault *and* offer the dialog in the same interaction?

**Yes.** `VaultWin` and `DialogWin`/`BlurbWin` are entirely separate CEGUI windows driven by entirely separate CME events (`Event_UI_VaultVisibility` for the vault; `Event_UI_DialogDisplay`/`Event_UI_DialogAvailable` for dialogs — `dialog-controller-wire-flow.md`'s "two active slots" constraint is a property of the `DialogController`'s own two-slot array, and has nothing to do with the Vault window). Sending `onVaultOpen(banker_id, banker_pos)` (client method 106) and `onDialogDisplay(banker_id, expand_dialog_id, 0, 1, 0)` (client method 105) in the same interaction response opens both windows together with no eviction conflict, because only one dialog is being offered (the two-dialog eviction problem only arises when the server tries to display *two* dialogs, which this flow doesn't do).

**Recommendation:** on a Banker `interact()`, always send `onVaultOpen` immediately (this is itself the primary "you clicked the right thing" feedback — the window appears). Additionally send the single-button Expand dialog **only when `bank_slots < 100`** (nothing to gate on cash at send time — validate cash server-side when the button reply arrives, and reply with existing chat/feedback machinery, `crates/cell-console/src/cell/console/chat.rs`'s `onPlayerCommunication` route, for both the success and insufficient-funds cases). This avoids a second interaction round-trip ("open dialog first, whose buttons are Open vault / Expand vault") and gives visible feedback three ways on a successful Expand click: the naquadah counter (`onCashChanged`, already a documented client method) ticks down, the open vault's scrollbar visibly grows (Q2), and a chat-line confirmation fires regardless of whether the vault window happens to be open.

## Q4 — What does `onVaultOpen`'s `Position` argument do? Does the client auto-close on walk-away?

**No client-side consumption of any kind was found.** Auto-close-on-distance is not implemented client-side; this must be entirely server-authoritative.

- Wire: `onVaultOpen`/`onTeamVaultOpen`/`onCommandVaultOpen` (client methods 106-108) each carry `INT32 EntityId, VECTOR3 Position` (`docs/protocol/client-method-dispatch-table.md:253-255`, re-confirmed this pass). Notably, the structurally similar `onStoreOpen` (109) and `onTrainerOpen` (113) carry **no** `Position` argument at all (`:256,260`) — `Position` is unique to the three Banker-family methods among the known "open an NPC window" methods, which argues against pure copy-paste-from-vendor boilerplate and makes a deliberate (if unfinished) vault-specific purpose somewhat more plausible.
- Lua: `Vault.lua` (986 lines, read in full) has zero references to `Position` outside of CEGUI widget x/y placement (`getXPosition`, `setPosition`, scrollbar `ScrollPosition`, etc.). No distance check, no auto-close-on-move logic anywhere in the vault window's script.
- Ghidra: `register_NetIn_onVaultOpen` (`0x00d7e560`) decompiles to a trivial stub returning the literal string `"Event_NetIn_onVaultOpen"` — a name/RTTI label, not an argument decoder. `CME_EventSignal_..._onVaultOpen___TypedEmitInfo__vfunc_0` (`0x00d7e640`) only installs a vtable via `FUN_00d7e5e0` and (conditionally) frees the object; no field-specific logic is visible at this layer. Both addresses are referenced only from data (a small RTTI/vtable block at `0x019c8980-0x019c89c8`, alongside the `"Event_NetIn_onVaultOpen"` name string); no further code cross-references to that block were found. This is the same generic CME argument-marshalling shape `dialog-controller-wire-flow.md` documents for `onDialogDisplay`, where the true per-field consumer sits below the emit boilerplate. The call site that would show whether `Position` is stored anywhere retrievable was **not** reached this pass — same open status as the prior pass's Unknown #4.

**Conclusion:** `Position` is part of the wire struct and reaches the client's event object, but nothing observed — in Lua or in the traced native path — reads it. Two explanations are consistent with the evidence and neither is provable here: (a) a proximity-close feature that was designed (hence appearing only on the three Banker methods) but never wired up before ship, matching SGW's pattern of other half-finished 2009 systems; or (b) inert/reserved data with no consumer. **Implementation guidance:** do not rely on the client to close the vault window when the player walks away — it will not. The server-authoritative `vault_move_allowed` / session-clearing design already scoped for BV-02/BV-03 in `work-packets.md` is therefore not just a good idea but the *only* enforcement point. Sending the player's actual position as `Position` costs nothing and matches the original wire struct, but no server behavior should depend on the client doing anything with it.

## Q5 — `isBankingOverride` property-index sweep (low priority)

**Unchanged negative result; a genuine property-index sweep was not performed (explicitly low priority in the packet), and the underlying architecture makes a positive result unlikely.**

- `entities/defs/SGWPlayer.def:91-95`: `isBankingOverride` is `INT8 CELL_PUBLIC`, default `0`, immediately following `pvpFlag` (also `INT8 CELL_PUBLIC`, `:85-89`). No doc comment either side.
- Ghidra `search_strings` for `"isBankingOverride"` and `"BankingOverride"` (this pass): **zero matches**, reproducing the prior pass's negative result exactly.
- Why a name-based search cannot resolve this either way: per this agent's own prior finding (`entity-property-sync-oq1.md`, closed 2026-05-16), `CELL_PUBLIC`/`CELL_PRIVATE` scalar properties are **not** read through a per-property named accessor client-side. They arrive through the generic `updateEntity` (Mercury msg_id `0x0A`) typed-envelope property-delta path, decoded generically as a `uint32_t` propID at `FNetworkPropertyChange__vfunc_0` (`0x015652d0`) with no per-property function name anywhere in the binary. A property with no bespoke consumer therefore leaves no distinguishing string or function name to find — this is the expected shape of "declared but unused," not evidence the search missed something.
- What would actually close this: decode `SGWPlayer`'s parsed `EntityDescription` property array to get `isBankingOverride`'s numeric index (its position among declared properties), then check whether any generic property-dispatch code branches on that specific index. Given the OQ-1 finding that this whole layer is a flat generic byte-copy into a property-value slot with no per-property switch statement at all, a "no consumer" result is the most likely outcome of that sweep too — but it was not run this pass.

## Recommended flow for BV-02 / BV-05 (summary)

1. Banker `interact()` → send `onVaultOpen(banker_id, banker_pos)` immediately (client method 106). This is the primary feedback; the window opening confirms the click landed.
2. In the same response, if `bank_slots < 100`, also send `onDialogDisplay(banker_id, expand_dialog_id, 0, 1, 0)` (client method 105) for a **single-screen, single-`button_type=4`-button** dialog whose screen text and button text both state the price (e.g. "+10 slots for 100 naquadah").
3. On `dialogButtonChoice(expand_dialog_id, expand_button_id)`, validate cash server-side (never trust a client amount — none is sent), debit cash and raise `bank_slots` by 10 in one statement/transaction, then send `onBagInfo` re-declaring container 17 (resizes any open vault window live per Q2) and `onCashChanged`, plus a chat/feedback line confirming the result either way.
4. Do not depend on `onVaultOpen`'s `Position` for anything (Q4) — closing/invalidating the session on distance is entirely server-side, exactly as already scoped in BV-02/BV-03.

## Evidence and address inventory

| Item | Address / location | Role |
|---|---|---|
| `register_NetIn_onVaultOpen` | `0x00d7e560` | Name/RTTI stub, not the arg decoder |
| `CME_EventSignal_..._onVaultOpen___TypedEmitInfo__vfunc_0` | `0x00d7e640` | Installs vtable via `FUN_00d7e5e0`; no field-specific logic |
| RTTI/vtable block for `Event_NetIn_onVaultOpen` | `0x019c8980-0x019c89c8` | Data block referencing both of the above; no further code xrefs found |
| `Events.InventoryUpdateContainerSize` plain string | `0x019b4d78` | Confirms the event name is native, not Lua-authored |
| `Event_UI_InventoryUpdateContainerSize` RTTI type descriptor | `0x01e0b118` | Confirms it is a real CME UI event class |
| `GameEventHandler<Event_UI_InventoryUpdateContainerSize, SGWScriptedWindow>` | `0x01e1add8` | The subscription mechanism `VaultWin:subscribe(Events.InventoryUpdateContainerSize, ...)` binds to |
| Lua property/event name registration blob | `0x00cc33f0` (~69 KB decompiled) | References the `InventoryUpdateContainerSize` string; a name-table registration, not the emit site (emit site not found this pass) |
| `onDialogDisplay` handler chain | `0x00d25900` → `0x00d25200` → `0x00d24f10` | Full trace in `dialog-controller-wire-flow.md`; unchanged this pass |
| `send_dialog_display` | `crates/cell-content/src/cell/interactions/dialog.rs` | Rust choke point a Banker module would call to offer the Expand dialog |
| `handle_dialog_button_choice` | `crates/cell-methods/src/cell/cell_methods/player/interaction/dialog.rs` | Server-side receipt, offer-gate (#479), and `fire_dialog_choice` dispatch |
| `Trigger::OnDialogChoice` matching | `crates/content-engine/src/triggers/matching.rs` | Confirms dialog-id-only matching (no `button_id` condition yet — DU-06) |
| `isBankingOverride` declaration | `entities/defs/SGWPlayer.def:91-95` | `INT8 CELL_PUBLIC`, no consumer found |
| `FNetworkPropertyChange__vfunc_0` | `0x015652d0` | Generic property-delta decode site cited for why Q5 is architecturally hard to find by name |

## Open questions

1. The exact native call/condition that raises `Event_UI_InventoryUpdateContainerSize` (Q2) — the emit site was not located; only its name-table registration was. Would need a fresh Ghidra pass on the generic bag-size cache-write function(s), or an x64dbg non-freezing breakpoint on the event's `TypedEmitInfo` constructor while sending a live resized `onBagInfo`.
2. Whether `onVaultOpen`'s `Position` field is stored anywhere retrievable client-side, or discarded outright (Q4) — the true argument-decode call site below the emit boilerplate at `0x00d7e5e0` was not reached. An x64dbg trace of a live `onVaultOpen` receipt would close this.
3. `isBankingOverride`'s numeric property index and whether any generic-dispatch code branches on it (Q5) — explicitly deferred as low priority per the packet.
4. Whether the client auto-closes the Dialog/Blurb window itself after a Generic-button click, or requires a subsequent Done click (relevant to how many player actions "Expand" costs) — `dialog-controller-wire-flow.md` documents the *send* path (`selectActiveDialogChoice`) but not whether the click also advances/closes the screen locally; not re-traced this pass.
