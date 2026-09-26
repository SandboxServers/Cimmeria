---
name: services-split-extraction-traps
description: Traps when extracting a module tree out of cimmeria-services into its own crate (services-crate-split waves) — guards that fire on a crate with no DB tests, allowlist edges that vanish, unreachable_pub, tests whose helper stays behind, and the shim for a file that stays behind
metadata:
  type: project
---

Learned extracting `cimmeria-resources` (wave W1b, 2026-09-26). Plan:
`docs/architecture/services-crate-split.md`; its §4 "Deviations" list records each wave.

- **A `cimmeria-test-support` dev-dependency forces a live-DB list entry**, even with zero
  `require_db_or_skip!` tests. `live_db_wrapper_lists_every_test_support_crate` keys on the
  dev-dependency, not on DB use. Add the crate to BOTH `tools/test-live-db.sh` and `.ps1`
  in the same order (a second guard compares them).
- **Allowlist edges into a moved module disappear.** `tools/layering/check.py` treats a
  `pub use cimmeria_x::…` re-export as external and stops, so any allowlisted edge whose
  target moved out becomes stale and fails the guard. Delete the line (`--prune`).
  `crate-map.toml` rows that now match nothing are NOT flagged.
- **`#![warn(unreachable_pub)]` + clippy `-D warnings`** flags `pub` items inside private
  modules (`mod patches_castle; pub const …`). Narrow to `pub(super)`; that is the only
  code change a pure move needs beyond widening what the compiler asks for.
- **A test whose helper stays in the monolith must be split out of the moved file.** Grep
  moved test files for `crate::mercury`, `crate::cell`, other not-yet-moved modules
  before `git mv`. It goes to a `*_tests.rs` file in services (`#[cfg(test)] mod x;` is
  invisible to the layering guard), and any private item it names must become `pub`.
- **Intra-doc links to modules left behind break silently** (`[`super::cooked_data::…`]`);
  CI never runs `cargo doc`. Turn them into plain code spans with the full crate path.
- **`module_path!()` changes** — rename `FILE_LAYERS` rows (stale-target guard catches
  it), add `cimmeria_<crate>=debug` to `OTEL_FILTER` if the modules relied on
  `cimmeria_services=debug`, and add the crate dir to `IN_PROCESS_CRATES`.
- **Drop now-unused deps from services** (`zip` left with resources); nothing warns.
- Baseline test counts: `nextest list --message-format oneline` before any `git mv`,
  then diff the moved names by suffix (the crate prefix and `::bin`/lib id change).
  `lane.sh --exclusive` queues behind every running single-slot job, so start the
  baseline first and do only non-Rust edits while it waits.

Learned extracting `cimmeria-cell-cover` (W2a), where one file (`stance.rs`) stayed:

- **Partial extraction = keep the old `mod.rs` as a shim**: `pub use
  cimmeria_x::cell::cover::*;` + `mod stance;` + its `pub use`. Zero call-site edits.
  The file left behind imports shared helpers through `super::` (the glob), so any
  `pub(crate)`/`pub(super)` helper it used must become `pub` and be re-exported.
- **check.py resolves every item used through such a shim to the shim module
  itself** (a glob from an external crate resolves to nothing). Map the shim in
  `crate-map.toml` to the crate that will own the leftover file (world here); that
  also retires the "mod re-exports leftover" allowlist edge. Check `--edges <shim>`
  first: every importer must sit at or above that crate.
- **No existing guard notices a missing `cimmeria_<crate>=debug` OTEL_FILTER row for
  a crate with no `FILE_LAYERS` row** (the file-parity guard walks file rows only).
  Add a `<crate>_events_keep_their_index` parity test and prove it fails with the row
  removed.
- A dev-dependency on the same crate with `features = ["test-support"]` beside the
  normal dependency is how a `#[cfg(any(test, feature = "test-support"))]` hook
  reaches the higher crate's tests; `pub use …::*` forwards it at the old path.
