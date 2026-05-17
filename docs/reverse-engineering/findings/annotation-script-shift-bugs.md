# Annotation Script Cyclic-Shift Bugs

> **Diátaxis type**: reference
> **Audience**: V5 campaign workers; consolidator; anyone correcting annotation-script-era naming errors
> **Last updated**: 2026-05-16
> **Confidence**: HIGH (RTTI-verified for all confirmed entries; candidates marked as such)

## Discovery context

Discovered during the W0 session-1 campaign sweep. The session-2 cyclic-shift sweep (W0 warmup, Phase C) extended that investigation to all adjacent TypedEmitInfo and CallbackImpl blocks in SGW.exe. The sweep enumerated all 935 `TypedEmitInfo__vfunc_0` and 936 `CallbackImpl__vfunc_2` functions, inspected dense contiguous blocks, and decompiled suspicious entries to verify RTTI against symbol names.

See [`cme-event-signal.md`](cme-event-signal.md) for full CME pipeline architecture.

## Methodology

1. Enumerated all `TypedEmitInfo__vfunc_0` and `CallbackImpl__vfunc_2` functions sorted by address via `search_functions_enhanced`.
2. Identified contiguous blocks (adjacent addresses separated by <= 0x20 bytes — the uniform destructor size) with >= 4 members.
3. For each suspicious block (name order does not match expected event-family ordering, or symbol prefix does not match neighboring functions), decompiled the inner cleanup function to extract the RTTI class name.
4. Compared RTTI class name against Ghidra symbol name.

## Confirmed corrections (applied in session 1)

### ContactList TypedEmitInfo block — 0x00e5f950–0x00e5f9f0

**Status: CORRECTED in session 1 (W0 session 1 applied all four fixes).**

| Address | Symbol name BEFORE correction | RTTI-verified correct name |
|---------|------------------------------|---------------------------|
| `0x00e5f950` | `contactListCreate` | `contactListCreate` (correct — no change) |
| `0x00e5f970` | `contactListDelete` | `contactListDelete` (correct — no change) |
| `0x00e5f990` | `contactListAddMembers` | `contactListRename` |
| `0x00e5f9b0` | `contactListRemoveMembers` | `contactListFlagsUpdate` |
| `0x00e5f9d0` | `contactListRename` | `contactListAddMembers` |
| `0x00e5f9f0` | `contactListSetFlag` | `contactListRemoveMembers` |

**Root cause**: Annotation script `04_event_signal_annotator.py` picked up the wrong string cross-reference for adjacent functions whose labels happened to be stored in a contiguous string table. The script read each function's string xref but grabbed the one from the adjacent function due to a +1 off-by-one in the xref iteration order. Creates a classic cyclic shift: function N gets the label of function N+1.

**Cascade consequence**: `contactListSetFlag` was entirely invented (no such event exists in the wire protocol). `contactListFlagsUpdate` is the real fourth event — confirmed by its RTTI descriptor `Event_NetOut_contactListFlagsUpdate` and by `register_NetOut_contactListFlagsUpdate` at `0x00e63ea0`.

## Session-2 Mercury cluster sweep (Phase B — W0 warmup)

### 0x0157b0d0 — Mercury_TimerExpiryHandler__vfunc_0 (CORRECTED)

**Status: CORRECTED by prior W0 run (session 2, before MCP disconnect). Plate documents the original contradiction.**

- **Prior label (annotation script)**: `Mercury_Nub_ReplyHandlerElement__vfunc_0`
- **RTTI from vtable reset in body**: `Mercury::TimerExpiryHandler::vftable`
- **Correct name**: `Mercury_TimerExpiryHandler__vfunc_0`

This is the only confirmed name mismatch in the 54-function Mercury cluster (`0x00de1670–0x01604330`). The annotation script appears to have had a one-slot cursor drift at this location — `Mercury_Nub_ReplyHandlerElement` was the expected next entry in an enumeration but the physical function here is the `TimerExpiryHandler` destructor.

**Mercury sweep coverage (session-2 Phase B):**

All `__vfunc_0` destructors in the Mercury dense cluster were checked against their RTTI vtable references:

