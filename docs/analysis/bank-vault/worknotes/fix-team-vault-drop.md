# Team Vault Drag-and-Drop Fix — Worknotes

> Type: reference. Audience: the Bank and Vault coordinator (cimmeria-23).
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [bv-07.md](bv-07.md), the finding [team-vault-drag-drop.md](../../../reverse-engineering/findings/team-vault-drag-drop.md).

## Contract

- **Follow-up, not a numbered packet**: find why the Team vault (container 19) refuses drag-and-drop from
  the backpack while the Command vault (container 20) accepts it, and fix it server-side if possible.
- **Symptom (owner playtest, 2026-09-28)**: opening the Team vault shows 40 empty slots, hover highlights
  slots, but dragging an item onto it never highlights and never sends anything — SigNoz shows zero
  `org_move_accepted`/`org_move_rejected`. The identical drag into the Command vault and the personal vault
  both work. Disbanding and re-founding the Team did not fix it.
- **Base**: `origin/main` @ `1732c6eaf` (`chore(docs): regenerate generated doc blocks`, current at task start).
- **Worktree**: `.claude/worktrees/team-vault`, branch `fix/team-vault-drop`.
- **Owned paths (this session, docs only — no code changed)**:
  - `docs/reverse-engineering/findings/team-vault-drag-drop.md` (new)
  - `docs/reverse-engineering/findings/README.md` (index row)
  - `docs/reverse-engineering/findings/inventory-state-machine.md` (closed part of open question 2)
  - this file

## Read set

- `docs/analysis/bank-vault/{README.md,audit.md,work-packets.md}`, worknote `bv-07.md` (the org-vault
  open/move design this bug sits on top of).
- `CLAUDE.md` "Project rules", `docs/reverse-engineering/evidence-standards.md`, user memory
  `reference_ghidra_headless` (headless Ghidra recipe).
- Server: `crates/base-methods/src/base/world_entry/methods/inventory/org_vault/{open.rs,access.rs}`,
  `crates/wire/src/cell/vault.rs`, `crates/base-session/src/base/organization/handlers/push.rs`,
  `crates/base-session/src/base/organization/persistence/loads.rs`, `crates/entity/src/inventory.rs`,
  `crates/entity/src/organization/types.rs`.
- Client: `Content/UI/Core/Team/{Team.lua,TeamVault.layout}`, `Content/UI/Core/Command/{Command.lua,CommandVault.layout}`
  (full-file diffs both ways), `entities/defs/enumerations.xml` (`EInventoryContainerId`, `EOrganizationType`).