- **A function moved INTO another file layer's module prefix changes its log file**
  (W2b): the spawn functions went into `space_manager`, which `aoi.log` keeps whole.
  Put moved code in its own module, name it in the old file layer and add
  `<module>=off` to the other layer (longest match wins); pin it in `parity_tests.rs`.
- **A `pub use` shim at the old path is itself a layering edge.** When code moves UP
  (catalog -> world), a shim in the lower module is an upward edge and cannot survive
  extraction; repoint the few callers instead of shimming.

Learned finishing `cimmeria-wire` (W3a: mercury glue, messages, firehose):

- **Split a moved test FILE by test when only some tests need the monolith.** The
  plan said `mercury/aoi/tests.rs` "needs world"; one test of twelve did. Keeping the
  file would have made every AoI message id `pub` for tests alone. Move the file, then
  cut the one test into a `<old>_tests.rs` at the services root.
- **`check -p <lower>` first.** Its `never used` warnings are exactly the items the
  higher crate's check will then report as E0603; widen that list in one pass.
- **A `pub use cimmeria_x::y;` in lib.rs retires the module for the stale-target
  guard** (it looks for `mod y`), so the old `FILE_LAYERS` row fails loudly: good
  revert-proof, add the old path to the resolver's must-not-resolve list.
- **Module-name clash with a lower crate.** `is_network_noise_target` matches the
  `cimmeria_mercury::` prefix (transport crate); `cimmeria_wire::mercury` is not noise
  and must not become noise. Pin both sides with assertions when a module shares a
  name with a crate.
- **A private import keeps `super::x` paths working**: `use cimmeria_wire::…::{constants,
  names};` in the old `mod.rs` lets children still write `super::constants::…`, and
  `check.py` follows the glob out of the crate without inventing edges.
- **Python replacement text ending in `\` + newline is a line continuation** in a
  triple-quoted string (the `FILE_LAYERS` rows end in `\`); use the Edit tool there.
- `RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links" cargo doc -p <crate> --no-deps`
  checks the doc links a move touches in seconds (it also surfaces pre-existing ones).

Learned extracting `cimmeria-wire-log` (W3b):

- **A crate name can prefix-match another crate's `OTEL_FILTER` row.**
  `cimmeria_wire=debug` already covers `cimmeria_wire_log::…` (EnvFilter is a
  string prefix), so a missing row would never fail a parity test. Add the
  explicit row anyway and prove independence: rebuild an `EnvFilter` from
  `OTEL_FILTER.replace("cimmeria_wire=debug,", "cimmeria_wire=info,")` and
  assert the new crate's DEBUG row still passes.
- **A crate that logs only on hand-named targets changes no `FILE_LAYERS` row.**
  The revert-proof for its `IN_PROCESS_CRATES` entry is adding its targets to
  `scan_finds_known_targets` (only it emits them), not a file-parity test.
- **No `cimmeria-test-support` dev-dependency = the tests leave the live-DB
  tier.** They ran there as services lib tests; report the tier shrinking
  rather than listing a crate with no DB tests.
- **The coordinator may edit shared tooling (`lane.sh`) inside your worktree
  mid-wave.** Stage by path; never `git add -A`.
Learned extracting `cimmeria-minigame` (W3c, a leaf module with one outside path):