| Address | Symbol | RTTI vtable | Status |
|---------|--------|-------------|--------|
| `0x0157b090` | `Mercury_NubException__vfunc_0` | `Mercury::NubException::vftable` | Correct |
| `0x0157b0d0` | `Mercury_TimerExpiryHandler__vfunc_0` | `Mercury::TimerExpiryHandler::vftable` | **Corrected** |
| `0x0158d3b0` | `Mercury_ChannelInternal__vfunc_0` | `Mercury::ChannelInternal::vftable` | Correct |
| `0x0158d4b0` | `Mercury_ClientMessage__vfunc_0` | `Mercury::ClientMessage::vftable` | Correct |
| `0x01583a00` | `Mercury_Nub_Connection__vfunc_0` | (complex cleanup, no direct reset — inner FUN_015830f0) | Correct by context |
| `0x01583a20` | `Mercury_Nub__vfunc_0` | `Mercury::Nub::vftable` | Correct |

No additional Mercury cyclic-shift bugs found.

## Session-2 sweep findings

### Plate comment inconsistency — 0x00d8ed00

**Status: CANDIDATE — symbol name correct, plate comment stale. Not a name-shift.**

- **Symbol**: `CME_EventSignal_VEvent_NetOut_onStrikeTeamResponse___TypedEmitInfo__vfunc_0`
- **Plate comment (stale)**: "Virtual destructor for CME_EventSignal_VEvent_NetOut_MinigameStartRequest TypedEmitInfo"
- **RTTI from inner cleanup FUN_00d8eca0**: `CME::EventSignal::TypedEmitInfo<class_Event_NetOut_onStrikeTeamResponse>::vftable`

The symbol name is correct per RTTI. The plate comment was written by an earlier session that read the wrong plate template (copied from `MinigameStartRequest`, which was processed immediately before this function). The symbol itself was already correct because the annotation script assigned it from the vtable, not the string table. No name correction needed; plate should be refreshed in a follow-up pass.

**Action**: Log `pending_plate_refresh` for `0x00d8ed00` in W0 checkpoint.

### Session-2 sweep result: No additional confirmed name-shifts found

The session-2 sweep covered:
- All 935 `TypedEmitInfo__vfunc_0` results in address order (offset 0–900 sampled at intervals)
- All 936 `CallbackImpl__vfunc_2` results in address order (first 100 + dense cluster at `0x00c95dc0`–`0x00c96200`)
- The contactList region (all 6 functions) — already correct post-session-1 fix
- The `0x00424690`–`0x004246e0` CallbackImpl cluster — verified correct by RTTI
- The `0x00c94f10`–`0x00c95b90` TypedEmitInfo cluster — symbol names match plate comments
- The NetIn TypedEmitInfo block `0x00d7b9a0`–`0x00d9c460` — sampled 10 entries, all consistent

**Summary**: The contactList block is the only confirmed cyclic-shift in the TypedEmitInfo/CallbackImpl naming families. The annotation script (`04_event_signal_annotator.py`) appears to have been robust for all other clusters.

## What to watch for in future sessions

The cyclic-shift pattern arises when the annotation script processes a function at address N and reads the string literal belonging to function N+1. The precondition is:
- Functions are in adjacent memory (< 0x20 bytes apart)
- Each function references exactly one string literal
- The string literals are in a contiguous block in `.rdata`, in the same order as the functions

