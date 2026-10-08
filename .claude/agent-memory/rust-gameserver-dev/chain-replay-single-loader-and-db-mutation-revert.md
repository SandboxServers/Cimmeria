---
name: chain-replay-single-loader-and-db-mutation-revert
description: load_single_chain_for_test skips the whole-chain refusals (unknown ability or tutorial ids); a seed guard can be revert-proven in seconds by mutating the worktree test database instead of reloading it
metadata:
  type: project
---

Two facts about chain-replay tests, found in Class Start v6 CS-05 (2026-10-05).

**`load_single_chain_for_test` does not run the whole-chain refusals.**
`refuse_chains_with_unknown_abilities` and
`refuse_chains_with_unknown_tutorials` run only in `build_engine`. A chain
with a mistyped `grant_ability` id or a non-tutorial `show_tutorial` id loads
fine through the single-chain loader and is dropped whole at server start,
taking its other actions (the item, the `complete_mission`) with it.

**How to apply:** any test module for chains that use `grant_ability` or
`show_tutorial` also needs one `build_engine(Some(&pool))` test that checks
each chain id is present with its full action count
(`engine.get_chain_actions(id).len()`). Example:
`chain_replay_tests/class_start_sgc/mod.rs`.

**Revert-proving a seed guard without a reload.** A reload of the worktree
database takes about four and a half minutes. To prove a chain-replay guard
fails when a seed row is reverted, delete or update the row in the worktree's
own database with `psql`, run the test binary directly
(`DATABASE_URL=<template url> cargo nextest run -p cimmeria-cell-content
--lib <filter>` through the lane; the default profile reads the template
database, no clones), and let the next `live-db-test.sh` reload restore it.

**Why:** the tests read the database, not the `.sql` file, so the database
row is the thing under test; several guards can be broken in one pass.
**How to apply:** say in the report that the revert was a database mutation,
and never do it to another worktree's database.

Related: [[seeds-and-content-chains-index]], [[testing-patterns-index]].