- **Don't fake a skeleton for a path into a LOWER crate.** When the moved tree's only
  outside reference is `crate::cell::messages::X` (already in wire), rewrite that import
  to `cimmeria_wire::…` (catalog's `ability_tree` precedent). A `mod cell` shim in a crate
  with no cell code misleads. rustfmt accepts the rewritten line left in the
  `self`/`super` group, so move it to the external group by hand.
- **`git ls-files --eol` before committing.** rustfmt rewrote an edited
  `parity_tests.rs` to all-LF; harmless there (index `i/lf`), but a file whose index form
  is `i/crlf` (`services/src/lib.rs`, the workspace `Cargo.toml`) would commit mixed
  endings. Count `\r\n` vs bare `\n` in every touched file.
- **Stale path entries hide in `codecov.yml` ignores.** `minigame/server.rs` had matched
  nothing since the file became a directory; re-pointing it would silently change
  coverage, so drop it and say so.
- A worktree can arrive with a coordinator's uncommitted edit (`tools/build-lane/lane.sh`):
  `git add` by explicit path, never `-A`.
Learned extracting `cimmeria-base-session` (B1, the first base-track wave):

- **Zero-edit `crate::` paths for moved code.** A private `use cimmeria_wire::mercury;`
  at the new crate root plus a private `mod cell { pub(crate) use …::messages; pub(crate)
  use …::spawner; }` made every moved `crate::mercury::…` / `crate::cell::…` path compile
  with no edit inside the moved files; the renames stayed at 100% similarity.
- **Widen field TYPES too.** Making `ConnectedClientState` `pub` fires
  `private_interfaces` ("type `DeferredAoiMsg` is more private than the item") for each
  `pub` field whose type is `pub(crate)`. The `check -p <new>` dead-code list misses
  items the new crate also uses itself (`send_to_witness_reliable`); only the higher
  crate's E0603 pass finds those.
- **An EnvFilter row matches sibling modules by string prefix.**
  `cimmeria_services::base::world_entry=trace` also kept `world_entry_chat` and
  `world_entry_appearance`, so ONE new row `cimmeria_base_session::base::world_entry`
  covers all three moved `world_entry*` modules. Check what an old row matched by
  prefix before assuming a moved module had no file.
- **A hollowed-out parent module can map to the facade.** Once `base/mod.rs` only
  declared the leftovers and re-exported, remapping `[exact] "base"` to
  `cimmeria-services` retired the `base -> base::service` allowlist line.
- **The Edit tool writes LF into CRLF files** (mixed endings). A later scripted
  mutation that matches `\r\n` silently fails its assert; compare `line.strip()` instead.
- **Something outside the wave may modify tracked files in the worktree** (the lane
  script changed mid-session). Stage explicit paths, never `git add -A`.
Learned extracting `cimmeria-cell-world` (C1, the biggest cell wave, 22k lines):

- **rustc reports privacy in phases.** The first services check shows only
  E0603/E0364 (paths); field and method privacy (E0616/E0624) appear only once
  those are fixed, and the test target adds a third wave. Loop: `check -p world`,
  widen restricted items at each dead-code site; `check -p services`, widen at
  each `::: file:line` definition site rustc prints for E0624; then
  `--all-targets`. The 452 E0624s collapsed onto six methods.
- **Dead-in-world is a proxy, not the answer.** A `pub(crate)` helper is dead
  in the new crate when its only callers are dead too; widening it gives an
  `unreachable_pub`. Grep the higher crate for each name and restore the HEAD
  visibility of the ones it never names.
- **A `#[cfg(test)]` hook other crates need** becomes
  `#[cfg(any(test, feature = "test-support"))] #[doc(hidden)] pub`, and so must
  every private `#[cfg(test)]` helper it calls, or the feature build breaks.
  A production `pub(crate)` fn behind a test-only re-export needs a gated
  wrapper instead: `pub` in a private module re-exported only under the
  feature trips `unreachable_pub` in a normal build.
- **A shim's private `mod x;` shadowing a public glob's `x`** (the services
  `ring_transport` shim declares `runtime` beside `pub use world::…::*`) trips
  `hidden_glob_reexports`; make the local module `pub`.
- **A test file under a moved directory that drives the monolith** moves back
  with `git mv` to a sibling (`detectors/tests/` -> `npc_ai/detector_tests/`);
  rewrite its `super::super::x` paths to `super::super::detectors::x`.
- **Prove a new parity test by reverting a row.** Removing
  `cimmeria_<crate>=debug` or one file row must fail it; do it with a script
  file, not a heredoc (the `\`-newline continuations get mangled).

Learned extracting `cimmeria-base-methods` (B2, a subtree two levels deep):

- **Scan `super::` chains, not just `crate::` paths, for outside edges.** Deep files
  write `super::super::super::super::helpers`; resolve each chain against the file's
  module path (a short script) to list what the skeleton must re-export. B2's
  skeleton: private `pub(crate) use` of the session modules in `base/mod.rs`, an
  inline `pub mod world_entry { pub mod methods; }`, and a private one-item
  `mod world_entry_appearance` for the builder the code names through that path.
- **A test-only re-export of a PRODUCTION item that a higher crate's test needs**
  cannot use the plain hook shape: the item is `pub` in a private module, so a
  build without the feature fires `unreachable_pub`, and narrowing it breaks the
  feature-gated `pub use` (E0364). Gate the re-export with
  `#[cfg(any(test, feature = "test-support"))] #[doc(hidden)] pub use`, and put
  `#[cfg_attr(not(any(test, feature = "test-support")), allow(unreachable_pub))]`
  on the item.
- **Run `clippy -p <new crate>` ON ITS OWN too.** In a combined `-p new -p services`
  run, services' dev-dependency turns the new crate's `test-support` feature on for
  its lib, which hides the no-feature `unreachable_pub` warning.
- **A path-compat re-export in services can lose its last user** (`base::gm_feedback`
  only served the moved handlers): services fails `unused_imports` right after the
  move. Drop the name and say why in the comment.
- **Revert-proof the tracing rows cheaply**: back up `filters.rs`, delete one row with
  a script, run the one parity test with `nextest run -p cimmeria-server <name>`,
  restore; repeat per row and for the `IN_PROCESS_CRATES` entry.

Learned extracting `cimmeria-base-world-entry` (B3, the base track's third wave):

- **The layering guard only proves PRODUCTION is clean.** `check.py --edges` showed zero
  upward edges, yet three tests drove cell code (`cell::gate_travel::handle_dial_gate`,
  `send_gate_sequence` on a `SpaceManager`). Grep the moved tree, tests included, for
  `crate::cell::(gate_travel|space_manager|spawner|…)` before `git mv`, and cut those
  tests to a services `*_tests/` dir with copies of their fixtures.
- **A test hook for a fn that its parent also imports privately under the same name**
  (`use persist_arrival::persist_arrival;`, fn == module name) needs a cfg-split pair:
  `#[cfg(any(test, feature = "test-support"))] #[doc(hidden)] pub use m::f;` plus
  `#[cfg(not(any(test, feature = "test-support")))] use m::f;`, or E0252.
- **Services' path-compat `pub(crate) use` re-exports die in bulk**, and the lib and
  lib-test targets report DIFFERENT unused sets (test-only users keep some alive).
  `sort | uniq` on `--message-format short` hides which target said what; items only
  tests use become `#[cfg(test)] pub(crate) use`.
- **Dev-dependencies die silently too**: `async-trait`, tokio `test-util` and a lower
  crate's `test-support` feature lost their last users here; grep for each after the move.
- **Don't widen a module to `pub` that services never reached.** A `pub mod` whose docs
  link private children trips rustdoc's `private_intra_doc_links` (CI never runs
  `cargo doc`); keep the old `pub(crate)` and re-export the handlers from a public sibling.
- **A FILE_LAYERS row that a sibling row covers by prefix survives every guard.** Revert-
  proof each row: removing `…::world_entry_appearance` changed nothing because
  `…::world_entry` matched it, so the row was dropped as redundant.
Learned extracting `cimmeria-cell-combat` (C2, the first trait inversion):

- **Prove an ordering guard by swapping, not by deleting.** The content-side
  tests of the pulse path stayed green with "death, then health-below"
  reversed: the real drain drops a lethal sample on `pct_after <= 0` either
  way. Only a `RecordingContentEvents` assertion on the call sequence caught
  the swap. Revert-proof each new order test by swapping the two calls.
- **Script the §2E call-site rewrite by parsing the argument list** (balanced
  parens, Nth argument), then read the forwarding wrappers: the script also
  wrapped `npc_ai_tick_for_test`'s own `dispatch::npc_ai_tick(.., events)`.
- **A test suite split by one tick call.** When 2 of 7 files in a moved test
  directory call a services tick, move the rest and put the shared fixtures
  in `<crate>::test_fixtures::<suite>` behind `test-support`; both halves
  `pub(super) use` them. Keep the `use` lines the children reach through
  `use super::*` in each half's `mod.rs` (unused-import errors tell you
  which). A single test that needs the tick moves to a same-named file in
  services, so its name does not change.
- **`source_scan::is_test_path` did not know `test_fixtures`**; a fixture's
  content-style `nav_path.clear()` tripped the world crate's nav-path guard
  the moment it lived under `crates/<new>/src/test_fixtures/`.
- **A higher crate's test naming a lower crate's private enum** (the
  `BlindInSlot` return type) cannot go through a wrapper fn; make that one
  module `#[doc(hidden)] pub mod` with the two items `pub`.
- **`clippy -p <new> -p cimmeria-services` unifies `test-support` into the new
  crate's lib**, so `unreachable_pub` on a gated hook never shows there. Run
  `clippy -p <new> --all-targets` on its own too.

Learned extracting `cimmeria-base` (B4, the top of the base track):

- **A crate whose name prefixes its siblings' names poisons their guards.** EnvFilter
  matches by string prefix, so an `OTEL_FILTER` row `cimmeria_base=debug` also matches
  `cimmeria_base_session`/`_methods`/`_world_entry`; deleting any of THEIR rows then
  changes nothing and their parity guards can never fail again. Name the crate's one
  top module instead (`cimmeria_base::base=debug`; stale-target resolves it) and pin
  sibling independence: drop each sibling row from `OTEL_FILTER` and assert its DEBUG
  row is no longer exported. Check this for any future `cimmeria_cell` crate too.
- **`otel::is_network_noise_target` compares some targets with `==`**, so a moved
  per-packet module (`connect_loop::{encrypted, cell_arms}`) silently leaves the
  `cimmeria-network` index unless the exact string is repointed; `each_level_lands_in_its_index`
  and `trace_directives_are_derived_from_the_tables` also hardcode those paths.
- **A feature that moves down keeps a forwarding feature** in services
  (`chaos-testing = ["cimmeria-base/chaos-testing"]`). `check --workspace --all-targets`
  compiles wireclient's test through feature unification, which proves the forward;
  also clippy the new crate alone with `--features <f>` for the gated branch.
- **A normal dependency can become test-only** after the last production user moves
  (`base-world-entry` only named behind `#[cfg(test)]` re-exports): drop the normal
  entry, keep the `test-support` dev-dependency. Same for a lower crate's re-export in
  services' `lib.rs` (`credential_redaction`) and a leaf dep (`cimmeria-game`).
- **Revert-proof with a mutation script**: back up `filters.rs`, apply one mutation per
  run (drop the row, widen it, restore an old file row), run
  `nextest -E 'test(parity_tests) | test(stale_target) | test(target_scan)'`, restore.
  Five mutations fit in ~1 min of lane time.
Learned extracting `cimmeria-cell-content` (C3, content + missions + ring dispatch):

- **§3's per-test destinations go stale as waves land.** It sent `gc1_escort` and
  `mission_742` to `cell`, but since C2 the NPC AI they drive is combat's: `gc1_escort`
  moved whole and only one `mission_742` test needed services. Re-derive each test's
  highest dependency with a grep of the moved tree, never from the plan's list.
- **A shim left by an earlier wave can move whole.** C1 left `ring_transport/` in
  services (dispatch, runtime, tests); its tests needed only world + content, so the
  entire directory went and services just `pub use`s it. Check leftover tests before
  keeping a shim.
- **`use lower::…::dialog;` under the old name** keeps a sibling's
  `super::super::dialog::x` compiling, and the stale-target resolver correctly treats
  it as not-a-module (pin `cimmeria_services::…::dialog` as must-not-resolve).
- **rustfmt sorts a run of `use` lines by path and leaves comments where they were**,
  so a `// In crate X` comment ends up above another crate's re-export. Write the run
  in path order yourself, or break it with a non-`use` item.
- **A staying test that calls a production `pub(super)` fn** (`execute_actions`,
  `populate_mission_context`) used B2's shape: `pub` + `cfg_attr(not(any(test,
  feature)), allow(unreachable_pub))` on the item and a gated `#[doc(hidden)] pub use`
  at the module root; staying tests import from that root, so their bodies are
  unchanged. `clippy -p <new>` alone proves the no-feature build.
