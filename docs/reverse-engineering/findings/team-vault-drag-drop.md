---
title: "Team Vault Drag-and-Drop Refusal — Server, Lua and Vtable Ruled Out"
type: reference
audience: bank-and-vault campaign, anyone touching Team/Command org state on the client
last_updated: 2026-09-27
---

# Team Vault Drag-and-Drop Refusal — Server, Lua and Vtable Ruled Out

> **Last updated**: 2026-09-27
> **Confidence**: HIGH for every ruled-out cause (decompiled code, full-file Lua/layout diffs, or a
> 48-slot vtable dump); LOW for the remaining hypothesis (native, not directly observed).
> **Companion docs**: [bank-vault-client.md](bank-vault-client.md) (BV-E1),
> [organization-restoration.md](organization-restoration.md) (Squad/Team/Command sub-object layout),
> [inventory-state-machine.md](inventory-state-machine.md) (`onClearOrgVaultInventory`, previously
> "not established from binary alone" — closed below), `docs/analysis/bank-vault/audit.md` (A-04, A-07,
> A-13), worknote
> [`docs/analysis/bank-vault/worknotes/fix-team-vault-drop.md`](../../analysis/bank-vault/worknotes/fix-team-vault-drop.md)

## Summary

A follow-up to the bank-and-vault campaign (BV-07): an owner playtest (2026-09-28) found that dragging an
item from the backpack onto an open **Team vault** (container 19) window produces no slot highlight and no
`moveItem` at all — SigNoz shows zero `org_move_accepted`/`org_move_rejected` events, so the client never
sends the RPC. The same drag into the **Command vault** (container 20) and the **personal vault**
(container 17) both work. Disbanding and re-founding the Team did not fix it.

This pass rules out, with direct evidence, every hypothesis that would have a server-side or Lua-side fix:
the server's org-vault code is provably symmetric between Team and Command; the client's `Team.lua` /
`Command.lua` and their `.layout` files are structurally identical apart from names; and — the new result —
the client's `Team` and `Command` C++ classes have **fully symmetric vtables**, slot for slot, through the
whole observed range. No missing override, no stub, no null slot exists on one side that the other has.
The remaining candidate is native and unproven: a per-player cached "my Team org id" / "my Command org id"
pair (confirmed to exist, at `localPlayer+0x70`/`+0x74`) that at least one native handler reads asymmetrically
by fixed offset rather than by a shared virtual call — whether the *drag-drop acceptance* path also consults
this cache, and whether the Team slot is populated correctly, needs a live trace (not run this pass; see
Open Questions).

## What was ruled out

### 1. Server: `bank-vault` org-vault code is symmetric (HIGH)

- `crates/base-methods/src/base/world_entry/methods/inventory/org_vault/open.rs::handle_org_vault_open` →
  `open()` is the single code path for both `VaultScope::Team` and `VaultScope::Command`. It always sends
  `onBagInfo` (via `org_vault_bag_info`) before the vault's rows, before granting the window — this holds
  for both scopes identically.
- `org_vault_bag_info(bank_slots, team_slots)` (`open.rs`) builds `Inventory::new(0)` — which already seeds
  **every** container 1-20 at its ceiling from `BAG_SIZES` (`crates/entity/src/inventory.rs`) — then
  overrides 17 (`with_bank_slots`) and 19 (`with_org_vault_slots(INV_TEAM_BANK, team_slots)`). Container 20
  is never touched by this call because it is fixed at its ceiling (100) already; that is not an asymmetry,
  it is why `INV_COMMAND_BANK` needs no override.
- `access.rs::lock_actor` computes `vault_slots` as `actor.vault_slots` (a real `SELECT vault_slots FROM
  sgw_organizations` read) for `VaultScope::Team`, and `COMMAND_VAULT_SLOTS` (a compile-time constant, 100)
  for `VaultScope::Command` — the only difference between the two arms is that Team's size is variable
  (40-100) and Command's is fixed, which is D-BV14, not a bug.
- `crates/wire/src/cell/vault.rs::vault_open_method`/`build_vault_open_args` are scope-generic; the byte
  layout (`INT32 EntityId, VECTOR3 Position`) and the method indices (106/107/108) are confirmed against
  `docs/protocol/client-method-dispatch-table.md:253-255` and pinned by
  `vault.rs`'s own `each_scope_opens_its_own_window`/`vault_open_args_are_int32_then_vector3` tests.
- `EOrganizationType` (`entities/defs/enumerations.xml:1939-1946`: `Squad=0, Team=1, Command=2`) matches
  `cimmeria_entity::organization::OrgType` (`crates/entity/src/organization/types.rs:25-32`) byte for byte.
