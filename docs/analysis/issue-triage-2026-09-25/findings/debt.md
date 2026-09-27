# Findings — batch `debt`

Code of record: `main-ro` at 059d6038 (2026-09-25). All paths below are relative to `main-ro`.

## #531 — Test gaps surfaced during the 700-line split effort (#529)

- Verdict: REWRITE
- Priority: P3
- Labels: add `testing` if it exists (no change otherwise); do NOT add `ready-for-agent` (collection issue, not one task)
- Summary: A collection checklist of coverage gaps that the June 2026 split PRs exposed. The body still says "_none yet_" and every entry lives in comments. About half the entries are now done: the NavMesh "needs a committed `.nav` fixture" gap is closed because six real meshes are committed under `data/spaces/`, trigger matching has a dedicated test directory, and admin-api telemetry has tests. The genuinely open items are the discord config watcher, the console `resolve_target`, and a few one-to-three-line branches.
- Evidence:
  - `git ls-files data/spaces` lists `agnos.nav`, `castle.nav`, `castle_cellblock.nav`, `harset.nav`, `harset_storagerm.nav`, `sgc_w1.nav` (not LFS; no `.gitattributes` rule). The `if !path.exists()` guards in `crates/entity/src/navigation/tests/{diagnose,height}.rs` therefore run in CI. `poly_count` is asserted at `crates/entity/src/navigation/tests/diagnose.rs:292`. `navigation/tests/` holds 37 tests across 6 files.
  - `crates/content-engine/src/triggers/tests/matching_{cover,entity,health,misc,mission,stargate}.rs` cover `Trigger::matches()` (the "highest value" quick win).
  - `crates/admin-api/src/routes/telemetry/tests.rs` has 9 tests.
  - `crates/discord/src/config/parse.rs` has 21 tests, `toggles.rs` 4, `xrc.rs` 6. **`crates/discord/src/config/watcher.rs` still has 0 tests.**
  - `crates/services/src/cell/console/dispatch.rs:140` `resolve_target` still has no direct test. `cell/console/tests/p02.rs:225-229` exercises it only through other commands.
  - `ability_ranges` (`crates/services/src/cell/service/npc_ai/ability_select.rs:172`) is pinned end-to-end through the tick in `cell/service/tests/npc_ai/ability_range.rs:428` and `melee_reach.rs:229`. That covers the missing-def fallback; there is no pure-fn test of the three branches.
  - `npc_ai/{fight,wander,patrol,follow}.rs` now have heavy tick coverage from the NPC-AI campaign (NA00-NA30).
- Related/duplicates: #529 (parent), #458 (audit rollup; overlapping gap style), #416, #205 (coverage uplift issues)

### Action text

Comment:

> Re-triaged against main 059d6038. Several entries in this thread are done, so I rewrote the body into one current checklist. Done: NavMesh fixture gap (six real `.nav` meshes are now committed under `data/spaces/`, and `crates/entity/src/navigation/tests/` runs in CI), `Trigger::matches()` (`content-engine/src/triggers/tests/matching_*.rs`), admin-api telemetry (`routes/telemetry/tests.rs`), discord `parse.rs`/`toggles.rs`. Still open: discord `config/watcher.rs` (0 tests), console `resolve_target`, a pure-fn test for `ability_ranges`, and the small one-to-three-line branches.

#### New body

```markdown
## Problem
The 700-line split PRs for #529 made previously buried untested lines show up in Codecov patch reports. Quick wins were fixed inside the split PRs. This issue collects the gaps that needed fixtures or harness work. Several are done now. The open ones are below.

## Evidence
Re-checked against main 059d6038 (2026-09-25).

Done (kept for history):
- NavMesh query paths: six real meshes are committed under `data/spaces/*.nav`, so the `if !path.exists()` guards in `crates/entity/src/navigation/tests/` no longer skip in CI (37 tests; `poly_count` at `tests/diagnose.rs:292`).
- `content-engine` `Trigger::matches()`: `crates/content-engine/src/triggers/tests/matching_*.rs` (6 files).
- admin-api telemetry routes: `crates/admin-api/src/routes/telemetry/tests.rs` (9 tests).
- discord `config/parse.rs` (21 tests), `config/toggles.rs` (4), entity `navigation/xrc.rs` (6).
- NPC AI `fight/wander/patrol/follow` tick paths: covered by the NPC-AI campaign tests under `crates/services/src/cell/service/tests/npc_ai/`.

## Acceptance criteria
- [ ] `crates/discord/src/config/watcher.rs`: live reload plus debounce tested with a temp-dir harness (currently 0 tests).
- [ ] `crates/services/src/cell/console/dispatch.rs:140` `resolve_target`: direct tests for no-selection, self, and a selected entity.
- [ ] `crates/services/src/cell/service/npc_ai/ability_select.rs:172` `ability_ranges`: a pure-fn test for each of the three branches (missing def, `max_range == 0`, `min_range == 0`). The missing-def branch is currently pinned only through the tick in `tests/npc_ai/ability_range.rs`.
- [ ] `crates/entity/src/cell_entity/appearance.rs` `filter_holstered_weapon` edge branch.
- [ ] `crates/discord/src/sender/{task,http,mock}.rs`: 429 and backoff edge lines.

Rule: files in the Codecov `ignore:` list are out of scope.

## Test type
Unit (TESTING.md type 1). Each guard must fail when the branch it names is removed.

## Docs to update
None, unless the test count moves by 5% or more (docs/testing/inventory/).

## Client impact
Free (tests only).

## Domain advisor
testing-validation-engineer

## Needs a human for
Nothing.
```

## #529 — Tech debt: split 46 files exceeding the 700-line hard cap

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: The body's 46-file list is fully stale. Tiers 1 and 2 were cleared by 17 split PRs (#536-#553) in June 2026, and the Tier 3 test files it names were all split too. Recounting `crates/**/*.rs` on main shows 24 files over 700 lines. Excluding generated code and the designated effect registry leaves 22. Only 4 are over 700 lines of production code; 7 are over because of inline `mod tests` blocks, and 11 are test-only files.
- Evidence (recount: `find crates -name '*.rs' | xargs wc -l`, 700+ lines, main-ro 059d6038):
  - **Excluded by design**: `services/src/wire_log/decoders/generated.rs` 1623 (generated). `services/src/cell/effects/scripts.rs` 1648: the grow-by-match-arm effect registry named in CLAUDE.md; production code ends at line 696 and inline tests start at 697, so the production part is just under the cap.
  - **Production code over 700 (the must-split set)**:
    - `services/src/base/helpers/mod.rs` 854 (about 852 production lines; tests already in `helpers/tests.rs`). Also breaks the CLAUDE.md naming rule ("avoid `helpers.rs`").
    - `mercury/src/test_harness/peer.rs` 794 (test-harness infrastructure, not a `#[cfg(test)]` file)
    - `services/src/cell/messages/cell_to_base.rs` 752. One `CellToBaseMsg` enum; 393 lines are `///` docs. Cohesive, with no natural seam.
    - `entity/src/cell_entity/entity_struct.rs` 739. One `CellEntity` struct definition. Cohesive, with no natural seam.
  - **Over 700 only because of inline tests** (a cheap fix: move `mod tests` to `tests.rs`):
    - `discord/src/lib.rs` 777 (production 544; was already flagged in the 2026-06-20 comment)
    - `entity/src/abilities/manager.rs` 762 (production 345; already flagged)
    - `services/src/auth/handlers.rs` 776 (production 505)
    - `services/src/minigame/session.rs` 764 (production 334)
    - `services/src/cell/cell_methods/player/interaction/mod.rs` 732 (production 50)
    - `services/src/cell/missions/progression.rs` 713 (production 496)
    - `services/src/cell/service/ticks/auto_cycle.rs` 707 (production 200)
  - **Test-only files** (Tier 3 equivalent):
    - `services/src/cell/content/chain_replay_tests/mission_742.rs` 1328
    - `services/src/mercury/protocol/tests.rs` 966
    - `chain_replay_tests/mission_1324.rs` 920
    - `chain_replay_tests/mission_1326.rs` 899
    - `services/src/cell/content/executor/world/tests.rs` 819
    - `chain_replay_tests/mission_701/body.rs` 776
    - `services/src/base/helpers/tests.rs` 775
    - `chain_replay_tests/mission_681_686.rs` 731
    - `chain_replay_tests/mission_640.rs` 731
    - `services/src/mercury/aoi/tests.rs` 719
    - `content-engine/tests/interact_tag_linter.rs` 718
  - All original Tier 3 files (for example `cell_methods/player/world/tests.rs` 1410 and `service/tests/npc_ai.rs` 1377) are no longer on the list.
- Related/duplicates: #531 (coverage gaps from the splits)

### Action text

Comment:

> Recounted on main 059d6038 (2026-09-25). The original 46-file list is fully cleared (#536-#553 plus later Tier 3 test splits). New drift: 24 files over 700 lines. Excluding `generated.rs` and the `effects/scripts.rs` registry (its production code ends at line 696) leaves 22. Only 4 are over the cap in production code, and 2 of those (`CellToBaseMsg`, `CellEntity`) are single cohesive type definitions. 7 are over only because of inline `mod tests` blocks, and 11 are test-only (mostly chain-replay suites). I rewrote the body to that current list.

#### New body

```markdown
## Problem
CLAUDE.md sets a hard cap of 700 lines per file. The June 2026 sweep (#536-#553) cleared the original 46 files, but new drift has built up since then. Recount on main 059d6038 (2026-09-25): 24 `crates/**/*.rs` files over 700 lines, 22 after exclusions.

## Evidence
Survey: `find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | awk '$1>700'`.

**Excluded**
- `crates/services/src/wire_log/decoders/generated.rs` (1623): generated code.
- `crates/services/src/cell/effects/scripts.rs` (1648): the grow-by-match-arm effect registry named in CLAUDE.md. Production code ends at line 696; the rest is inline tests. Moving `mod tests` into `scripts/tests.rs` would be enough, but it needs a deliberate registry strategy first.

**A. Production code over 700 (split along a seam, or record "cohesive, no seam")**
- [ ] `crates/services/src/base/helpers/mod.rs`: 854 lines (about 852 production). Also breaks the "no `helpers.rs`" naming rule. Split by concern (the witness and bundle send-outcome types, etc.) into files with specific names.
- [ ] `crates/mercury/src/test_harness/peer.rs`: 794 (test-harness infrastructure).
- [ ] `crates/services/src/cell/messages/cell_to_base.rs`: 752. One `CellToBaseMsg` enum; 393 lines are doc comments. Possibly cohesive with no seam: decide.
- [ ] `crates/entity/src/cell_entity/entity_struct.rs`: 739. One `CellEntity` struct. Possibly cohesive with no seam: decide.

**B. Over 700 only because of inline `mod tests`**
The fix is to move `mod tests { … }` into a sibling `tests.rs`. No logic moves.
- [ ] `crates/discord/src/lib.rs`: 777 (544 production)
- [ ] `crates/services/src/auth/handlers.rs`: 776 (505 production)
- [ ] `crates/services/src/minigame/session.rs`: 764 (334 production)
- [ ] `crates/entity/src/abilities/manager.rs`: 762 (345 production)
- [ ] `crates/services/src/cell/cell_methods/player/interaction/mod.rs`: 732 (50 production)
- [ ] `crates/services/src/cell/missions/progression.rs`: 713 (496 production)
- [ ] `crates/services/src/cell/service/ticks/auto_cycle.rs`: 707 (200 production)

**C. Test-only files (lowest priority; batch them)**
- [ ] `crates/services/src/cell/content/chain_replay_tests/mission_742.rs`: 1328
- [ ] `crates/services/src/mercury/protocol/tests.rs`: 966
- [ ] `crates/services/src/cell/content/chain_replay_tests/mission_1324.rs`: 920
- [ ] `crates/services/src/cell/content/chain_replay_tests/mission_1326.rs`: 899
- [ ] `crates/services/src/cell/content/executor/world/tests.rs`: 819
- [ ] `crates/services/src/cell/content/chain_replay_tests/mission_701/body.rs`: 776
- [ ] `crates/services/src/base/helpers/tests.rs`: 775
- [ ] `crates/services/src/cell/content/chain_replay_tests/mission_681_686.rs`: 731
- [ ] `crates/services/src/cell/content/chain_replay_tests/mission_640.rs`: 731
- [ ] `crates/services/src/mercury/aoi/tests.rs`: 719
- [ ] `crates/content-engine/tests/interact_tag_linter.rs`: 718

## Acceptance criteria
- Every file in groups A and B is under 700 lines, or carries a one-line justification in this issue ("cohesive, no seam").
- Public surface unchanged (`pub use` re-exports from the new `mod.rs`); no behavior change; all tests still pass.
- One file or one tight cluster per PR.
- Before closing, re-run the survey and paste the result.

## Test type
None new. Splits must preserve behavior, and the existing suite must pass unchanged. Log any coverage gaps the splits expose in #531.

## Docs to update
`crates/README.md` only if a crate's module layout changes in a way that section describes.

## Client impact
Free.

## Domain advisor
rust-gameserver-dev

## Needs a human for
Nothing. Before starting, check that no active campaign branch is editing the file.
```

## #484 — test-infra: in-process MeterProvider tap for byte-exact counter/histogram assertions

- Verdict: KEEP
- Priority: P3
- Labels: add `enhancement` (the issue has no labels)
- Summary: Asks for a `MetricsCapture` test guard, modeled on `LogCapture`, so counter and histogram emissions can be asserted. Nothing like it exists on main: there is no `MetricsCapture`, `ManualReader`, or `init_with_provider` anywhere in `crates/`. The three motivating counter sites still exist. The body is accurate, with one design caveat: `cimmeria-observability` caches the meter in a process-global `OnceLock`, so a per-test provider needs a test-only hook, or has to go through the global provider before the first `init()`.
- Evidence:
  - `crates/observability/src/lib.rs:75` `init()` sets a `OnceLock<Meter>` from `opentelemetry::global::meter`. `:109` `meter()` and `:152/:172/:189` `get_or_register_*` exist. There is no test-provider entry point.
  - Counter sites are still present: `crates/services/src/base/crafting/persistence.rs:65,224` (`crafting_persist_attempts_total`), `crates/services/src/cell/cover/reservation.rs:62,80,88` (`cover_reservation_state`), `crates/services/src/cell/interactions/trainer.rs:79,89,105` (`trainer_opens_total`).
  - `rg MetricsCapture|ManualReader|init_with_provider crates TESTING.md` returns no hits.
- Related/duplicates: #483 (origin PR), #482; loosely #416 (instrumentation coverage) and #246 (correlation IDs)

### Action text

Status comment:

> Still open on main 059d6038: there is no metrics capture in `test_support.rs` and no test hook in `crates/observability/src/lib.rs`. All three motivating counters are still emitted (`crafting/persistence.rs:65,224`, `cover/reservation.rs:62-88`, `interactions/trainer.rs:79-105`). Design note for whoever picks this up: `observability::init()` caches a `Meter` in a process-global `OnceLock`, so a per-test `MeterProvider` needs a `#[cfg(test)]`/feature-gated override, or the tests have to install the global provider before anything calls `init()` (nextest's process-per-test model helps here).

## #458 — Test-suite remediation: 2026-05-31 codebase-wide audit rollup

- Verdict: REWRITE
- Priority: P2 (one real latent bug remains: a duplicated live-DB sentinel)
- Labels: no change
- Summary: A roughly 60-item rollup from the May 2026 test audit (`docs/testing/audit-2026-05-31.md`). Some headline items are done: the bandolier-ammo TOCTOU now filters on `item_id` and has a revert-verifier, and the TESTING.md/CLAUDE.md "eleven vs twelve" count is fixed. Several items are verifiably still open: the `0x7000_0500` sentinel collision, the missing `method_idx` ↔ `.def` conformance test, zero tests in the 7 stub `cell_methods` files, zero tests in the `defs` build script and the supervisor, `set_npc_poi`/`set_npc_ai_state` still not clearing AI scratch, and the stale test inventory. The rest (G1-G25, tightening A-H, deletes) was not individually re-verified. Too many things have landed since May to trust it without a pass.
- Evidence:
  - DONE: `crates/services/src/base/world_entry/methods/inventory/ammo.rs:35` is `AND item_id = $5`; revert-verifier at `:256`/`:293`.
  - DONE: `CLAUDE.md:146` and TESTING.md both say "twelve test types".
  - OPEN (real bug): `crates/services/src/base/world_entry/methods/inventory/core/resync_tests.rs:26` and `crates/services/src/base/world_entry/methods/mail/tests.rs:17` both declare `const TEST_BASE: i32 = 0x7000_0500;`.
  - OPEN: no `method_idx` conformance test. The only `.def`-derived conformance test is typeID/clientIndex (`crates/services/src/mercury/protocol/tests.rs:121-175`). `crates/defs/build/*.rs` has 0 tests (G22).
  - OPEN: `cell_methods/{ability_manager,black_market,gate_travel,mail,minigame,missionary,organization}.rs` all have 0 tests (scaffolding item; `contact_list/` has since gained `tests.rs`).
  - OPEN: `crates/supervisor/src` has 0 tests (G24). admin-api tests cover only `routes/dev_session/` and `routes/telemetry/` (G23).
  - OPEN (G6/G7): `crates/services/src/cell/content/executor/world/mod.rs:157` `set_npc_poi` sets `target.poi` but never clears `investigate_until`. `set_npc_ai_state` does not clear `wander_next_at`/`poi`/`investigate_until`/`follow_target_id`. Not retested dynamically.
  - OPEN (G25): each `crates/mercury/src/test_harness/tests/chaos/` scenario still uses a single primitive.
  - OPEN: `docs/testing/inventory/README.md:6` says "1,351 catalogued (stale; current 2,936)".
  - Fixture promotion: `test_support.rs` has only `make_space_manager{,_with_player}`. `make_test_space_mgr` is still defined locally (for example `cell_methods/inventory/tests/mod.rs:16`) and is referenced from 19 files.
- Related/duplicates: #531, #416, #205 (coverage), #279/#278 (negative-log audit), #213 (enum canonical source). The `method_idx` conformance item overlaps #213.

### Action text

Comment:

> Re-triaged against main 059d6038. Done since the audit: the bandolier-ammo TOCTOU (`inventory/ammo.rs:35` now filters `item_id`, with a revert-verifier), and the "eleven vs twelve test types" doc count. Still open and verified: the `0x7000_0500` sentinel is declared in both `inventory/core/resync_tests.rs:26` and `mail/tests.rs:17`; there is no `method_idx` ↔ `.def` conformance test; the 7 stub `cell_methods` files, the `defs` build script, and the supervisor all have zero tests; `set_npc_poi`/`set_npc_ai_state` still don't clear AI scratch; and the test inventory snapshot is stale. G1-G25, tightening A-H, and the delete list were not individually re-verified. The rewritten body marks them "re-verify before working". Suggest splitting the sentinel collision out as its own small `ready-for-agent` ticket.

#### New body

```markdown
## Problem
Rollup of the 2026-05-31 codebase-wide test audit (full synthesis: `docs/testing/audit-2026-05-31.md`). Re-triaged 2026-09-25 against main 059d6038. Many subsystems changed after the audit (the NPC-AI campaign NA00-NA30, Castle/Harset), so any item not marked "verified open" must be re-checked before anyone works on it.

## Evidence
### Done
- Bandolier-ammo TOCTOU: `crates/services/src/base/world_entry/methods/inventory/ammo.rs:35` filters `AND item_id = $5`; revert-verifier at `:256`/`:293`.
- TESTING.md/CLAUDE.md now agree on twelve test types.

### Verified open (2026-09-25)
- [ ] **Sentinel collision (live-DB)**: `inventory/core/resync_tests.rs:26` and `mail/tests.rs:17` both use `const TEST_BASE: i32 = 0x7000_0500;`. This is masked only by `ci-live-db` serialization, and TESTING.md forbids it. Move one of them.
- [ ] **`method_idx` ↔ `entities/defs/*.def` conformance test**: the `crates/defs` build script does not extract method indices, so a `.def` reorder shifts the hand-listed indices in `crates/services/src/mercury/mod.rs` and `cell/dispatch/constants.rs` silently. The only existing `.def`-derived guard is the typeID/clientIndex test (`mercury/protocol/tests.rs:121-175`). See also #213.
- [ ] **Scaffolding tests** for `cell/cell_methods/{ability_manager,black_market,gate_travel,mail,minigame,missionary,organization}.rs` (all have 0 tests).
- [ ] **G6/G7**: `cell/content/executor/world/mod.rs:157` `set_npc_poi` does not clear `investigate_until`, and `set_npc_ai_state` does not clear the per-state scratch fields. Write the failing test first, then fix.
- [ ] **G22**: `crates/defs/build/{codegen,def_parser,entities_xml,main,types}.rs` have 0 tests.
- [ ] **G23**: admin-api route tests exist only for `routes/dev_session/` and `routes/telemetry/`.
- [ ] **G24**: `crates/supervisor` has 0 tests.
- [ ] **G25**: every chaos scenario in `crates/mercury/src/test_harness/tests/chaos/` uses a single primitive. Add a combined one.
- [ ] **Fixture promotion**: `test_support.rs` exposes only `make_space_manager{,_with_player}`. `make_test_space_mgr` is still redefined locally and used in 19 files.
- [ ] **Test inventory regen**: `docs/testing/inventory/README.md:6` still reads 1,351 against about 2,936.

### Not re-verified (check against current code before working)
- [ ] `destroy_entity` non-disconnect witness-scrub pin; G5 disconnect leak assertion.
- [ ] G1-G4, G8-G21 (see audit doc §4).
- [ ] Tightening groups A-H (audit doc §3).
- [ ] 13 theatre-test deletes (audit doc §2).
- [ ] The `npc_ai_fight_warns_when_handle_use_ability_returns_false` LogCapture flake.

## Acceptance criteria
Each checked item lands as its own focused PR with a guard that fails when the fix is reverted. Close this rollup when the "verified open" list is empty and the "not re-verified" list has been either verified or dropped.

## Test type
Live-DB (sentinel), unit (G6/G7, conformance), wire-format (method_idx), chaos (G25). Per item.

## Docs to update
`docs/testing/audit-2026-05-31.md` (mark items resolved), `docs/testing/inventory/` (regen).

## Client impact
Free.

## Domain advisor
testing-validation-engineer; npc-ai-spawn-advisor for G6/G7.

## Needs a human for
Nothing.
```

## #416 — Workspace coverage uplift to ~88% — exclusions, prioritized tests, and ratcheted component targets

- Verdict: REWRITE (narrow it to the Phase 4 ratchet; the ~88% goal is met)
- Priority: P3
- Labels: remove `documentation`; keep `enhancement`
- Summary: A four-phase plan to take workspace coverage from 77% to about 88%. The headline goal is met: the Codecov totals API reports **89.34%** workspace coverage today. Phase 0 (exclusions) landed exactly as proposed, and much of Phases 1-3 happened through #422, the #529 split follow-ups, and the campaigns. What remains is Phase 4: every component target in `codecov.yml` still has its May value (services 62% against 90.5% measured; admin-api `auto`). mercury is still just below its own 92% target (91.62%). admin-api (28.6%) is the one crate with real residual debt, and it duplicates #458 G23.
- Evidence:
  - `codecov.yml` `ignore:` now contains `wire_log/decoders/generated.rs`, `crates/upk/**`, `crates/upk-objects/**`, `crates/server/src/otel.rs`, plus later additions (`cell/spawner/abilities.rs`, `cell/service/startup.rs`, `base/dispatch/**`, `minigame/server.rs`, …). Phase 0 is done.
  - Components on `codecov.yml:143-230`: mercury 92%, services 62%, content-engine 82%, entity 88%, game 80%, defs 84%, common 98%, commands 92%, admin-api `auto`, launcher `auto`. There are no `wireclient` or `discord` components. Phase 4 is not done.
  - Codecov API (`/repos/Cimmeria/totals/`, `/components/`, fetched 2026-09-25): workspace 89.34%; services 90.53, mercury 91.62 (below its 92 target), content-engine 92.51, entity 95.99, game 88.72, defs 91.68, common 98.99, commands 94.37, admin-api 28.62.
  - Phase 1 landed via PR #422 ("cell_dispatch + character + executor/world coverage uplift (#416)"). `base/world_entry/cell_dispatch` now has 58 tests.
  - admin-api tests exist only under `routes/dev_session/` and `routes/telemetry/`. There is no `crates/admin-api/tests/common/` router harness (Phase 2A not done).
- Related/duplicates: #458 (G23 admin-api, G24 supervisor), #531, #205

### Action text

Comment:

> Re-triaged against main 059d6038 and the Codecov API. The ~88% goal is met: workspace coverage is **89.34%**. Phase 0 landed as written, and Phase 1 went in via #422 plus later work. Phase 4 has not happened: `codecov.yml` still enforces services at 62% (measured 90.5%), leaves admin-api on `auto` (measured 28.6%), and has no wireclient/discord components. mercury sits at 91.62% against its own 92% target. I narrowed this issue to the ratchet. The admin-api harness and route tests are tracked in #458 (G23).

#### New body

```markdown
## Problem
The coverage uplift plan's goal has been reached: workspace coverage is 89.34% (Codecov, 2026-09-25), up from 77.13%. The component targets in `codecov.yml` were never ratcheted, though. They still hold the May 2026 values, so coverage can drift down 10 to 28 points in some crates before any gate trips.

## Evidence
Codecov components API (2026-09-25), measured vs `codecov.yml` target:

| Component | Measured | Target today |
|---|---:|---:|
| services | 90.53% | 62% |
| mercury | 91.62% | 92% (currently below target) |
| content-engine | 92.51% | 82% |
| entity | 95.99% | 88% |
| game | 88.72% | 80% |
| defs | 91.68% | 84% |
| common | 98.99% | 98% |
| commands | 94.37% | 92% |
| admin-api | 28.62% | `auto` |
| wireclient / discord | (no component) | none |

Done earlier: the Phase 0 exclusions (generated decoders, `upk`, `upk-objects`, `server/src/otel.rs`) are in `codecov.yml` `ignore:`. The Phase 1 services tests landed via #422 and later work.

## Acceptance criteria
- [ ] One `codecov.yml`-only PR sets each component target 2-3 points under its measured coverage (for example services 88%, entity 93%, content-engine 90%, game 86%, defs 89%, commands 92%).
- [ ] mercury: either retarget to 90% or add the `mercury/src/test_harness/pcap_replay.rs` bad-key fixture test and keep 92%.
- [ ] admin-api: replace `auto` with a fixed floor at about its current coverage (e.g. 25%), so it can only go up. The route-test work is tracked in #458 G23.
- [ ] Add `wireclient` and `discord` components with informational targets.
- [ ] The first PR after the ratchet shows every component passing.

## Test type
None (CI config only). Optional: one unit test for the pcap_replay bad-key path if mercury keeps 92%.

## Docs to update
TESTING.md or CLAUDE.md, only if either quotes component target numbers.

## Client impact
Free.

## Domain advisor
testing-validation-engineer

## Needs a human for
Nothing. Maintainer sign-off on the chosen floors.
```

## #304 — Negative-logging audit: 40+ expectation seams across cell/base/content

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: The original audit listed about 40 seams under a four-PR plan. PR1 (#377) landed the Tier 1 sweep (25 seams), `LogCapture`, and `docs/architecture/negative-logging-convention.md`. Much of the Tier 2 and Tier 3 surface was later rewritten by other work: the gate-travel split (`persist_arrival.rs` now carries the full `rows_affected`/`expected`/`reason` field set) and the NPC-AI campaign (per-tick `npc_ai.tick` rows with `decision_outcome` replace the old "no threat" and "no path" debug lines). T1-12 ("reward dispatch `todo!()`") is not a logging seam: `crates/game/src/missions/rewards.rs:52` is dead code with no runtime caller, and mission XP is tracked in the gap analysis. Several Tier 2 seams are verifiably still open.
- Evidence:
  - Done: #377 (PR1). `docs/architecture/negative-logging-convention.md` header: "Convention adopted in issue #304 PR1". `crates/services/src/base/world_entry/methods/inventory/core/remove_instance.rs:120-125` now emits `rows_affected` plus `expected` (T1-13). `cell/service/npc_ai/fight.rs:358` `reason = "handle_use_ability_returned_false"` (T1-15).
  - Superseded: `crates/services/src/base/world_entry/gate_travel/persist_arrival.rs:131-140` warns with `rows_affected = 0, expected = 1, reason = "rows_affected_zero"` (T2-10's field set, at warn rather than error). The NPC-AI tick row is documented in `docs/architecture/observability.md:232`.
  - T1-12: `crates/game/src/missions/rewards.rs:52` `todo!()`. `MissionReward` is referenced only from `crates/game/src/missions/manager.rs` (which also has `todo!()` at `:102`/`:107`), and nothing in `services` calls it. Mission XP is tracked at `docs/gap-analysis.md:1071` (`reward_xp` is 0 in seeds; no `GrantXP` executor arm).
  - Still open: T2-7, `crates/services/src/cell/spawner/respawners.rs:63` still `info!(count = …)` with no zero-count warn. T2-8, `crates/services/src/base/world_entry/teleport.rs:151-157` still `warn!` with no `rows_affected`/`expected`/`reason` fields. T2-12, there is no `expected_swap` field anywhere in `base/`.
  - T2-13: `crates/services/src/base/outbox/mod.rs:296-406` warns now carry `outbox_id`. `event_type`/`payload_type` were not confirmed.
- Related/duplicates: #458 (LogCapture flake), #484 (metrics analogue)

### Action text

Comment:

> Re-triaged against main 059d6038. PR1 (#377) shipped the Tier 1 sweep, `LogCapture`, and the convention ADR. Since then the gate-travel split and the NPC-AI campaign rewrote most of the Tier 2 NPC and gate code, so those line references no longer apply. I removed T1-12 from scope: `crates/game/src/missions/rewards.rs` is dead code with no runtime caller, and mission-XP rewards are tracked in `docs/gap-analysis.md` (no `GrantXP` executor arm yet). The rewritten body lists only the seams I confirmed still open, plus a "re-verify" list for Tier 3.

#### New body

```markdown
## Problem
Expectation seams (glossary: a call site that assumes a downstream effect landed) that still emit no signal, or emit one without the fields the convention requires, when the expectation fails. The convention is `docs/architecture/negative-logging-convention.md` (adopted in PR1, #377).

## Evidence
Done (PR1 #377 and later work): the witness-miss and disconnect seams in `base/helpers`, the `enable_entities`/`map_loaded` send errors, the `first_login` `rows_affected` guard, the content executor and dialog `let _ = tx.send`, the inventory `remove_instance`/`remove_by_type` `rows_affected`+`expected`, and the NPC `handle_use_ability` false return (`cell/service/npc_ai/fight.rs:358`). Gate travel persist (`gate_travel/persist_arrival.rs:131-140`) carries the full field set. NPC-AI decisions are visible via the `npc_ai.tick` row plus `decision_outcome` (docs/architecture/observability.md).

Out of scope: `crates/game/src/missions/rewards.rs:52` `todo!()` is dead code (no caller outside `crates/game`). Mission rewards are an implementation gap tracked in docs/gap-analysis.md, not a logging seam.

Verified open (main 059d6038):
- [ ] `crates/services/src/cell/spawner/respawners.rs:63`: `info!(count)` with no `warn!` when a world loads 0 respawners.
- [ ] `crates/services/src/base/world_entry/teleport.rs:151-157`: the `rows_affected == 0` warn lacks `rows_affected`/`expected`/`reason` fields. Decide warn vs error per the level rules.
- [ ] `crates/services/src/base/world_entry/methods/inventory/grant/`: the bandolier-slot UPDATE failure has no `expected_swap` field.
- [ ] `crates/services/src/base/outbox/mod.rs:296-406`: `mark_delivered` warns. Confirm `event_type`/`payload_type` are present.

Re-verify before working (old line refs are stale): the respawn invalid `respawner_id` entry warn, the gate-travel "no active player id" abort, `cell_dispatch/aoi.rs` create-plus-cascade phase field, `threat` re-add canary, and the Tier 3 canaries (cooldown double-start, ammo underflow, damage-to-dead, `ON_VISIBLE` append debug).

## Acceptance criteria
Each seam emits a structured log at the level the convention sets, with `reason` and, where it applies, `rows_affected`/`expected`. Each has a `LogCapture` guard that fails when the log is removed.

## Test type
Negative-log (TESTING.md type 12). Live-DB for `rows_affected` seams.

## Docs to update
docs/architecture/negative-logging-convention.md (only if a new defensible exception is added).

## Client impact
Free.

## Domain advisor
testing-validation-engineer

## Needs a human for
Nothing.
```

## #279 — BeingAppearance recomposite broadcast on equip-change (parent of #240/#249)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The issue asked for one helper that recomposes `BeingAppearance` and fans it out to the owner and AoI witnesses on equip changes and holster. Both children are closed (#240 on 2026-05-18, #249 on 2026-05-20). The helper exists as `refresh_player_appearance` on the base side, which asks the cell to fan out to witnesses, plus `request_appearance_refresh` on the cell side. It is wired into inventory move, grant, bandolier, vendor, holster, and damage paths, with regression tests.
- Evidence:
  - `crates/services/src/base/world_entry/methods/inventory/appearance.rs:38` `refresh_player_appearance`. The test `refresh_player_appearance_asks_the_cell_to_fan_out_to_witnesses` is at `:252`.
  - Callers: `inventory/move_/mod.rs:625`, `grant/grant_item.rs:657`, `cell_dispatch/bandolier.rs:69,159`, `vendor/helpers.rs:177,297`.
  - `crates/services/src/cell/abilities/messaging.rs:270` `request_appearance_refresh`, used from `use_ability/handle.rs:349` and `damage_apply/mod.rs:324`.
  - Holster: `crates/services/src/cell/cell_methods/combatant.rs:169` `request_holster_weapon_dispatches_refresh_appearance`, and a two-phase holster tick at `crates/services/src/cell/service/ticks/holster.rs:61`.
  - Player-to-player visibility (the "player B sees A" acceptance criterion) arrived with #737. Two-client UAT of that is tracked as its follow-up, not here.
- Related/duplicates: #278 (sibling, also done), #240, #249, #737

### Action text

Closing comment (reason: completed):

> Done. The recomposite-and-fanout helper exists as `refresh_player_appearance` (`base/world_entry/methods/inventory/appearance.rs:38`; it asks the cell to fan out to AoI witnesses, pinned by the test at `:252`). It is called from inventory move, grant, bandolier slot changes, and vendor flows. The holster path dispatches it too (`cell/cell_methods/combatant.rs` tests, plus the two-phase `cell/service/ticks/holster.rs`). Both children (#240, #249) are closed. Player B seeing player A depends on player-to-player AoI (#737), whose two-client validation is tracked separately.

## #278 — Witness-fanout helper for entity-method dispatch (parent of #219/#232/#240/#249/#270)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The requested `send_entity_method_to_witnesses` helper and the self-plus-witnesses variant both exist. They were added by PR #336, and #580 extended them to combat and death state. All five children are closed (#219, #232, #240, #249, #270). The property-fanout question was settled by `refresh_player_appearance` plus `broadcast_movement_type`.
- Evidence:
  - `crates/services/src/cell/abilities/messaging.rs:98` `send_entity_method_to_witnesses`, `:153` `send_entity_method_to_self_and_witnesses`, `:206` `broadcast_movement_type`.
  - Commits e2bdc13b "feat(cell/messaging): witness-fanout helpers for entity-method dispatch (#278) (#336)" and 7cd7d5ec "feat(278): broadcast player combat + death state to AoI witnesses (#580)".
  - `gh issue view`: #219 closed 2026-05-18, #232 closed 2026-06-20, #240 closed 2026-05-18, #249 closed 2026-05-20, #270 closed 2026-05-31.
  - The body's path `cell/messaging.rs` no longer exists. The helpers live in `cell/abilities/messaging.rs`.
- Related/duplicates: #279

### Action text

Closing comment (reason: completed):

> Done. `send_entity_method_to_witnesses` and `send_entity_method_to_self_and_witnesses` live in `crates/services/src/cell/abilities/messaging.rs:98,153` (added in #336, extended to combat and death state in #580), alongside `broadcast_movement_type`. All five child issues (#219, #232, #240, #249, #270) are closed. Appearance fanout goes through the separate `refresh_player_appearance` path (#279).

## #261 — Add correlation IDs / trace context across cell↔base boundaries to make distributed logs greppable

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `needs-info`; the issue has no labels
- Summary: The issue proposed a per-request `correlation_id` threaded through the `CellToBaseMsg`/`BaseToCellMsg` envelopes. Since then the project adopted option 3 (full OTel/OTLP to SigNoz) and a different correlation strategy: **identity fields** (`account_id`/`player_id`) on every event, because spans do not cross the base↔cell mpsc boundary. `instrumentation-discipline.md` states that limit explicitly and chose event fields instead. Nothing propagates per-request trace context across the boundary: there is no `correlation_id`, no `traceparent`, and no OTel `Context` in the envelopes. What is still missing is per-*request* (not per-player) correlation across that hop. Whether that is still wanted is a scope call.
- Evidence:
  - `docs/architecture/instrumentation-discipline.md:238-247`: "Spans do not cross the base↔cell boundary … `mpsc::Sender<BaseToCellMsg>` / `CellToBaseMsg` … could never inherit it". The chosen remedy is identity fields on the event.
  - `crates/server/src/otel.rs` exists (OTLP bridge, `trace_id`/`span_id` on log records). docs/architecture/observability.md is the ADR.
  - `rg 'correlation_id|traceparent|opentelemetry::Context' crates` returns no hits.
  - The per-system `logs/*.log` files still exist (`crates/server/src/logging.rs:163-168`), but SigNoz is now the main query surface.
- Related/duplicates: #484 (metrics test tap), the observability ADR

### Action text

Question for @Cadacious:

> Since this was filed, the server ships OTLP to SigNoz, and `instrumentation-discipline.md` settled correlation on `account_id`/`player_id` identity fields on every event, because spans don't cross the base↔cell `mpsc` hop. Do you still want per-*request* trace propagation across that hop (carry an OTel `Context`/trace id in `BaseToCellMsg`/`CellToBaseMsg` and re-enter it on dequeue, so a single interact → chain → ring → gate-travel flow is one trace)? If yes I'll rewrite this into that narrow ticket. If identity fields are enough, close as not planned.

## #205 — Test coverage residuals from #123 (Group A leftovers)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: Listed files that had zero tests in May 2026, plus a generic chain-replay harness. Every "open file" now has tests or is deliberately Codecov-ignored. The chain-replay pattern is established as TESTING.md type 6, with about 50 modules. The deferred items (`minigame/server.rs`, `base/cooked_data.rs`, `base/dispatch/**`, `cell/service/startup.rs`) are all in the `codecov.yml` ignore list by decision.
- Evidence (test attribute counts on main-ro):
  - `cell/service/ticks/`: 68. `cell/service/base_messages/`: 89. `base/world_entry/cell_dispatch/`: 58. `base/resources/`: 37.
  - `base/world_entry/teleport.rs`: 4. `enable_entities.rs`: 3. `map_loaded.rs`: 2. `methods/world_entry_db.rs`: 5. `cell/service/startup.rs`: 1 (and Codecov-ignored).
  - `crates/services/src/cell/content/chain_replay_tests/mod.rs` has about 50 sibling modules loading seeded chains through `build_chains_from_rows`, and TESTING.md §6 documents chain-replay as a type.
  - The `codecov.yml` ignore list includes `crates/services/src/minigame/server.rs`, `base/cooked_data.rs`, `base/dispatch/**`, and `cell/service/startup.rs`.
- Related/duplicates: #123 (parent, closed), #416, #458. #190 and #191 are still open separately.

### Action text

Closing comment (reason: completed):

> Every file on the "open" list now has tests: `cell/service/ticks/` 68, `base_messages/` 89, `cell_dispatch/` 58, `base/resources/` 37, and teleport, enable_entities, map_loaded, and world_entry_db each have direct tests. The chain-replay harness became TESTING.md type 6 (`cell/content/chain_replay_tests/`, about 50 modules). The deferred items are Codecov-ignored by decision (`codecov.yml`). Remaining coverage debt is tracked in #458 and #531.

## #282 — chore(bible): Phase 0.6 — workflow skills across doc / dev / testing / validation

- Verdict: NEEDS-OWNER (recommend CLOSE, not planned)
- Priority: P3
- Labels: no labels today; add `needs-info` if kept open
- Summary: The issue proposes 10 `.claude/skills/*/SKILL.md` workflow skills (chapter scaffolding and promotion, `/check-pre-pr`, `/new-test`, `/new-method-handler`, …) to speed up Bible chapter authoring. None of them landed: `.claude/skills/` does not exist, and `.github/spec-paths.yml` (a prerequisite named in the acceptance criteria) was never extracted. The project went another way. It uses 16 domain agents in `.claude/agents/`, a `/re-verify` command (`.claude/commands/re-verify.md`), and the live research lab. Bible chapter authoring, which this issue was meant to speed up, has not moved since May (see #264). Whether this is still wanted depends on the #264 decision.
- Evidence:
  - `main-ro/.claude/`: `agent-memory/ agents/ commands/ plans/ superpowers/`. There is no `skills/` directory.
  - `.claude/commands/re-verify.md` is the only project-specific workflow command. The rest are generic frontend and game-design packs.
  - `.github/workflows/spec-touch.yml` and `spec-lint.yml` exist. `.github/spec-paths.yml` does not.
  - `docs/drafts/spec/` still holds the same three drafts. The last substantive Mercury-draft commit is c2d89953 (2026-05-23).
- Related/duplicates: #264 (parent), #262 (overlapping "workflow primitives (skills)" section)

### Action text

Question for @Cadacious (bundle it with the #264 question):

> None of the 10 Phase 0.6 skills landed. `.claude/skills/` doesn't exist, and the workflow grew domain agents plus `/re-verify` instead. Bible authoring (#264) has been idle since May. Is Phase 0.6 still wanted? My recommendation is to close this as not planned and, if the Bible resumes, file individual skills only when a concrete workflow needs one (`/new-test` against the TESTING.md picker is the most reusable of the ten).

## #281 — Headless wire-level test client ("wireclient") — Castle Cellblock end-to-end harness

- Verdict: REWRITE
- Priority: P3
- Labels: remove `documentation` (the ADR is written); keep `enhancement`
- Summary: The body still says "Status: implementation plan, not yet started", but Phase 1 shipped in #376. `crates/wireclient` has 30 tests covering SOAP auth, byte-exact handshake builders, and JSONL session-trace load and diff, and the ADR `docs/architecture/wireclient.md` exists. The ADR also changed the design from the body's scripted-step oracle to a hybrid pcap-baseline replay (byte-exact for base messages, behavioral for entity methods). Phases 1.5-7 are all pending, and the crate has no UDP socket yet. The server-side LOS parity check for player `useAbility` that this issue asks for is still absent: NPCs have LOS policy, players do not.
- Evidence:
  - `crates/wireclient/src/{auth,client,error,handshake,lib,session_trace}.rs` (1,727 lines) and `tests/{auth_smoke,trace_load}.rs`. Commit 0d272c43 "feat(wireclient): Tier 3 headless test client scaffold (#281, Phase 1) (#376)".
  - `docs/architecture/wireclient.md` Status: "Phase 1 accepted, Phases 1.5–7 pending … the crate contains no UDP socket". Phase table at `:244-253`.
  - `rg -i 'los|line_of_sight' crates/services/src/cell/abilities` (production code) has no LOS gate on the player ability path. NPC LOS lives in `crates/entity/src/navigation/line_of_sight*.rs` and `npc_ai`.
  - There is no `wireclient-e2e` nextest profile in `.config/nextest.toml`.
  - Overlap: the live research lab (`docs/architecture/live-research-lab.md`, epic #690) now covers some "what does a real client send" questions against the real SGW.exe.
- Related/duplicates: #690 (live research lab), #459 (the security audit covers player-ability LOS trust), #63 (movement validation)

### Action text

Comment:

> Re-triaged against main 059d6038. Phase 1 shipped in #376 (`crates/wireclient`, 30 tests), and `docs/architecture/wireclient.md` is the ADR. It moved the design to a pcap-baseline hybrid diff. Phases 1.5-7 are all pending, and the crate has no UDP loop yet. I rewrote the body to the ADR's phase table so the next phase (1.5: a UDP send/recv loop plus the first encrypted round-trip against a spawned BaseApp) is the obvious pick-up. The player-side LOS parity item is still valid and could be split out as its own server-authority ticket.

#### New body

```markdown
## Problem
No test proves that a sequence the server accepts is one a real client could have sent. `cimmeria-wireclient` is the Tier 3 harness meant to close that gap by driving the full protocol (SOAP auth, Mercury handshake, encrypted gameplay) and diffing against recorded reference sessions. The design is `docs/architecture/wireclient.md` (ADR).

## Evidence
- Phase 1 is done (#376): `crates/wireclient` has SOAP auth Phase 1+2 against an in-process `AuthService`, byte-exact handshake builders/parsers, and JSONL `session_trace` load plus diff classification. 30 tests.
- The ADR's status section says the crate has no UDP socket. `Client::from_handshake` is test-only, and `Trace::c2s()/s2c()` have no consumer.
- Player `useAbility` has no server-side LOS gate (`crates/services/src/cell/abilities/`). NPCs use `crates/entity/src/navigation/line_of_sight*.rs`.

## Acceptance criteria
Per the ADR phase table:
- [ ] **1.5**: UDP send/recv loop and the first encrypted round-trip against a spawned BaseApp.
- [ ] **2**: `mapLoaded()` plus an initial entity-hydration assertion into Castle Cellblock.
- [ ] **3**: entity mirror, a behavior-trace module, and a semantic diff against a reference pcap.
- [ ] **4**: Castle Cellblock script (steps 1-8, 10, 12-20).
- [ ] **5**: real combat at step 9, plus server-side LOS parity on the player ability path, with a negative test for spoofed-LOS fire. (Could be split into its own server-authority ticket.)
- [ ] **6**: a `#[cfg(test)]` minigame force-victory hook that production builds cannot reach.
- [ ] **7**: nextest `wireclient-e2e` profile and a CI workflow (path-filtered).

## Test type
Wire-level replay (TESTING.md type 11). Negative-log and server-authority negative tests for phase 5.

## Docs to update
docs/architecture/wireclient.md (phase table), TESTING.md type 11, the `cimmeria-wireclient` row in crates/README.md.

## Client impact
Free.

## Domain advisor
bigworld-engine-advisor (protocol), testing-validation-engineer; server-authority-enforcer for phase 5.

## Needs a human for
A decrypted reference capture of a Castle Cellblock run (pcap plus `keys.txt`). Only a 5-event head fixture is checked in.
```

## #264 — Cimmeria Bible: canonical, evidence-backed spec for the SGW server emulator

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `needs-info`
- Summary: The umbrella for a 17-chapter evidence-backed spec. Phase 0 (the meta layer) is in place: `docs/spec/{README,conventions,glossary,how-to-read,how-to-write}.md`, the `tools/spec-lint` crate, and the warn-only `spec-lint.yml`/`spec-touch.yml` workflows. Phase 0.5 stalled at three drafts in `docs/drafts/spec/` (Mercury, position-updates, entity-property-sync), none promoted to `docs/spec/`. Their last substantive authoring was May 2026; later touches are claim corrections (#704, #775). Phase 1 (11 gameplay chapters) has not started. The Bible apparatus is still live infrastructure (blocking figure lints in CLAUDE.md, the `docs/spec/glossary.md` domain glossary referenced by docs/agents), so the question is whether chapter authoring continues.
- Evidence:
  - `docs/spec/README.md` status table: "0.5 … First three `spec.protocol` chapters drafted … remaining three queued. 1 — gameplay: Not yet authored."
  - `docs/drafts/spec/`: `entity-property-sync.md`, `mercury-wire-format.md`, `position-updates.md`, `figures/`. Mercury draft history: last feature-level edit c2d89953 (2026-05-23).
  - Child issue states: #263 closed, #283 closed, #282 open (no skills landed), #262 open (no ledger), #295/#318 open (Mercury chapter corrections).
- Related/duplicates: #262, #282, #295, #318

### Action text

Question for @Cadacious:

> Bible status on main 059d6038: Phase 0 (meta layer, spec-lint, warn-only CI) is done. Phase 0.5 is three drafts in `docs/drafts/spec/` that were never promoted, with no authoring since May. Phase 1 hasn't started. Is the Bible still an active program? (a) **Yes**: I'll rewrite this into a current checklist (promote the three drafts, fold in #295/#318, author the remaining three Phase 0.5 chapters). (b) **Shelved**: close #264/#262/#282 as not planned, keep `docs/spec/` meta, the glossary, and the drafts as reference, and state in `docs/spec/README.md` that chapter authoring is paused.

## #262 — Adopt a Spec-Extraction Development framework: evidence ledger, reachability graph, round-trip verification

- Verdict: NEEDS-OWNER (recommend CLOSE, not planned)
- Priority: P3
- Labels: add `needs-info`
- Summary: Proposes an SQLite evidence ledger (`.cimmeria-workflow/evidence.db`), a Rust importer, claim markdown, skills, and a round-trip CI gate, in five phases. None of Phase 1 exists. Parts of the underlying need are now met by other tools: `tools/spec-lint` catches citation rot for spec chapters, the `/re-verify` command plus `tools/re_parity.py` verifies reconstructions against the binary, the live research lab (#690) covers "verify against the running client", and the rules "check docs/ first" and "a ticket is a claim, not evidence" are codified in `docs/agents/`. The ledger itself was never started, and it depends on the stalled Bible (#264).
- Evidence:
  - `.cimmeria-workflow/` and `tools/evidence-importer/` do not exist on main-ro.
  - `tools/spec-lint/` exists (crate `cimmeria-spec-lint`). `.claude/commands/re-verify.md` references `tools/re_parity.py`.
  - `docs/agents/rules-and-gotchas.md` and `docs/agents/domain.md` codify the provenance rules (CLAUDE.md "Project rules").
- Related/duplicates: #264, #282

### Action text

Question for @Cadacious (bundle with the #264 question):

> The evidence-ledger framework never started (no `.cimmeria-workflow/`, no importer). Since then, spec-lint, `/re-verify` with `re_parity.py`, the live research lab, and the `docs/agents/` provenance rules each cover part of the problem it described. Recommend closing as not planned unless you want the ledger revived alongside a resumed Bible.

## #246 — Follow-ups from #245 / #244 review: pre-existing bugs and code-quality nits surfaced by the file splits

- Verdict: REWRITE
- Priority: P3
- Labels: add `enhancement`
- Summary: A 10-item list from the #245 split review. The only real bug, item 1 (`active_objective_ids` carrying `step_id`), is fixed: the persist path now sends real objective ids, pinned by a test that names the pre-H50 bug. Item 2 (the mission_641 guard) is tightened, and item 10 was already resolved. The rest are low-value cleanups that are still open: 19 unchecked `as i32` narrowings in the loader, the `removed_flags` name, the "earlier draft" comment, the triplicated `player_id` parse, and outbox payload exhaustiveness. Items 3 and 9 need a quick re-check.
- Evidence:
  - Item 1 fixed: `crates/services/src/cell/missions/persist.rs:6` doc records "`advance_step` wrote `active_objective_ids: vec![step_id]`". `:48` builds real objective ids. The test at `crates/services/src/cell/content/executor/mission.rs:499-514` asserts "active_objective_ids must hold the step's OBJECTIVE ids — pre-H50".
  - Item 2 fixed: `crates/services/src/cell/content/chain_replay_tests/mission_641.rs:101-130` sets `step_2121_status = completed` and asserts chain 1051 does not fire.
  - Item 4 open: `crates/content-engine/src/loader/action.rs` has 19 `as i32` casts (e.g. `:25,38,46,68,154,247`). Only one `try_from` (`:328`).
  - Item 5 open: `crates/services/src/cell/content/executor/dialog/mod.rs:252` still `removed_flags`.
  - Item 6 open: `crates/services/src/base/world_entry/cell_dispatch/bandolier.rs:36` still "an earlier draft used `active_bandolier_slot`".
  - Item 7 open: `crates/services/src/base/connect_loop/account_arms.rs:111,131,143` has three copies of `i32::from_le_bytes([payload[0..4]])`.
  - Item 8 open: `crates/services/src/base/outbox/tests/payload.rs` covers `ItemUsed`/`InventoryItemGranted`/`InventoryItemRemoved` by hand, with no exhaustive match.
  - Item 3: `crates/mercury/src/packet/tests.rs:48-105` has `FLAG_HAS_SEQUENCE` round-trip tests. Confirm they are byte-exact on `legacy::Packet::encode()` (`packet/legacy.rs:95`).
- Related/duplicates: #529 (split effort), #458

### Action text

Comment:

> Re-triaged against main 059d6038. The critical item (1, `active_objective_ids` = `step_id`) is fixed: `cell/missions/persist.rs` now sends real objective ids, pinned in `executor/mission.rs:499-514`. Item 2 (mission_641 guard) is tightened, and item 10 was already resolved. What's left is low-risk cleanup, so I rewrote the body to that list.

#### New body

```markdown
## Problem
Code-quality follow-ups from the #245 split review (pre-existing code, moved verbatim). The one real bug, `active_objective_ids` carrying `step_id`, is fixed (`cell/missions/persist.rs`). These cleanups remain.

## Evidence
Re-checked on main 059d6038.

## Acceptance criteria
- [ ] `crates/content-engine/src/loader/action.rs`: replace the 19 unchecked `as i32` narrowings (slot, qty, level, regionId, …) with the existing `i32::try_from` plus warn-and-drop pattern (`:328`). Add a unit test with an out-of-range value.
- [ ] `crates/services/src/cell/content/executor/dialog/mod.rs:252`: rename `removed_flags` to `remaining_flags`.
- [ ] `crates/services/src/base/world_entry/cell_dispatch/bandolier.rs:36`: drop the "earlier draft used `active_bandolier_slot`" history comment.
- [ ] `crates/services/src/base/connect_loop/account_arms.rs:111,131,143`: extract the triplicated `player_id` payload parse into one helper.
- [ ] `crates/services/src/base/outbox/tests/payload.rs`: add an exhaustive `match` over `CellOutboxPayload` so a new variant without coverage fails to compile.
- [ ] Verify `crates/mercury/src/packet/tests.rs` has byte-exact coverage of both `FLAG_HAS_SEQUENCE` branches of `legacy::Packet::encode()` (`packet/legacy.rs:95`). Add it if not.
- [ ] Verify `chain_replay_tests/mission_638.rs` `require_db_or_skip!` usage against the macro definition.

## Test type
Unit (loader narrowing, encode byte-exact). The rest are cosmetic.

## Docs to update
None.

## Client impact
Free.

## Domain advisor
rust-gameserver-dev

## Needs a human for
Nothing.
```

## #213 — Game enum / constant data has no canonical source — drift across PG enums, Rust consts, and python/Atrea + python/common/Constants.py

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `needs-info`
- Summary: The issue asks which of three architectures (DB-canonical, code-canonical, manifest-canonical) should own enum and constant data. Its suggested first step, porting the high-pain tables, is done: `ARCHETYPE_ITEM_EVENT_SETS` is `archetype_item_event_set()`, and `bag_max_slots` has pinning tests. The trainer lookup is DB-driven (`trainer_abilities` keyed by `(list_id, archetype_id)`). A de-facto fourth option has also emerged: tests treat the client's own data files as canonical and pin Rust constants against them (`entities/defs/enumerations.xml` for the faction reaction table; `entities.xml` plus `.def` for wire typeIDs). What's left is an architecture decision, plus the missing `method_idx` ↔ `.def` conformance (#458). The body's `python/` paths are stale; they are now under `deprecated/python/`.
- Evidence:
  - `crates/services/src/cell/spawner/abilities.rs:31-39` `archetype_item_event_set` (ported from `Constants.py:ARCHETYPE_ITEM_EVENT_SETS`).
  - `crates/services/src/base/resources/mod.rs:28` `bag_max_slots`, with tests in `base/resources/tests/inventory_slots.rs:42,66`.
  - `crates/services/src/cell/interactions/trainer.rs:11-12` (per-archetype trainer lists come from the DB).
  - Conformance-test precedent: `crates/services/src/cell/combat/faction_reaction.rs:101-105` reads `entities/defs/enumerations.xml`. `crates/services/src/mercury/protocol/tests.rs:121-175` derives clientIndex from `entities.xml` plus `.def` `<ServerOnly/>`.
  - `BSF_IN_COMBAT` now lives at `crates/services/src/cell/combat/state.rs:61`, not `use_ability.rs`.
  - Legacy files: `deprecated/python/Atrea/enums.py`, `deprecated/python/common/Constants.py`.
- Related/duplicates: #458 (method_idx conformance, G12), #316 (PROTOCOL_DIGEST computed from entity defs)

### Action text

Question for @Cadacious:

> The "port the high-pain tables first" step is done (`archetype_item_event_set`, `bag_max_slots`, DB-driven trainer lists). A pattern has emerged without being decided: the client's own files (`entities/defs/enumerations.xml`, `entities.xml`, `*.def`) act as canonical, and Rust constants are pinned by tests that read them (`faction_reaction.rs:101`, `mercury/protocol/tests.rs:121`). Do you want to adopt that as the rule ("client data files are canonical; every hand-written Rust enum or constant with a client counterpart gets a conformance test")? If so, I'll rewrite this into a checklist of the constants that still lack a guard (method indices per #458, `EStateField` bits, `EInventoryContainerId`, archetypes). Otherwise, pick A, B, or C from the body.

## #167 — Test coverage: DB-driven testing opportunities beyond group A

- Verdict: REWRITE
- Priority: P3
- Labels: add `enhancement`
- Summary: A ranked list of live-DB guard opportunities from May 2026. The Tier 1 account-isolation items are done: delete and visuals have wrong-account tests, and auth credentials were rewritten for argon2id with 16 tests. The `drain_undelivered` fragility is fixed with per-entity cleanup. Grant concurrency covers `reserve_free_inventory_slots`. Still open: character-create name-collision and `char_def` validation, direct live-DB tests for `remove_instance`/`remove_by_type` (the files have none), a direct `item_allows_container` test, and gate-travel `known_stargates` append idempotence.
- Evidence:
  - Done: `crates/services/src/base/character/delete_live_db_tests.rs:101` `handle_delete_character_with_wrong_account_does_not_delete`. `base/character/request_visuals_live_db_tests.rs:319` `…_account_mismatch_logs_warn`. `crates/services/src/auth/credentials.rs` (16 tests; `enabled` check at `:146`). `base/outbox/tests/live_db.rs` scopes cleanup by `TEST_ENTITY_BASE`. `base/world_entry/methods/inventory/grant/tests.rs:175-183` concurrency.
  - Open: `crates/services/src/base/character_create_live_db_tests.rs` has only `create_character_persists_session_access_level` (`:148`), with no name-collision or `char_def_id` test. `base/world_entry/methods/inventory/core/{remove_instance,remove_by_type}.rs` have 0 tests (only `resync_tests.rs` 1 and `use_instance_tests.rs` 3 in that directory). `item_allows_container` (`inventory/grant/validation.rs:27`) is exercised only indirectly (`move_/tests.rs:62`). There is no gate-travel `known_stargates` idempotence test.
  - Paths in the body are stale (`inventory/core.rs`, `grant.rs`, `gate_travel.rs` were split into directories).
- Related/duplicates: #166, #458, #79

### Action text

Comment:

> Re-triaged against main 059d6038. The Tier 1 account-isolation guards landed (delete and visuals wrong-account tests; auth rewritten on argon2id with 16 tests), and the outbox drain fragility is fixed. I rewrote the body to the remaining gaps with current paths.

#### New body

```markdown
## Problem
Live-DB regression guards are still missing for several SQL contracts where a changed predicate or DELETE-vs-UPDATE branch would corrupt data silently.

## Evidence
Done: account isolation on delete and visuals (`base/character/*_live_db_tests.rs`), auth credential checks (`auth/credentials.rs`, 16 tests), outbox drain isolation (`base/outbox/tests/live_db.rs`), and grant slot-reservation concurrency (`inventory/grant/tests.rs`).

## Acceptance criteria
- [ ] `crates/services/src/base/character_create.rs`: live-DB tests for name-collision rejection (unique constraint) and invalid `char_def_id` rejection (`character_create_live_db_tests.rs` has only the access-level test).
- [ ] `crates/services/src/base/world_entry/methods/inventory/core/remove_instance.rs`: full-stack DELETE vs partial UPDATE, plus the outbox enqueue in the same transaction. The guard must fail if the full-stack path regresses to UPDATE-to-zero.
- [ ] `…/inventory/core/remove_by_type.rs`: order-of-walk (`container_id, slot_id`) and the "no rows for this design id" early return.
- [ ] `…/inventory/grant/validation.rs:27` `item_allows_container`: all three branches, including the empty-`container_sets` → main-bag-only fallback, against real seed rows (query them at runtime rather than hard-coding ids).
- [ ] `crates/services/src/base/world_entry/gate_travel/`: `known_stargates` append is idempotent when travelling the same gate twice, and the UPDATE is bound to `account_id`.

## Test type
Live-DB (TESTING.md type 3), with `require_db_or_skip!`, a fresh sentinel range, and cleanup by exact sentinel.

## Docs to update
docs/testing/inventory/ only if the total moves by 5% or more.

## Client impact
Free.

## Domain advisor
database-persistence; items-systems-advisor for inventory.

## Needs a human for
Nothing.
```

## #166 — Live-DB test fixtures: hard-coded resource ids + outbox cleanup pattern across merged group A tests

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change
- Summary: Pattern 1 (the dead `DELETE … WHERE entity_id < 0` outbox cleanup) is fixed. `vendor/purchase/tests.rs` now carries a regression guard that fails if the clause is reverted. Pattern 2 (hard-coded seed ids such as vendor template 25 and type 21/5228) is still there in `buyback`, `paid_repair`, `purchase`, and `data` tests. The issue itself rated this "low priority, fold opportunistically", and its mitigation still holds: CI loads the same seeds, and the colo DB is rebuilt from the seeds. TESTING.md's "don't trust seed data; re-fetch" gotcha covers new tests. Keeping it open adds no information.
- Evidence:
  - `crates/services/src/base/world_entry/methods/vendor/purchase/tests.rs:323-364`: guard documenting that "the clause was `entity_id < 0`, which matched zero rows"; a revert fails it.
  - Still hard-coded: `vendor/buyback/tests.rs:20,23,26` (`SEEDED_VENDOR_TEMPLATE_ID = 25`, `BUYBACK_TYPE_ID = 21`), `vendor/paid_repair/tests.rs:21-32`, `vendor/purchase/tests.rs:22-36`, `vendor/data/tests.rs:28`.
- Related/duplicates: #167, #458

### Action text

Closing comment (reason: not planned):

> Pattern 1 is fixed: `vendor/purchase/tests.rs:323-364` now guards against the `entity_id < 0` no-op cleanup. Pattern 2 (hard-coded seed ids in the vendor tests) remains, but, as this issue said, it's low priority. CI and the colo both load the same seeds, and TESTING.md's "re-fetch, don't trust seed ids" gotcha stops new instances. Closing rather than keeping a standing cleanup ticket. The next PR that touches one of those test files should switch it to a runtime pick.

## #76 — Legacy code audit: archive C++ src/ and identify Python scripts still needed as reference

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The audit's main recommendation (archive the C++ and keep the Python as reference) is done: the whole legacy tree lives under `deprecated/` (`cpp/`, `python/`, …), and `codecov.yml` ignores it. Of the "confirmed Rust bugs", QR betavariate, state-flag refcounting, the mission `repeats` UPSERT, fragment reassembly, and keepalive are fixed. Trade, crafting, vendor, trainer, and mob AI all have Rust implementations now. Of the security items, passwords moved to argon2id and `developer_mode` defaults to off. Auth brute-force limiting belongs to the security audit (#460 CAT-A). The CLAUDE.md "port 8989" claim is gone. Nothing actionable remains that isn't tracked elsewhere.
- Evidence:
  - `main-ro/deprecated/`: `cpp cpp-build cpp-config cpp-deps cpp-tools data-scripts db-monolithic-sql entities-editor python`. There is no top-level `src/` or `python/`.
  - QR: `crates/services/src/cell/combat/damage/qr.rs:79` and `cell/abilities/rng.rs:12` implement `betavariate(α, α + qr * mult)`.
  - Refcount: `state_flag_counts` in `crates/entity/src/cell_entity/entity_struct.rs` (CLAUDE.md abilities ADR row).
  - `repeats`: `crates/services/src/base/world_entry/methods/missions/mod.rs:104` plus the regression guard `tests.rs:119`.
  - Reassembly and keepalive: `crates/mercury/src/channel/tests/reassembly.rs`, `channel/tests/channel_lifecycle.rs`, `channel/channel_core.rs`.
  - Auth: `crates/services/src/auth/credentials.rs:23-24` (argon2 `PasswordVerifier`). `crates/common/src/config.rs:238` `default_config_developer_mode_off`.
  - Not addressed and not tracked: the cache-stamp delta on AoI re-entry (`cached_entity.cpp`). Rust still sends a full create. File it separately only if a client-side symptom is observed.
- Related/duplicates: #460 (auth brute-force), #434 (encryption modernization)

### Action text

Closing comment (reason: completed):

> Done. The legacy tree is archived under `deprecated/` (C++ and Python kept as reference, Codecov-ignored). The audit's confirmed Rust bugs are fixed: QR betavariate (`combat/damage/qr.rs`), state-flag refcounting (`state_flag_counts`), mission `repeats` UPSERT (`missions/mod.rs:104` plus a guard), Mercury fragment reassembly and keepalive (`mercury/src/channel/`). Trade, crafting, vendor, trainer, and mob AI are implemented in Rust. On security: passwords are argon2id (`auth/credentials.rs`), `developer_mode` defaults off, and login brute-force limiting is tracked in #460. One item isn't tracked anywhere: the AoI re-entry cache-stamp delta from `cached_entity.cpp` (Rust sends a full create). It deserves its own issue only if a client symptom appears.

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 531 | REWRITE | P3 | About half the logged gaps are done (NavMesh fixtures now committed, trigger matching and admin-api telemetry tested). Discord watcher, `resolve_target`, and `ability_ranges` pure tests remain. |
| 529 | REWRITE | P3 | The original 46 files were all split. Recount: 24 over 700 (22 after exclusions): 4 production, 7 over only because of inline tests, 11 test-only. |
| 484 | KEEP | P3 | No MetricsCapture or test MeterProvider exists. The three counter sites are still unguarded. Note the OnceLock meter design constraint. |
| 458 | REWRITE | P2 | Bandolier TOCTOU and the doc count are fixed. The 0x7000_0500 sentinel collision (mail vs resync tests) is still live, and there is no method_idx conformance test. |
| 416 | REWRITE | P3 | Workspace is at 89.34%, so the goal is met. Narrow to the Phase 4 codecov component ratchet (services target 62% vs 90.5% measured; mercury below its 92% target). |
| 304 | REWRITE | P3 | PR1 (#377) plus later rewrites closed most seams. T1-12 is dead `game` crate code. The respawner zero-count, teleport fields, and grant `expected_swap` seams remain. |
| 279 | CLOSE (completed) | P3 | `refresh_player_appearance` fans out to witnesses from move, grant, bandolier, vendor, and holster. #240 and #249 are closed. |
| 278 | CLOSE (completed) | P3 | `send_entity_method_to_witnesses` and the self-plus-witnesses variant exist (#336, #580). All 5 children are closed. |
| 261 | NEEDS-OWNER | P3 | OTLP and identity-field correlation replaced the proposal. Per-request trace propagation across the base↔cell mpsc is still absent: wanted or not? |
| 205 | CLOSE (completed) | P3 | Every listed file now has tests or is Codecov-ignored by decision. The chain-replay harness is TESTING.md type 6. |
| 282 | NEEDS-OWNER | P3 | None of the 10 skills landed (no `.claude/skills/`). It depends on the stalled Bible. Recommend closing as not planned. |
| 281 | REWRITE | P3 | Phase 1 shipped (#376, ADR). Phases 1.5-7 are pending with no UDP loop. Player-ability LOS parity is still missing. |
| 264 | NEEDS-OWNER | P3 | Phase 0 is done. Three drafts were never promoted and nothing has been authored since May. Continue or shelve? |
| 262 | NEEDS-OWNER | P3 | The evidence ledger never started. spec-lint, `/re-verify`, the lab, and the docs/agents rules cover parts of it. Recommend closing as not planned. |
| 246 | REWRITE | P3 | Item 1 (objective ids) and item 2 are fixed. The loader `as i32` narrowings and four cosmetic nits remain. |
| 213 | NEEDS-OWNER | P3 | The high-pain tables are ported. A "client data files canonical plus conformance tests" pattern emerged but was never adopted. Decide the rule. |
| 167 | REWRITE | P3 | Tier 1 account isolation, auth, and outbox drain are done. Character-create collision, `remove_instance`/`remove_by_type`, `item_allows_container`, and `known_stargates` guards remain. |
| 166 | CLOSE (not planned) | P3 | The outbox cleanup pattern is fixed and guarded. The hard-coded seed ids were self-rated low priority and are covered by the TESTING.md gotcha. |
| 76 | CLOSE (completed) | P3 | Legacy archived under `deprecated/`. Every confirmed Rust bug and security item is fixed or tracked (#460). |

### Cross-batch observations

- **#264, #262, #282 should get one owner decision.** They are the Bible program. If it is shelved, close all three. #295 and #318 (Mercury chapter corrections) ride on the same answer.
- **The sentinel collision in #458 is the only concrete latent bug in this batch.** `inventory/core/resync_tests.rs:26` and `mail/tests.rs:17` both use `0x7000_0500`. It deserves a standalone small `ready-for-agent` ticket.
- **The `method_idx` ↔ `.def` conformance test is asked for in both #458 (G12) and #213.** Track it in one place, ideally a new ticket that both link to.
- **admin-api route tests** appear in #416 Phase 2 and #458 G23. The #416 rewrite defers them to #458.
- **Player `useAbility` has no server-side LOS gate** (#281 phase 5). The security-audit batch (#459/#460) should check whether it is already listed there.
- **`crates/services/src/base/helpers/mod.rs`** (854 lines) is both the largest production file over the cap and a CLAUDE.md naming-rule violation ("avoid `helpers.rs`").
- **`crates/game/src/missions/{rewards,manager}.rs` contain `todo!()` panics in dead code** that nothing calls. Worth a cleanup, or at least a note that mission rewards live in the content executor.