Learned doing the C4-C6 preparation (moves inside the monolith, no new crate):

- **Two plan rows can contradict each other through a call chain.** §1 put
  `resync.rs` in cell-methods, but the GM respawn edge forced the respawn fork
  below methods AND console, and the fork calls the resync, so the resync had to
  go down with it. Before moving a module to its §1 home, list its callers'
  planned crates, not just its own.
- **A `pub use` of a MODULE is a layering edge** (`check.py` counts `pub use x;`
  of a module, not only of items). A back-compat shim in a sibling crate's module
  (`cell_methods` re-exporting `console::gm`) is a violation; only a
  `#[cfg(test)]` shim is invisible to the guard. Repoint production callers.
- **Don't move code under another file layer's module prefix and "fix" it with
  `=off`.** The OTLP trace index is derived from the `=trace` rows only, so an
  `=off` row keeps the file clean but not the trace index. Pick a module path no
  file row prefixes (trade went to `cell::trade`, not `cell::interactions::trade`).
- **`check.py` resolves every path to the item's definer, so it misses a path
  that passes THROUGH a higher crate's module to a lower item**
  (`super::cell_methods::inventory::flush_dirty_bandolier_ammo` from interactions,
  `crate::cell::service::npc_ai` from console). The extracted crate has no such
  module. Scan for these hops (walk each written path with `check.Resolver`,
  flag intermediate modules whose crate is not reachable) and repoint them to the
  owning crate's path (`cimmeria_cell_combat::…`, `cimmeria_wire::state_field`).