- Existing RE findings: `bank-vault-client.md` (BV-E1), `organization-restoration.md` (Squad/Team/Command
  sub-object layout, the `+0x6c`/`+0x70`/`+0x74` offsets this session's lead depends on),
  `inventory-state-machine.md` (the `onClearOrgVaultInventory` open question this session partially closes).

## Evidence / hypotheses tried and their outcome

Followed the four hypotheses in the brief, plus a fifth found along the way. Full citations are in the
finding doc; this is the short version with the verdict on each:

1. **A native "accessible containers" cache gated on org state pushes [35]/[38]/[49]** — **not
   disproven, but narrowed**. `onOrganizationJoined` [35] is sent identically for both org types by
   `restore_on_login`, at every world entry, for every membership a player has (`load_memberships` has no
   `LIMIT`, so a player in both a Team and a Command — the reporting owner's case — gets both). Found a
   confirmed native mechanism that *does* cache "my Team org id" / "my Command org id" separately, by fixed
   offset (`localPlayer+0x70`/`+0x74`, read by `onClearOrgVaultInventory`'s handler `FUN_00e1dcc0`) — but
   could not reach the native code that *writes* those two fields (the true handler behind
   `onOrganizationJoined`'s `MemberCallback` RTTI, heap-constructed, no static xref found this pass) to check
   whether it treats the two org types symmetrically. This is the strongest remaining lead. **Needs a live
   trace to close.**
2. **A per-container item-type/`container_sets` compatibility check gates the drop client-side** —
   **ruled out as the client-side cause**. The server enforces "must allow 17" for both org vaults (BV-07),
   but that is a server-side move-time check, unreachable if the client never sends `moveItem`. Nothing in
   `Team.lua`'s drag chain reads any item-type/container-compatibility data before calling `moveItem`.
3. **Our client-method index (107) is wrong and lands on a different handler** — **ruled out**.
   `register_NetIn_onTeamVaultOpen` is at `0x00d7e800` and `register_NetIn_onCommandVaultOpen` at
   `0x00d7eaa0`, matching audit A-11's addresses exactly; `vault_open_method`/its tests pin 106/107/108.
4. **The declared size (40 vs 100) puts the client out of range** — **ruled out as a data problem, not
   fully ruled out as a mechanism**. The server always computes and sends a correct, non-zero `team_slots`
   (default 40, `TEAM_VAULT_SLOTS_DEFAULT`, clamped 0..100) before granting the window
   (`org_vault/open.rs::open`), for a fresh org exactly as much as an old one — re-founding proved this isn't
   about one org's stored value. Whether the *client's cache* of that size is what it should be at drag time
   was not observed live.
5. **(Found this session) The `Team`/`Command` C++ class hierarchy has an unfinished override** —
   **ruled out**. Dumped both vtables 48 slots deep and compared them side by side (see the finding doc's
   table). Every slot that differs between the two classes has its own present, distinct implementation on
   *both* sides. No missing/null/stub slot anywhere in the observed range.

Also ruled out, independently of the four hypotheses: the server's org-vault open/move code (fully symmetric
between scopes, single code path, D-BV14's size difference is the only intentional difference); `Team.lua`
vs `Command.lua` and their `.layout` files (structurally identical apart from names, confirmed by full-file
diff both ways — the earlier BV-07 audit's "identical apart from two tooltips" claim for the layouts is
independently reconfirmed here); `EOrganizationType`/`EInventoryContainerId` (both match the Rust constants
exactly, no shift).

## Commands run

All headless Ghidra, read-only, one at a time (per `reference_ghidra_headless`), against the project already
open at `<SGW client>\Working\binaries\SGW` (no GUI/MCP session was available this session):

```bash
analyzeHeadless.bat <binaries> SGW -process SGW.exe -noanalysis -readOnly \
  -scriptPath <repo>/tools/re/ghidra-headless -postScript Probe.java <tokens>
```

| Tokens | Purpose | Result |
|---|---|---|
| `D:0x00d7e800 D:0x00d7eaa0 FNSUB:MoveItem FNSUB:ContainerSize FNSUB:DragDrop FNSUB:isValidContainer FNSUB:CanDropItem` | Confirm the two open handlers are name/RTTI stubs; look for a named native drag/move validator | Both stubs as expected; no named validator function exists (consistent with `bank-vault-client.md`'s prior finding that these Lua-native bindings are anonymous) |
| `FNSUB:execMoveItem FNSUB:InventorySlot FNSUB:SlotWindow FNSUB:execGetContainerSize FNSUB:GetContainerSize FNSUB:InventoryContainer FNSUB:execIsSlotEmpty S:moveItem` | Same search, wider net | Zero hits on every token |
| `X:0x00d7e800 X:0x00d7eaa0 X:0x00d7e560` | Xrefs to the three vault-open registration stubs | Each referenced once from its own RTTI/vtable data block, contiguous with `onVaultOpen`'s known block — confirms standard per-class boilerplate, nothing more |
| `D:0x00e1dcc0 X:0x00e1dcc0` | Decompile `onClearOrgVaultInventory`'s native handler | Full decompile obtained (see finding doc); resolves `OrganizationId` to container 19/20 via the `localPlayer+0x70`/`+0x74` org-id cache |
| `D:0x00c66ad0 X:0x00c66ad0` | Identify the singleton `onClearOrgVaultInventory` reads through | `GameEntityManager::instance()`, `+0x8c` = `localPlayer` |
| `FNSUB:OrganizationJoined S:OrganizationJoined` | Find the true `onOrganizationJoined` handler | Found the RTTI: a `MemberCallback<NoSubject, Organization, void(Event_NetIn_onOrganizationJoined const*)>`; no static data xref to its type descriptor, so the handler body itself was not reached this pass |
| `X:0x01e64240 D:0x00d8a600` | Try to reach the handler via its RTTI type descriptor | No xrefs (heap-constructed, as expected); the emit-side installer is the same generic boilerplate as every other event |
| `D:0x00e1dcc0` region cross-check against `organization-restoration.md` | Corroborate the `+0x70`/`+0x74` offsets against the prior, independently-derived Q2 finding | Matches exactly — upgrades this pass's reading of those offsets from a single decompile to two independent sources (HIGH per `evidence-standards.md`) |
| `D:0x00eb4140 D:0x00eb3000` | Decompile `Team`/`Command` constructors | Both install their own vtable then call the shared `Organization_ctor_body` |
| `I:0x00eb4140,15 I:0x00eb3000,15` | Get the raw vtable addresses out of the ctor disassembly | `Team::vftable = 0x019eb1dc`, `Command::vftable = 0x019eb07c` |
| `VT:0x019eb1dc,45 VT:0x019eb07c,45` (comma-quoted) | Dump 45 vtable slots each in one call | **Failed silently** — `analyzeHeadless.bat`'s own argument handling splits on the comma even when the token is bash-quoted (Windows batch treats unquoted-at-its-level commas as argument separators); output showed `x12` (the token's own default) plus a stray `UNKNOWN TOKEN 45`. Worked around below. |
| `VT:0x19eb20c VT:0x19eb23c VT:0x19eb26c VT:0x19eb0ac VT:0x19eb0dc VT:0x19eb10c` (default 12-slot dumps, addresses pre-offset by 12/24/36 slots) | Cover slots 12-47 of both vtables in 12-slot chunks | Full 48-slot comparison obtained; see finding doc's table. **Gotcha for future headless sessions: don't rely on `VT:addr,n` for `n != 12` through this batch-file path — chain default-12 dumps at `addr + 12*4*k` instead.** |

No live debugger (x64dbg) was used. `SGW.exe` (PID 10764) happened to be running on this machine for the
duration of the session with no attached debugger; attaching one was not done without checking first (see
below), and no reply had arrived by the time this worknote was written.

## Regression proof

Not applicable — no code was changed. This session's deliverable is evidence, not a fix, per the packet's
"if it is a stock client defect... don't build it" instruction, applied here to "the cause is not proven
server-side, so no speculative server change was made."

## Known gaps / what the next session needs

- **A live x64dbg trace is the concrete next step.** With a character that already leads a Team and a
  Command (reproducing the exact reported scenario), break (non-freezing: condition 0 + log, fast resume
  off, per `feedback_x64dbg_nonfreezing_breakpoints`) on:
  - the native handler behind `onOrganizationJoined`'s `MemberCallback` (RTTI at `0x01e64240`) to see
    whether it writes `localPlayer+0x70` and `+0x74` symmetrically for `aOrganizationType` 1 and 2;
  - reads of `localPlayer+0x70`/`+0x74` (or a `getContainerSize`-style call) during an actual Team-vault
    drag attempt, to catch whatever refuses the drop before Lua's `EventDragDropItemEnters` fires.
- **I did not attach a debugger to the running `SGW.exe` (PID 10764) this session.** It was already running,
  unattached, when I found it; since it looked like it might be an active human session (not something I
  started), I sent the coordinator a status check asking whether it was safe to attach and whether they or
  the owner could reproduce the drag live while a log breakpoint was armed, rather than risk freezing or
  crashing someone's in-progress playtest. No reply had arrived by session end.
- **If a live trace confirms the org-id cache write is asymmetric**, the fix is still a client patch (native
  code, not Lua/wire), which per the packet's instructions this session does not build — report the finding
  and the smallest patch proposal instead, once confirmed.
- **If a live trace instead shows the drag-drop acceptance path doesn't consult this cache at all**, the
  next static lead is the ~69 KB generic Lua-property-registration function `bank-vault-client.md` already
  flagged (its `InventoryUpdateContainerSize` emit site was never traced either) — tracing it fully was out
  of budget for a single follow-up session.
