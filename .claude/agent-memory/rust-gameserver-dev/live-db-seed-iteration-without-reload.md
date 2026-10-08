---
name: live-db-seed-iteration-without-reload
description: Iterating on seed-driven live-DB tests without a full database reload each time; why a direct nextest ci-live-db run fails with "database sgw_<wt>_N does not exist"
metadata:
  type: reference
---

A full `tools/build-lane/reload-db.sh` took about 10 minutes on the WSL host
(2026-10-05), and `live-db-test.sh` reloads every time. For iteration:

- **Do not call `cargo nextest run --profile=ci-live-db` yourself.** Each
  live-DB test connects to its group slot's clone, `<db>_<slot>`, and only
  `tools/test-live-db.sh` creates the clones. A direct run fails every test
  with `database "sgw_<worktree>_3" does not exist`.
- **Iterate with `cargo test`, single-threaded, against the template:**
  `LANE_VERBOSE=1 bash tools/build-lane/lane.sh bash -c 'export
  DATABASE_URL=postgres://.../sgw_<worktree>; cargo test -p <crate> --lib --
  --test-threads=1 <filters>'`. Without `LANE_VERBOSE=1` the lane prints a
  summary only, so a `grep` on the output finds nothing.
- **Re-apply one chain seed file in a transaction** instead of reloading:
  delete the file's chain ids from `content_counters`, `content_actions`,
  `content_conditions`, `content_triggers` and `content_chains` (in that
  order), then `\i` the file. For `loot.sql` rows, insert with offset ids:
  in a loaded database the ammo rows already hold the next ids.
- **Mutating the loaded rows is a fast teeth check** (swap two
  `sort_order`s, delete a condition row, rerun, restore). The reported
  revert proof still needs the seed reverted and a real reload.
- Finish with one real `live-db-test.sh <filter>` run: it is the only run
  that proves the seed files load from scratch.
