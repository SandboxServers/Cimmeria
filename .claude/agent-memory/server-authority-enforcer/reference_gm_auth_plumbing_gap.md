---
name: reference-gm-auth-plumbing-gap
description: Cell-method dispatch has no access to access_level — systemic gap blocking GM auth on any future gm* handler
metadata:
  type: reference
---

## The systemic GM-auth plumbing gap in Cimmeria

`access_level` lives ONLY on `ConnectedClientState.access_level`
(`crates/services/src/base/mod.rs:117`). Sourced from
`account.accesslevel` DB column via
`crates/services/src/auth/handlers.rs:486-488`. Today consumed by
exactly ONE site: the chat dispatch's `SPEAKER_GM` bit computation
in `crates/services/src/base/dispatch.rs:131-133`.

The cell-method dispatch entry point at
`crates/services/src/cell/dispatch/router.rs:33` has signature
`(entity_id, method_index, args, tx, space_mgr, engine)` — NO
caller-identity parameter beyond entity_id. None of the per-
interface dispatchers (`cell_methods::being::dispatch`,
`cell_methods::ability_manager::dispatch`, ...) accept an
access_level either.

`grep access_level crates/services/src/cell` returns ZERO files —
the cell layer literally has no access to the bit.

### Why this matters

The natural implementation path for adding `gmGiveItem`,
`gmSpawnByCmd`, `gmKillTarget`, etc. (the ~120 GM cell methods) is:

1. Add a new `match` arm in the appropriate `cell_methods/...` dispatch.
2. Decode args from the byte slice.
3. Apply effect to entity / DB.

Step 0 should be: check `access_level >= GameMaster`. But there's
nothing to check against — the access_level isn't in scope.

### Required fix shape

Plumb `access_level: u32` (or an enum) through:

1. `BaseToCellMsg::CellMethodCall` — add the field; the base
   already has it on `ConnectedClientState`.
2. `dispatch_cell_method` signature — add the param.
3. Each per-interface `cell_methods::*::dispatch` — add the param.
4. Add a helper `fn require_gm(access_level: u32) -> Result<(), ()>`
   at the top of every `gm*` handler.

Without (1)-(3), step (4) cannot be added even if a contributor
remembers to. This is structural.

### Related concern: entity-class hardcode

`crates/services/src/base/world_entry/play_character.rs:89-94`
forces `class_id = 0x02 (SGWPlayer)` regardless of access_level.
The TODO says: "Until we build a separate SGWGmPlayer index
table, always use SGWPlayer (0x02) regardless of access_level."

When that TODO is addressed, the SGWGmPlayer flat index space
(80+ extra cell methods) opens up. If the access_level plumbing
isn't done FIRST or AT THE SAME TIME, every gm* handler added
will be unauthenticated by default. Couple the fixes.

### Other validation locations on the GM surface

- `cimmeria-commands` (the slash-command registry at
  `crates/commands/src/registry.rs` + `permissions.rs`) DOES
  enforce access_level — `CommandContext.access_level` is checked
  in `execute()`. But that's for chat-typed `/spawn`, `/give`,
  `/kill` commands, NOT for the wire `Event_NetOut_*` GM messages.
  The two paths are completely separate.
- The chat-command stubs at `crates/game/src/commands/gm_cmds.rs`
  (spawn, teleport, kill, give, setlevel, shutdown) register with
  `AccessLevel::GameMaster` / `Admin` — but they're TODO stubs
  with no actual effect. When wired up they'll use the slash-
  command path, not the wire cell-method path.

So: TWO GM dispatch paths. The slash-command path has gating
infrastructure. The wire cell-method path does not.
