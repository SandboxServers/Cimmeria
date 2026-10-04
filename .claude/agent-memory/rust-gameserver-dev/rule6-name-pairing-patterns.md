---
name: rule6-name-pairing-patterns
description: How to pair ID fields with names (Rule 6) on hot NPC paths cheaply, and the borrow/type traps hit in the NT-25 sweep
metadata:
  type: project
---

Patterns from the NT-25 sweep (2026-10-04, NPC AI / pets / spawner):

- **Inline name expressions in a `tracing` macro are lazy.** Field values are
  evaluated only when the callsite is enabled, so
  `npc_name = space_mgr.entity_label(npc_id)` inside a per-tick `debug!` costs
  nothing with debug off. A `let n = space_mgr.entity_names(id);` before the
  macro is eager: keep it inside the branch that logs (past a throttle, or a
  rare transition).
- **`cimmeria_names::book().ability(id)` works inline** (the guard temporary
  lives to the end of the statement), but not inside a closure:
  `opt.and_then(|a| cimmeria_names::book().ability(a))` fails E0515. Bind
  `let book = cimmeria_names::book();` first.
- **`book.x(id)` takes `impl Into<i64>`**, so an `Option<i32>` id needs
  `.and_then`; same for `entity_label`, which takes `u32`, not `Option<u32>`.
- A log helper with no `SpaceManager` param (`fn log(s: &Step, ...)`) has to
  gain one; the caller's `&mut SpaceManager` reborrows as `&`.
- A function that despawns snapshots `entity_names` at its top; after
  `despawn_npc_releasing_combat` the label is `None`.
- Cover `chunk_id` / `node_id`, `spawn_id` and `loot_table_id` have no
  NameBook lookup: mark them `// nt:id-only <reason>`, one field per line.

Related: [[python-write-mangles-utf8-and-crlf]] (the stdin cp1252 trap bit
again when a heredoc block held an em dash).
