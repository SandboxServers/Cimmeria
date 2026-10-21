# BL-07 docs report

Branch `cellblock-boot-lock/integration`, worktree `cbl-integration`. No Rust or seed files touched. All docs edited in CRLF.

## What changed

| File | Change |
|---|---|
| `docs/content/content-engine-vocabulary.md` | `movement_lock` row (3 seed rows), pointer paragraph, new "`movement_lock` params" section (params table, hold check order with log events, release, `client_flag` ref-count rule, lifetime, seeded uses) |
| `docs/architecture/movement-validation.md` | Layer row 0a, "Content movement hold" section (position in the gate order, slack, throttles, stale-space rule, telemetry, interaction with the Prison Boot effect, tested vs never-run table), three constants rows |
| `docs/architecture/state-field-bits.md` | "Who writes `BSF_MOVEMENT_LOCK`" writer table; `movement_lock` with `client_flag: true` listed, off in all seeds until BL-U2 |
| `docs/architecture/observability-target-catalog.md` | `movement.hold_snap` row and `content` `movement_lock.*` row (beyond the packet spec; telemetry is first-class) |
| `docs/gameplay/mission-system.md` | Hidden-missions paragraph: boot held until Livewire win on the prop; chain 1024 is `interact_tag` |
| `docs/content/consumable-via-onitemuse-pattern.md` | Baseline table row 1024/3438 struck through and explained (no longer `item_use`) |
| `docs/content/debug-hub.md` | Spawn 284 row plus a note that it is not a hub service or NPC |
| `docs/analysis/castle-cellblock-rebuild/uat-guide.md` | T25 rewritten for the right-click flow with exact hold and release lines and server greps; C00 row; restore-chain rows 1023 and 1047; "Effects are inert" bullet corrected for 1939 and 1942 |
| `docs/guides/unified-uat.md` | K7, T25 row and the "only a human can check" line |
| `README.md`, `docs/readme.md`, `docs/agents/doc-update-map.md`, `docs/testing/inventory/README.md` | Output of `python tools/docs-gen/regen.py` only (generated test counts were stale from BL-01..BL-06 tests); no hand edits |

## doc-update-map rows touched

- Game systems, content chains or content-engine actions (docs/content, vocabulary).
- Architecture decisions / state-flag conventions (state-field-bits.md, movement-validation.md).
- Per-system game-mechanic docs (mission-system.md).
- UAT steps for a restored system (unified-uat.md plus the campaign checklist).
- Observability catalog (new telemetry rows).
- Generated blocks: refreshed by `regen.py`, `--check` now reports "generated doc blocks are current".

Not touched on purpose: `docs/gap-analysis*`, `docs/project-status.md` (BL-09), `.github/instructions/content-chains.instructions.md` (no review rule shifted).

## Tested vs never tested

- Covered by tests: loader validation, executor hold and release (skips, ref-count, end to end), the gate (`Locked`, slack, stale space, throttles), the `EntityMove` handler (snap, one feedback line, immediate second move silent, move in slack written), live-DB chains 1023 (including Goa'uld), 1047, 1025, 1048, and the prop spawn.
- Never run in the lab: BL-U1 and BL-U2. Unverified: the prop rendering with nameplate and glow, right-click working while held, rubber-band feel at 500 ms, both holds active at once, the Goa'uld run, and the relog flows. The docs say "never run" where it matters (T25, movement-validation).

## Findings the orchestrator and architect need

1. **Campaign README F3 and D-BL5 are stale in this tree.** They say abilities 1597 and 1598 are no-ops and the hold never sets `BSF_MOVEMENT_LOCK`. MG-63b (commit 1d97b5cd6, already in the base) gave effect 1939 the `MovementHold` script and 1942 `MovementRelease` (`crates/cell-effect-scripts/src/cell/effects/movement_hold.rs`, `db/resources/Effects/Seed/effects.sql`). Chain 1023 does not launch 1597 and the verb has `client_flag: false`, so the boot lock does not set the client-side bit: effect 1939 is not cast there. The BL-U2 risk (client refuses the right-click while the bit is set) is not live from chain 1023; it stays open for BL-U2 if a later change sets the bit. I documented the code as it is.
2. **One hold for the boot lock**: chain 1023 applies only the content hold (1.0 u slack, snap to the anchor). The older effect hold (effect 1939, 0.5 m slack, counted as `movement_locked`) is not applied by this chain, so the older gate does not refuse moves because of the boot lock.
3. **Spec said the anchor is the room centre; the seed anchor is the respawner point** (-334.231, 73.472, -228.026). The prop (-333.03, 73.472, -227.32) is about 1.4 u away. Docs use the seed values.
4. **Stale seed comment**: fixed. The mission 689 section of `castle_cellblock_chains.sql` now says chain 1023 does not launch 1597.
5. The packet said `mission-system.md` line 168 mentions chain 1024's `item_use 3438`; it does not. I added the correct statement instead of correcting a mention.
6. Chain 1023 launches only the verb (no 1597), so disabling chain 1023 (the kill switch) removes the hold.

## Documentation debt

- `docs/analysis/four-zone-planning/cellblock-work-packets.md` probe is superseded (OQ-9); not touched.
- `id-allocation.md` Cellblock row (template 1250, spawn 284): BL-06 owns it; not verified here.
- Campaign docs live untracked in the main checkout only, so the uat-guide cites the campaign by name, not by link.
- Status docs (`gap-analysis*`, `project-status.md`) wait for BL-09.
- `lint-md` has pre-existing errors elsewhere (not mine); the touched pages have none from this change, apart from pre-existing MD018 at state-field-bits.md:44, MD040 at mission-system.md:88 and heading-spacing at unified-uat.md:1354 and :1448.
- Link check: the touched pages have no new broken links; two pre-existing broken links in content-engine-vocabulary.md (`conditions.rs`, `chain.rs`) remain.
