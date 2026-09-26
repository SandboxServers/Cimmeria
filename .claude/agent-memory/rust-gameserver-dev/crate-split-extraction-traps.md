---
name: crate-split-extraction-traps
description: Compiler, lint and guard traps hit when moving modules out of cimmeria-services into a new split crate (found extracting cimmeria-wire, wave W1c, 2026-09-26)
metadata:
  type: project
---

Moving code out of `cimmeria-services` into a new crate
(`docs/architecture/services-crate-split.md`) trips the same few things
every wave. Found while extracting `cimmeria-wire` (W1c).

- **`pub(crate)` becomes dead code.** An item the monolith used across
  modules is `pub(crate)`; in the new crate nothing uses it, so `dead_code`
  fires under `-D warnings`. Widen exactly what the old crate imports to
  `pub`; leave crate-internal helpers `pub(crate)`.
- **`#![warn(unreachable_pub)]` flags `pub` FIELDS of a `pub(crate)` struct**
  (player_journal's `Entry`). Narrow the fields to `pub(crate)` rather than
  widening the struct.
- **Intra-doc links to `crate::cell::service::…`** break once the file sits
  in another crate. Rewrite them as plain code spans naming
  `cimmeria_services::…`.
- **The layering guard and globs.** A module that keeps
  `pub use constants::*` after `constants` became a `pub use cimmeria_wire::…`
  used to attribute every name behind the glob to itself (four false
  edges). `tools/layering/check.py` `_item` now returns `None` when the
  module has a glob into another crate. A re-export at the old path is
  enough to drop an edge: callers need not change.
- **The live-DB list guard** requires every crate that dev-depends on
  `cimmeria-test-support` (even only for `LogCapture`) to be in
  `tools/test-live-db.{sh,ps1}`, live-DB tests or not.
- **`target_scan_tests::every_crate_is_classified`** fails until the new
  crate is in `IN_PROCESS_CRATES`, and its literal `target:` rows stop being
  scanned until then.
- **Test-count bookkeeping.** `cargo nextest list --message-format oneline`
  prints `<binary-id> <test>`; the lib binary id is the bare package name,
  so the live-DB tier size is the lines whose first field is a listed crate.

Related: [[python-write-mangles-utf8-and-crlf]] (scripted edits),
[[lane-sh-masks-cargo-exit-code]].