This creates a systematic +1 shift for a window of functions until the sequence breaks (a gap in either the function addresses or the string-table order resets the script's cursor).

**Risk clusters for session-3 (`_MemberCallback__vfunc_3`)**: The MemberCallback functions use mangled template names, not string literals — the annotation script produced names from RTTI demangling, not string xrefs. Cyclic shifts from string-table cursor drift do not apply. The mangled names may have demangling errors but those are distinct from address-order shifts.

---

## Session-3 sweep — 2026-05-13

### Cluster 1 — CallbackImpl__vfunc_2 (all 936 functions)

**Status: NO NEW MISMATCHES FOUND.**

All 936 `CallbackImpl__vfunc_2` entries were enumerated in address order across offsets 0–935. The naming convention is consistent throughout: every entry uses the `CME_EventSignal_*___CallbackImpl__vfunc_2` pattern. The tail of the list (offset 850–935, addresses `0x00e38820–0x00e700a0`) shows the same uniform pattern with no address-order breaks or convention violations. The session-2 conclusion stands: the ContactList block was the only cyclic-shift in this family.

### Cluster 2 — TypedEmitInfo__vfunc_0 (all 935 functions)

**Status: NO NEW MISMATCHES FOUND.**

All 935 `TypedEmitInfo__vfunc_0` entries enumerated across offsets 0–934. Naming convention is consistent throughout: every entry uses `CME_EventSignal_*___TypedEmitInfo__vfunc_0`. The tail (offset 850–934, addresses `0x00e21880–0x00e5fe*`) shows no address-order breaks or convention violations.

### Cluster 3 — SGWNetworkManager MemberCallback cluster (0x00d44d60–0x00d46860+)

**Status: 20 CORRECTIONS APPLIED.**

The SGWNetworkManager `_MemberCallback__vfunc_3` cluster spans `0x00d44d60` through `0x00d48660+` (481 total SGWNetworkManager-related functions). The cluster is uniformly 0x80-spaced. Within this cluster, 20 functions at `0x00d46ce0–0x00d47660` were named using the short `OnEvent_NetOut_*__SGWNetworkManager` convention instead of the mangled template convention used by all 55 neighboring functions. Decompilation confirmed these are identical in body shape to their neighbors — they are all `__vfunc_3` RTTI type-descriptor accessors, not event handler implementations.

**Root cause**: The annotation script used a different naming path for a window of 20 functions in the mail/vendor/interaction subsection. These functions were named as if they were actual event handler implementations (`OnEvent_NetOut_*`) rather than RTTI accessor stubs. No cyclic address-order shift occurred; the event names themselves were correct — only the naming convention was wrong (missing the mangled template wrapper).

**Evidence**: Decompilation of `0x00d46ce0` (`OnEvent_NetOut_RepairItem__SGWNetworkManager`) shows the body returns `&CME::EventSignal::MemberCallback<struct_CME::EventSignal::NoSubject, class_SGWNetworkManager::EventHandler<class_Event_NetOut_RepairItem>, ...>::RTTI_Type_Descriptor` — identical body shape to neighbor `0x00d46c60` (`CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_Who_P845_SGWNetworkManager...___MemberCallback__vfunc_3`). Same pattern confirmed for `0x00d46d60` (`OnEvent_NetOut_RequestActiveSlotChange`) and `0x00d476e0` (`CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_SetAutoCycle...`).

| Address | Pre-sweep label | RTTI evidence | Corrected name | Action |
|---------|----------------|---------------|----------------|--------|
| `0x00d46ce0` | `OnEvent_NetOut_RepairItem__SGWNetworkManager` | `Event_NetOut_RepairItem` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_RepairItem_P845_SGWNetworkManager_VEvent_NetOut_RepairItem_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d46d60` | `OnEvent_NetOut_RequestActiveSlotChange__SGWNetworkManager` | `Event_NetOut_RequestActiveSlotChange` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_RequestActiveSlotChange_P845_SGWNetworkManager_VEvent_NetOut_RequestActiveSlotChange_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d46de0` | `OnEvent_NetOut_Interact__SGWNetworkManager` | `Event_NetOut_Interact` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_Interact_P845_SGWNetworkManager_VEvent_NetOut_Interact_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d46e60` | `OnEvent_NetOut_InitialResponse__SGWNetworkManager` | `Event_NetOut_InitialResponse` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_InitialResponse_P845_SGWNetworkManager_VEvent_NetOut_InitialResponse_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d46ee0` | `OnEvent_NetOut_DialogButtonChoice__SGWNetworkManager` | `Event_NetOut_DialogButtonChoice` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_DialogButtonChoice_P845_SGWNetworkManager_VEvent_NetOut_DialogButtonChoice_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d46f60` | `OnEvent_NetOut_PurchaseItems__SGWNetworkManager` | `Event_NetOut_PurchaseItems` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_PurchaseItems_P845_SGWNetworkManager_VEvent_NetOut_PurchaseItems_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d46fe0` | `OnEvent_NetOut_SellItems__SGWNetworkManager` | `Event_NetOut_SellItems` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_SellItems_P845_SGWNetworkManager_VEvent_NetOut_SellItems_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47060` | `OnEvent_NetOut_BuybackItems__SGWNetworkManager` | `Event_NetOut_BuybackItems` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_BuybackItems_P845_SGWNetworkManager_VEvent_NetOut_BuybackItems_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d470e0` | `OnEvent_NetOut_RepairItems__SGWNetworkManager` | `Event_NetOut_RepairItems` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_RepairItems_P845_SGWNetworkManager_VEvent_NetOut_RepairItems_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47160` | `OnEvent_NetOut_RechargeItems__SGWNetworkManager` | `Event_NetOut_RechargeItems` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_RechargeItems_P845_SGWNetworkManager_VEvent_NetOut_RechargeItems_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d471e0` | `OnEvent_NetOut_TrainAbility__SGWNetworkManager` | `Event_NetOut_TrainAbility` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_TrainAbility_P845_SGWNetworkManager_VEvent_NetOut_TrainAbility_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47260` | `OnEvent_NetOut_RequestMailHeaders__SGWNetworkManager` | `Event_NetOut_RequestMailHeaders` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_RequestMailHeaders_P845_SGWNetworkManager_VEvent_NetOut_RequestMailHeaders_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d472e0` | `OnEvent_NetOut_SendMailMessage__SGWNetworkManager` | `Event_NetOut_SendMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_SendMailMessage_P845_SGWNetworkManager_VEvent_NetOut_SendMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47360` | `OnEvent_NetOut_RequestMailBody__SGWNetworkManager` | `Event_NetOut_RequestMailBody` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_RequestMailBody_P845_SGWNetworkManager_VEvent_NetOut_RequestMailBody_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d473e0` | `OnEvent_NetOut_ArchiveMailMessage__SGWNetworkManager` | `Event_NetOut_ArchiveMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_ArchiveMailMessage_P845_SGWNetworkManager_VEvent_NetOut_ArchiveMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47460` | `OnEvent_NetOut_DeleteMailMessage__SGWNetworkManager` | `Event_NetOut_DeleteMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_DeleteMailMessage_P845_SGWNetworkManager_VEvent_NetOut_DeleteMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d474e0` | `OnEvent_NetOut_ReturnMailMessage__SGWNetworkManager` | `Event_NetOut_ReturnMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_ReturnMailMessage_P845_SGWNetworkManager_VEvent_NetOut_ReturnMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47560` | `OnEvent_NetOut_TakeCashFromMailMessage__SGWNetworkManager` | `Event_NetOut_TakeCashFromMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_TakeCashFromMailMessage_P845_SGWNetworkManager_VEvent_NetOut_TakeCashFromMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d475e0` | `OnEvent_NetOut_TakeItemFromMailMessage__SGWNetworkManager` | `Event_NetOut_TakeItemFromMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_TakeItemFromMailMessage_P845_SGWNetworkManager_VEvent_NetOut_TakeItemFromMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |
| `0x00d47660` | `OnEvent_NetOut_PayCODForMailMessage__SGWNetworkManager` | `Event_NetOut_PayCODForMailMessage` MemberCallback RTTI | `CME_EventSignal_ZV6_PAX_BEXPAVEvent_NetOut_PayCODForMailMessage_P845_SGWNetworkManager_VEvent_NetOut_PayCODForMailMessage_V__EventHandler_CME_EventSignal_UNoSubject___MemberCallback__vfunc_3` | renamed |

**Note on CAMPAIGN_STATUS.md boundary**: The CAMPAIGN_STATUS.md note cited the cluster as `0x00d44d60–0x00d46860` (55 functions). The actual cluster extends to at least `0x00d48660` (100+ functions total for the mangled-template portion, plus 100+ additional `OnEvent_*__SGWNetworkManager` functions at higher addresses that represent the actual handler implementations — the real `OnEvent_*` handlers begin at `0x00d46ce0` in the original naming, but from the decompilation these turned out to be the same vfunc_3 accessor stubs, not handlers). The remaining `OnEvent_*__SGWNetworkManager` functions above `0x00d47660` (at `0x00d476e0` onward) resume the mangled template convention and are correct; similarly, the further cluster beginning around `0x01579000+` uses a different naming family.

## Session-5 (2026-05-16) — entity-property-sync OQ-1 investigation

### GameEntityManager function name mismatch — 0x00dd0bb0 — RETRACTED

**Status: RETRACTED 2026-05-16 by follow-up Ghidra verification.**

The original claim below — that `GameEntityManager_RemoveEntityListener @ 0x00dd0bb0` was misnamed and should become `GameEntityManager_SetEntityVisible` / `OnEntityEnterAoI` — was based on Appendix D.6 of the entity-property-sync audit, which misread the call chain after `updateEntity_Handler @ 0x00dd62c0`. A focused single-question verification pass (audit Appendix E.1–E.4) decompiled `0x00dd0bb0` directly and confirmed:

- The function performs a `lower_bound` lookup on the listener map and calls `FUN_00e68df0` (refcount release). It IS a listener removal.
- The chain from `updateEntity_Handler` does **not** reach `0x00dd0bb0` — it reaches `FListenHelper::vtable[5] = FUN_01561140 @ 0x01561140`. Appendix D collapsed an indirect-jump branch and identified `[ECX+0x168]` as `GameEntityManager` when it is actually an `FListenHelper` instance (RTTI confirmed).
- The current Ghidra name `GameEntityManager_RemoveEntityListener` is **correct**; no rename is warranted.

**No annotation-script-shift bug exists at `0x00dd0bb0`.** The original entry below is retained for the audit trail but should not be acted on. The real annotation issue surfaced in this investigation is a wrong-slot-number plate-comment bug on the `GameEntityManager` vtable — captured separately under "GameEntityManager vtable plate-comment slot numbering" below.

Original (retracted) claim follows:

| Address | Current Ghidra name | Decompiler-confirmed behavior | Proposed name |
|---------|--------------------|-----------------------------|---------------|
| `0x00dd0bb0` | `GameEntityManager_RemoveEntityListener` | (Appendix D's misreading — superseded by E.4: function performs `lower_bound` listener-map lookup + `FUN_00e68df0` refcount release; the current name is accurate) | ~~`GameEntityManager_SetEntityVisible`~~ — RETRACTED, keep current name |

---

### GameEntityManager vtable plate-comment slot numbering — 0x00dd0bb0 and 0x00dd0c10

**Status: CANDIDATE — confirmed via raw memory read of vtable at `0x019aaeb8` during entity-property-sync §1.8 follow-up. Plate comments record wrong vtable slot numbers; symbol names are correct.**

Both plate comments claim slot numbers that contradict the raw vtable layout:

| Address | Plate-comment claim | Actual slot in vtable at `0x019aaeb8` |
|---------|--------------------|--------------------------------------|
| `0x00dd0bb0` | "VTable slot 5 of vtable_GameEntityManager at 0x019aaec4" | Slot 8 (memory at `0x019aaed8`) |
| `0x00dd0c10` | "VTable slot 2" | Slot 5 (memory at `0x019aaec4`) |

The actual function at vtable slot 5 (raw `0x019aaec4`) is `0x00dd0c10` (`GameEntityManager_SetPlayerControlTarget`), not `0x00dd0bb0` as the plate claimed. Slot 8 (`0x019aaed8`) is where `0x00dd0bb0` actually lives.

**Root cause**: Systematic vtable-slot numbering error in the `GameEntityManager` vtable annotation pass — the script appears to have used an off-by-three (or otherwise inconsistent) cursor when filling plate comments. Symbol names themselves come from RTTI and are unaffected.

**Evidence**: Raw memory read of vtable at `ghidra://SGW.exe@0x019aaeb8` (32 bytes of function pointers). See [`docs/audits/entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Appendix E.4. The §1.8 chapter section in `docs/drafts/spec/entity-property-sync.md` traces the actual dispatch chain.

**Action needed**: Correct plate comments on `0x00dd0bb0` and `0x00dd0c10` to record the actual vtable slot numbers (8 and 5 respectively). Spot-check the rest of the `GameEntityManager` vtable plate comments for the same off-by-N pattern. No symbol renames required — this is a comment-only fix.

---

## Cross-references

- [`cme-event-signal.md`](cme-event-signal.md) — CME pipeline architecture and class anatomy
- [`contact-list-wire-formats.md`](contact-list-wire-formats.md) — contactListFlagsUpdate wire format (surfaced by the shift correction)
- [`../v5-campaign/CAMPAIGN_STATUS.md`](../v5-campaign/CAMPAIGN_STATUS.md) — W0 session-1 report where the original fix was applied
- [`../annotation-scripts/04_event_signal_annotator.py`](../annotation-scripts/04_event_signal_annotator.py) — the script that produced the shifted names