- **Place a new `mod` line in a shared `mod.rs` away from lines a parallel wave
  edits**: git conflicts on adjacent hunks, so one untouched line between is the
  minimum. `cell/mod.rs` is edited by every cell wave.

Learned extracting `cimmeria-cell-interactions` (C4, a wave whose prep did the hard part):

- **An earlier wave's cut-out test can become yours.** C3 left
  `content_tests::stargate_grant_dial` in services only because the gate dial
  was still there; once C4 moved the dial, that test's highest dependency was
  the new crate. Each wave, grep the services `*_tests` dirs left by earlier
  waves for the modules you are moving, not just the moved tree itself.
- **A module that was `pub(crate)` in services must be `pub` in the new crate**
  (`space_transfer`, `respawn`, `trade`); keep services' re-export
  `pub(crate) use` so the facade's surface does not grow, and fix any comment
  that promised "crate-internal".
- **A module turned `pub` exposes its doc to `private_intra_doc_links`**
  (`trade::wire` linked a private const). Check public AND private docs in one
  pass each: `RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D
  rustdoc::private_intra_doc_links" cargo doc -p <crate> --no-deps`, then again
  with `--document-private-items`.
- **Compiling the new crate on its own (`check -p <new>`) is the proof that no
  path passes through a higher crate's module**; its dead-code list is then the
  widening list for services' E0603 pass.
