# tools/layering — crate-split layering guard

`cimmeria-services` is being split into an acyclic set of crates
([docs/architecture/services-crate-split.md](../../docs/architecture/services-crate-split.md)).
This guard keeps the split from sliding backwards while it happens: it fails
CI when a module edge in `crates/services/src` points against the planned
crate DAG and is not on the allowlist.

```bash
python tools/layering/check.py              # the check CI runs (build job)
python tools/layering/check.py --list       # current violations, allowlist format, with file:line
python tools/layering/check.py --prune      # delete allowlist lines whose edge is gone
python tools/layering/check.py --edges cell::spawner::npcs   # one module's edges, both directions
python tools/layering/check.py --modules    # every production module, its crate, its size
```

Python 3.11+ (for `tomllib`), no other dependencies. A run takes about a
second.

## Files

| File | What it holds |
|---|---|
| `check.py` | Builds the production module graph and checks it. |
| `crate-map.toml` | `[crates]`: the planned DAG, each crate's direct dependencies among the split crates. `[prefix]`/`[exact]`: the target crate of every module, by longest module-path prefix. |
| `allowlist.txt` | The violations that exist today, grouped by the plan section that removes them. |

## What fails

- **A new violation.** A module edge goes from a lower crate to a higher one
  (`upward`), or between two crates with no dependency path either way
  (`no-path`). Fix it the way the plan fixes its neighbours: move the item
  down into the lower crate and leave a `pub use` at the old path, or invert
  the call through a trait. Remapping a module in `crate-map.toml` is right
  only when the plan itself changed.
- **A stale allowlist line.** The edge is gone, so the line must go
  (`--prune`). The allowlist can only shrink; each wave of the split should
  leave it shorter.
- **An unmapped module.** Every production module must match a row in
  `crate-map.toml`, so a new module forces the question of where it belongs.

## What counts as an edge

The graph comes from source text, and `check.py` works to count only real
production dependencies:

- Comments, doc comments (so intra-doc links) and string literals are
  ignored.
- Every `#[cfg(test)]` item is removed where it sits, including a test module
  in the middle of a file and a `#[cfg(test)] mod x;` declaration, whose file
  is never read.
- `pub(in crate::…)` visibility is not an edge.
- `use a::{b, c::{d, e}}` groups are expanded; `self::`, `super::` and
  child-module paths resolve relative to the module they are written in.
- A name is followed through `use` and `pub use` re-exports, globs included,
  to the module that defines it. Importing a module is not an edge by itself;
  using an item through it is. A `pub use` of an item is an edge from the
  re-exporting module.
- A name that resolves into another crate is not an edge, including one
  reached through a glob of a module that has already moved out
  (`pub use constants::*` where `constants` is now a
  `cimmeria_wire::…::constants` re-export).

It does not see macro-generated paths other than `$crate::…`, and it
resolves names textually, so a local binding that shadows a child module's
name could create a false edge. Neither has come up in this crate.
