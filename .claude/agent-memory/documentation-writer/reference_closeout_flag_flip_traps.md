---
name: reference-closeout-flag-flip-traps
description: Traps when a campaign close-out flips a feature flag default and completes a telemetry catalog (ammo AM-12, 2026-09-28)
metadata:
  type: reference
---

Found at the ammo close-out (AM-12, PR #1072, 2026-09-28):

- **Flipping a process-wide flag's default breaks old fixtures.** Nine pre-campaign tests in `cimmeria-cell-methods` and `cimmeria-cell` loaded ammo type 2 as an arbitrary value. Since the campaign's AM-F packet, 2 has meant `Bullet_Armor_Piercing`, so with the flag on those tests took the new reserve path. Run nextest on every crate that reads the flag, and on the crates next to them, before calling the flip done. Fix the fixture when the test is not about the feature; pin the flag off explicitly when the test pins an older message shape.
- **Worknotes leave catalog rows "for the coordinator", and they drift.** Wave 1 and Wave 2 emitted about 20 `ammo` events that the Rust catalog never listed. A source-sweep unit test (`ammo_telemetry::catalog_covers_every_ammo_event_in_the_workspace`) stops the next packet from repeating that. Grep the code for `target: "<t>"` and diff the result against the catalog before writing the observability rows.
- **The harness worktree has no `external/` junction**, so the first lane build fails in `cimmeria-entity`'s `build.rs` (Detour headers). Create the junction to the main checkout's `external/`; `mk-worktree.sh` does this, but an agent worktree made by the harness does not.
- **The build lane masks cargo's exit code** in a pipe: read the nextest `Summary` line, not the exit status.

Related: [[reference-campaign-closeout-status-docs]].