- `crates/base-session/src/base/organization/handlers/push.rs::restore_on_login`/`org_state_messages` push
  `onOrganizationJoined` [35] (plus name/MOTD/cash/XP/ranks/roster) for **every** membership a player has —
  Team and Command alike — at every world entry and gate travel, and again on founding (ORG-05,
  `new_member=true`). `load_memberships` (`crates/base-session/src/base/organization/persistence/loads.rs:53`)
  returns a `Vec<OrgMembership>` with no `LIMIT`, ordered by `org_type`, so a player in both a Team and a
  Command (the reporting owner's exact situation) gets both pushes, not just one.
- BV-07's own regression suite (`org_vault::tests::*`, `bank::org_open_tests::*`) already exercises the
  open, the resend and the move path for both scopes; nothing in this pass found a code path that treats 19
  and 20 differently apart from the documented, intentional D-BV14 size difference.

**Conclusion**: there is no server-side asymmetry to fix. A server round trip is not even reached — SigNoz
shows zero events for this bug, which independently confirms the client never sends `moveItem`.

### 2. Client Lua and layout: byte-identical apart from names (HIGH)

- `Team.lua` vs `Command.lua` (full-file `diff`): the drag/drop chain —
  `onSlotItemDragStarted`/`onSlotItemDragEnters`/`onSlotItemDragReceived`/`onSlotHighlightOn`/
  `getSlotId`/`getSlotIDForVisibleSlot`/`ValidateScrollbar` — is structurally identical between the two
  files, differing only in the `TeamMod`/`CommandMod` prefix and the mixed `Container.TeamBank`/
  `Container.TeamVault` (both 19) vs `Container.CommandBank`/`Container.CommandVault` (both 20) naming
  (confirmed against `entities/defs/enumerations.xml:925-951`, no shift: `INV_TeamBank=19`,
  `INV_CommandBank=20`, exactly matching the Rust constants).
- Critically, `TeamMod.onSlotItemDragEnters` (`Team.lua:558-564`) calls `TeamMod.onSlotHighlightOn`
  **unconditionally** for any item drag — there is no `getContainerSize` check, no permission check, and no
  size gate anywhere in the scripted drag path. If CEGUI fires `EventDragDropItemEnters` to a
  `TeamVault_SlotN` window at all, the highlight shows regardless of the container's declared size. This
  rules out "the Lua reads a bad cached size and silently no-ops" — the Lua doesn't read the size at all
  for this purpose.
- `TeamVault.layout` vs `CommandVault.layout` (full-file `diff`): identical apart from window names and two
  tooltip strings (confirmed independently of the earlier BV-07 audit note). Every `DragContainer` slot
  window (`TeamVault_Slot1..40` / `CommandVault_Slot1..40`) has the identical property set; there is no
  `DragDropTarget`-style property on either side to compare (CEGUI's drag-drop-target flag isn't declared in
  layout XML on this window type either way).

**Conclusion**: whatever refuses the drag happens *before* Lua's `EventDragDropItemEnters` handler runs, or
CEGUI never fires the event to these windows for a reason unrelated to anything the Lua or layout controls.

### 3. Client C++ class hierarchy: `Team` and `Command` vtables are fully symmetric (HIGH, new this pass)

Both `Team` and `Command` derive from `Organization` (per `organization-restoration.md`'s Q2 finding: three
sub-objects inline on the local player at `+0x6c` Squad / `+0x70` Team / `+0x74` Command). Their
constructors were decompiled and disassembled:

```text
Team__vfunc_0    @ 0x00eb4140: *this = Team::vftable (0x019eb1dc); CALL Organization_ctor_body(0x00e4c570)
Command__vfunc_0 @ 0x00eb3000: *this = Command::vftable (0x019eb07c); CALL Organization_ctor_body(0x00e4c570)
```

Both install their own vtable, then run the identical shared base constructor (which — per
`organization-restoration.md` — registers 16 CME event subscribers). Dumping both vtables slot by slot,
48 slots deep (`0x019eb1dc`/`0x019eb07c` through `+0xBC`):

| Slots | Team | Command | Relationship |
|---|---|---|---|
| 0 | `Team__vfunc_0` (ctor) | `Command__vfunc_0` (ctor) | Expected per-class |
| 1-11 | `0xe57230`, `0x1604f20` (`CookedTextType::vfunc_2`), `0xe51520`, `0xe51830`, `0xe51b60`, `0xe4ca90`, `0xe4cfa0`, `0xe52070`, `0xe52280`, `0xe524f0`, `0xe52780` | identical addresses, all 11 | Shared/inherited (byte-identical) |
| 12-16 | `0xe52a20`, `0xe52cd0`, `0xe52f80`, `0xe53220`, `0xe4d590` | identical addresses, all 5 | Shared/inherited |
| 17, 19, 21 | `0xeb3680`, `0xeb3730`, `0xeb37f0` | `0xeb24c0`, `0xeb25f0`, `0xeb26b0` | **Both** override, own distinct code — neither is missing |
| 18, 20, 22, 23 | `0xe4c430`, `0xe4c4c0`, `0xe4c550`, `0xe4b5b0` | identical addresses, all 4 | Shared/inherited |
| 24-28 | `0xeb36a0`, `0xeb36d0`, `0xeb3700`, `0xeb3720`, `0xeb3890` | `0xeb24e0`, `0xeb2550`, `0xeb25c0`, `0xeb25e0`, `0xeb2750` | **Both** override, own distinct code |
| 29-35 | `0xe4dd80`, `0xe4e190`, `0xe4e4c0` (`onMemberJoinedOrganization`, matches vfunc[0x1F] per `organization-restoration.md`), `0xe4ea50` (`onOrganizationRosterInfo`, matches Q1), `0xe4f400` (`onMemberLeftOrganization`, matches vfunc[0x21]), `0xe4fb70`, `0xe50120` | identical addresses, all 7 | Shared/inherited — confirms the roster handlers this doc already knew about live at the same slots for both classes |
| 36-42 | `0xe50290`..`0xe51100` | identical addresses, all 7 | Shared/inherited |
| 43 | `0xeb3d40` | `0xeb2c10` | **Both** override, own distinct code |
| 44, 46 | `0x1bdc500`, `0x1bdc54c` (data, not code — per-class state, not a vtable slot mistake: `getFunctionAt` returns none because these are data addresses, consistent with per-instance member storage rather than a function pointer) | `0x1bdc384`, `0x1bdc3d0` (different data addresses, as expected for separate instances) | Both present, both data |
| 45 | `CME_EventSignal_..._UpdateTeamMember___TypedEmitInfo__vfunc_0` @ `0xeb47f0` | `CME_EventSignal_..._UpdateCommandMember___TypedEmitInfo__vfunc_0` @ `0xeb3580` | Parallel named event classes, both present |
| 47 | `CME_EventSignal_..._TeamUpdateCash___TypedEmitInfo__vfunc_0` @ `0xeb4100` | `CME_EventSignal_..._CommandUpdateCash___TypedEmitInfo__vfunc_0` @ `0xeb2fc0` | Parallel named event classes, both present |

Every slot that differs between the two classes differs because **both** classes have their own, present,
distinct implementation (an intentional override), never because one side is missing, null, or stubbed.
There is no vtable-level evidence of an unfinished Team-only or Command-only code path anywhere in this
48-slot range.

**Conclusion**: the bug is not "the shipped client never finished the Team override of some method the
Command override has." Both class hierarchies are complete and parallel.

### 4. Re-founding rules out corrupted org-specific data (HIGH, reported by the coordinator)

Disbanding and re-founding the Team creates a brand-new `sgw_organizations` row with a fresh default
`vault_slots = 40` (`db/sgw/Organizations/Tables/sgw_organizations.sql`), and the bug still reproduces. This
independently rules out "this one Team's `vault_slots` column is corrupted or NULL" — a fresh org can't
carry over a bad value from the old one.

## What was not ruled out (the remaining lead)

`FUN_00e1dcc0` — the native handler for `onClearOrgVaultInventory` (client method 74, cited but not
resolved in `inventory-state-machine.md:264`) — decompiles to:

```c
// Reads the incoming "OrganizationId" field off the wire (Mercury__unknown_00e3cba0), then:
iVar4 = FUN_00c66ad0();   // GameEntityManager::instance()
if (iStack_40 == *(int *)(*(int *)(*(int *)(iVar4 + 0x8c) + 0x70) + 4)) {
    local_44 = 0x13;      // 19 = INV_TeamBank
} else {
    iVar4 = FUN_00c66ad0();
    if (iStack_40 == *(int *)(*(int *)(*(int *)(iVar4 + 0x8c) + 0x74) + 4)) {
        local_44 = 0x14;  // 20 = INV_CommandBank
    }
}
// ...then walks an item collection on `this`, removing every item whose stored
// container id (offset 0x18 on each item record) equals local_44.
```

`GameEntityManager::instance()+0x8c` resolves to `localPlayer` (matching `organization-restoration.md`'s
"`GameEntityManager::instance()->localPlayer + 0x6c/0x70/0x74`" citation exactly); `+0x70`/`+0x74` are the
Team's and Command's inline `Organization`-subclass sub-objects; `+4` within each is that sub-object's own
`org_id` field. So `onClearOrgVaultInventory` maps a server-sent `OrganizationId` to a container id (19 or
20) by comparing it against the client's **own cached copies** of "my Team's org id" and "my Command's org
id" — two independent fields, read by fixed offset rather than by a shared virtual call.

This is the one place in the whole investigation where Team and Command are handled by **parallel data
reads at different offsets** rather than by a shared, symmetric code path — everything else (the vtables,
the Lua, the server) goes through one shared mechanism for both. If the native code that *writes* the Team
slot (`+0x70`, presumably from receipt of `onOrganizationJoined` [35] with `aOrganizationType=1`) has a bug
that the Command-writing path (`+0x74`, `aOrganizationType=2`) does not — or if whatever drag-drop
acceptance check exists (not yet located; the generic `moveItem`/`getContainerSize` Lua-native bindings are
anonymous functions in this binary, confirmed by an `FNSUB` sweep, consistent with `bank-vault-client.md`'s
Q2 finding that the `InventoryUpdateContainerSize` emit site was never traced either) consults this same
per-org-type cache — that would explain a Team-only, data-independent failure. **This is not confirmed.**
It is offered as the most concrete remaining lead, not a diagnosis.

## Evidence and address inventory

| Item | Address / location | Role |
|---|---|---|
| `Team__vfunc_0` (ctor) | `0x00eb4140` | Installs `Team::vftable`, calls `Organization_ctor_body` |
| `Command__vfunc_0` (ctor) | `0x00eb3000` | Installs `Command::vftable`, calls `Organization_ctor_body` |
| `Organization_ctor_body` | `0x00e4c570` | Shared base ctor, registers 16 CME event subscribers (per `organization-restoration.md`) |
| `Team::vftable` | `0x019eb1dc` | 48 slots dumped this pass, fully compared against Command's |
| `Command::vftable` | `0x019eb07c` | Same |
| `onMemberJoinedOrganization` handler | `0x00e4e4c0` (slot 31 both classes) | Shared, matches `organization-restoration.md` Q1's `vfunc[0x1F]` |
| `onOrganizationRosterInfo` handler | `0x00e4ea50` (slot 32 both classes) | Shared, matches Q1 |
| `onMemberLeftOrganization` handler | `0x00e4f400` (slot 33 both classes) | Shared, matches Q1's `vfunc[0x21]` |
| `onClearOrgVaultInventory` native handler | `0x00e1dcc0` (`FUN_00e1dcc0`) | Resolves `OrganizationId` → container 19/20 via `localPlayer+0x70`/`+0x74` org-id cache; closes the "not established from binary alone" note in `inventory-state-machine.md:264` |
| `GameEntityManager::instance()` | `0x00c66ad0` (`FUN_00c66ad0`) | Global singleton accessor; `+0x8c` = `localPlayer` |
| `register_NetIn_onTeamVaultOpen` | `0x00d7e800` | RTTI/name stub only, matches audit A-11's address |
| `register_NetIn_onCommandVaultOpen` | `0x00d7eaa0` | RTTI/name stub only, matches audit A-11's address |
| `register_NetIn_onOrganizationJoined` | `0x00d8a520` | RTTI/name stub; true handler is a `MemberCallback<NoSubject, Organization, void(Event_NetIn_onOrganizationJoined const*)>` (RTTI type name at `0x01e64240`), not reached this pass (heap-constructed, no static xref found) |

## Open questions

1. **Does the client ever call a native drag-drop-acceptance predicate that consults the Team/Command
   org-id cache (or a container-size cache) before invoking `EventDragDropItemEnters`'s scripted handler?**
   Not located. The generic `moveItem`/`getContainerSize` Lua-native bindings have no discoverable symbol
   name (confirmed by `FNSUB` sweep for `execmoveitem`, `getcontainersize`, `inventoryslot`, `slotwindow`,
   `candropitem`, `isvalidcontainer` — zero hits for all). Closing this needs either a full trace of the
   ~69 KB generic Lua-property-registration function `bank-vault-client.md` already flagged as untraced, or
   a live x64dbg session.
2. **Does `onOrganizationJoined` [35] write the Team org-id cache (`localPlayer+0x70`) correctly, on the
   same code path that writes the Command cache (`+0x74`)?** The true `Organization`-subclass member
   function behind the `MemberCallback` RTTI at `0x01e64240` was not reached this pass (it is
   heap-constructed by the subscribe call, with no static data cross-reference found — the same shape
   `bank-vault-client.md` hit for `InventoryUpdateContainerSize`'s emit site). A live x64dbg session,
   breaking on `onOrganizationJoined`'s Mercury dispatch with the player already in both a Team and a
   Command, would show whether both cache slots are populated and with what values.
3. **Is `getContainerSize(Container.TeamBank)` actually non-zero and correct at the moment of the drag?**
   Not observed live. Everything server-side computes and sends a correct, non-zero value (default 40); this
   question is about what the client's cache *holds*, which static analysis cannot read.

None of these three could be closed with the static toolchain (headless Ghidra, no debugger) available this
pass; a live x64dbg trace against a running client, with a Team and Command org already founded, is the
concrete next step.