- **A hand-named target only the moved tree emits** (`bandolier.resend` in the
  resync) is the cheapest revert-proof for the `IN_PROCESS_CRATES` entry.

Learned extracting `cimmeria-cell-console` (C5b, run in parallel with C5a):

- **A hop through a higher crate's module can be free.** `console/mod.rs` named
  `crate::cell::dispatch::is_gm` (services' router module, the cell crate); the
  prep's hop scan missed it. World has a `cell::dispatch` at the same path with
  the same items, so the skeleton imports world's module and nothing is edited.
  Check a lower crate for a same-path module before repointing.
- **Services loses re-exports AND normal deps whose last user moved.** Here
  `cell::dispatch::is_gm`, `cell::space_transfer` and `cimmeria-discord`
  (`git grep -l cimmeria_discord HEAD -- crates/services/src` listed only the
  console). The compiler flags the re-exports; nothing flags the dependency.
- **An import only the moved tests use** trips `unused import` on
  `check -p <new>`: make it `#[cfg(test)]` and its crate a dev-dependency.
- **A doc link left behind into a private module of the moved tree breaks**
  (`[`crate::cell::console::stats::set_speed`]` in services' `ticks`). Grep the
  staying code for `[`crate::<moved path>` links; make them code spans.
- **The test inventory can name the wrong file.** Two "chat.rs" rows had been
  wire's tests since W1c. Find each test by `fn` name before fixing anchors.
- **Parallel waves on the same list files**: insert BEFORE the latest wave's
  line (e.g. `cimmeria_cell_console=debug` above `cimmeria_cell_interactions`),
  leaving that line untouched between you and the sibling wave's append.
- A Python mutation driver (edit, `subprocess` the lane nextest, restore in
  `finally`) ran five revert-proofs in about two minutes of lane time.
Learned extracting `cimmeria-cell-methods` (C5a, a leaf wave whose prep did the hard part):

- **Pre-flight in two scripts, then compile alone.** A `super::`-chain resolver over the
  moved tree plus a `crate::` path census listed every outside edge (one: a test-only
  shim); `check -p <new>` then compiled clean first time. Its unused-import warnings are
  exactly the `pub(crate) use` path-compat re-exports the higher crate still names
  (combat's `handle_reload` through `cell_methods::player::world`): widen those to
  `pub use`, and repeat for each E0603 `services` reports.
- **`target_scan_tests` cannot see a target below a `mod tests;` line.**
  `strip_test_module` cuts a file at its first `#[cfg(test)] mod`, and a `mod.rs` that
  declares its tests above its code (`player/combat/mod.rs`) hides every hand-named
  target in it (`player.respawn`). Grep the target's file before adding it to
  `scan_finds_known_targets`; if hidden, `every_crate_is_classified` is the revert-proof.
- **A same-crate intra-doc link through a private `use` import breaks when the import's
  module leaves the crate** (`cell_methods::inventory::bandolier::…` from the service
  loop). Point it at a public re-export; `cargo doc --document-private-items` on
  services finds these among the pre-existing noise if you grep for the moved path.
- **A services test file that mixes two dispatchers splits along them**: the three
  Missionary `mission_abandoned` tests went down with the dispatcher, the GM one stayed,
  each half with fixture copies, so only the moved tests were renamed.
- **The Bash tool mangles `\` + newline even inside a quoted `<<'EOF'` heredoc**: the
  inserted `OTEL_FILTER` row landed on the previous line. Use the Edit tool for any
  replacement that ends in a backslash.

Related: [[python-write-mangles-utf8-and-crlf]] (use byte-level scripted edits; the Bash
tool mangles `\\\r` in heredocs, so write scripts with the Write tool),
[[lane-sh-masks-cargo-exit-code]].
