# Test inventory

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25 (header figures re-counted; catalogue tables are still the 2026-06-12 snapshot)  
> **Total tests catalogued**: 1,351 *(stale snapshot; the current workspace count is in [Workspace totals](#workspace-totals). Inventory regeneration is pending the next sweep)*  
> **Companion docs**: [TESTING.md](../../../TESTING.md) (the playbook for *how to write* tests), [maintenance.md](maintenance.md), [review-report.md](review-report.md) (audit findings — owned by the testing-validation-engineer agent)

> **Catalogue drift warning.** The per-crate tables below cover 1,351 tests,
> far fewer than the [workspace total](#workspace-totals). They are missing most of
> the suite, and several crates added since the snapshot have no file at all
> (`admin-api`, `discord`, `navmesh-extractor`, `observability`,
> `client-telemetry`, and every `base-*` and `cell-*` crate from the services
> split). `wireclient` was catalogued separately on 2026-07-25 —
> see [wireclient.md](wireclient.md). Treat this directory as a partial index
> until the next regeneration sweep; use it to look tests up, not to reason
> about coverage totals.

Catalogue of every test in the workspace. The playbook for *how to write* tests is [TESTING.md](../../../TESTING.md); this directory is the reference complement — what tests already exist, where they live, and what each one asserts.

## Totals

### Workspace totals

The canonical workspace test count. Other docs link here rather than repeat the
numbers. `tools/docs-gen/regen.py` generates this block from
`tools/extract_tests.py`, and the `regen-docs` workflow reruns it on `main` after
every merge, so don't edit it by hand or bump it in a PR. "Gated in CI" leaves out
the crates on CI's exclude list (`WORKSPACE_EXCLUDES` in
[test.yml](../../../.github/workflows/test.yml)). The last row is the threshold in
[maintenance.md](maintenance.md#when-to-update): a PR that adds or removes that
many tests also updates the catalogue.

<!-- gen:tests-totals -->

| Metric | Count |
|---|---:|
| Tests (`#[test]` / `#[tokio::test]`) | 10,782 |
| Files with tests | 1,864 |
| Gated in CI (every crate but CI's exclude list) | 9,157 |
| Live-DB tests (`require_db_or_skip!` in the body) | 1,566 |
| Inventory threshold (5% of the tests) | 539 |

<!-- /gen:tests-totals -->

### By crate

*Snapshot as of 2026-06-12 except where noted. The `Tests` column is what the
catalogue files contain, not what the crate has today — see the drift warning
above. Live per-crate counts are in [Live counts by crate](#live-counts-by-crate).*

| Crate | Tests | File |
|---|---:|---|
| `services` | 773 | [services.md](services.md) |
| `launcher` (`sgw-launcher`) | 176 | [launcher.md](launcher.md) — recatalogued 2026-07-25 |
| `entity` | 160 | [entity.md](entity.md) |
| `mercury` | 123 | [mercury.md](mercury.md) |
| `content-engine` | 85 | [content-engine.md](content-engine.md) |
| `game` | 70 | [game.md](game.md) |
| `common` | 31 | [common.md](common.md) |
| `commands` | 29 | [commands.md](commands.md) |
| `wireclient` | 30 | [wireclient.md](wireclient.md) — catalogued 2026-07-25 |
| `tools/SGWLauncher` | 22 | [tools-sgwlauncher.md](tools-sgwlauncher.md) |
| `tools/ContentEditor` | 12 | [tools-contenteditor.md](tools-contenteditor.md) |
| `upk-objects` | 11 | [upk-objects.md](upk-objects.md) |
| `tauri-app` | 6 | [tauri-app.md](tauri-app.md) |
| `defs` | 5 | [defs.md](defs.md) |
| `server` | 2 | [server.md](server.md) |
| **Total** | **1535** | |

> **Double-count fixed, 2026-07-25.** `launcher.md` used to carry 22 rows that
> were a verbatim duplicate of `tools-sgwlauncher.md` — all 22 describe tests in
> `tools/SGWLauncher/src-tauri/`, filed under `crates/launcher/src/…` paths that
> never contained them. The old total of 1,351 counted those 22 twice. The two
> launchers are genuinely separate crates (`crates/launcher` is the **egui**
> launcher, `sgw-launcher`; `tools/SGWLauncher` is the **Tauri** one), and
> `crates/launcher`'s real 176-test suite was catalogued nowhere. It is now in
> [launcher.md](launcher.md).

### Live counts by crate

Generated from `tools/extract_tests.py`, like the workspace totals above. `In CI`
is `no` for the crates on CI's exclude list; `Catalogue` is `none` for crates
with no file in this directory yet.

<!-- gen:tests-by-crate -->

| Crate | Package | Tests | Files | Live-DB | In CI | Catalogue |
|---|---|---:|---:|---:|---|---|
| `crates/cell-combat` | `cimmeria-cell-combat` | 885 | 157 | 45 | yes | none |
| `crates/cell-content` | `cimmeria-cell-content` | 835 | 123 | 477 | yes | none |
| `crates/client-telemetry` | `cimmeria-client-telemetry` | 649 | 126 | 0 | no | none |
| `crates/base-methods` | `cimmeria-base-methods` | 611 | 133 | 433 | yes | none |
| `crates/cell` | `cimmeria-cell` | 577 | 132 | 26 | yes | none |
| `crates/cell-world` | `cimmeria-cell-world` | 540 | 94 | 32 | yes | none |
| `crates/cell-console` | `cimmeria-cell-console` | 501 | 82 | 1 | yes | none |
| `crates/lab` | `cimmeria-lab` | 469 | 98 | 0 | no | none |
| `crates/entity` | `cimmeria-entity` | 453 | 58 | 0 | yes | [entity.md](entity.md) |
| `crates/navmesh-extractor` | `cimmeria-navmesh-extractor` | 449 | 52 | 0 | yes | none |
| `crates/base-session` | `cimmeria-base-session` | 422 | 77 | 186 | yes | none |
| `crates/launcher` | `sgw-launcher` | 396 | 54 | 0 | no | [launcher.md](launcher.md) |
| `crates/mercury` | `cimmeria-mercury` | 338 | 56 | 0 | yes | [mercury.md](mercury.md) |
| `crates/base-crafting` | `cimmeria-base-crafting` | 321 | 59 | 142 | yes | none |
| `crates/wire` | `cimmeria-wire` | 319 | 58 | 0 | yes | none |
| `crates/cell-methods` | `cimmeria-cell-methods` | 277 | 45 | 8 | yes | none |
| `crates/content-engine` | `cimmeria-content-engine` | 272 | 24 | 0 | yes | [content-engine.md](content-engine.md) |
| `crates/cell-interactions` | `cimmeria-cell-interactions` | 204 | 35 | 0 | yes | none |
| `crates/cell-catalog` | `cimmeria-cell-catalog` | 196 | 50 | 111 | yes | none |
| `crates/base` | `cimmeria-base` | 183 | 33 | 10 | yes | none |
| `crates/base-world-entry` | `cimmeria-base-world-entry` | 181 | 50 | 27 | yes | none |
| `crates/cell-effect-scripts` | `cimmeria-cell-effect-scripts` | 162 | 19 | 21 | yes | none |
| `crates/resources` | `cimmeria-resources` | 138 | 20 | 0 | yes | none |
| `crates/server` | `cimmeria-server` | 113 | 20 | 0 | yes | [server.md](server.md) |
| `crates/discord` | `cimmeria-discord` | 109 | 22 | 0 | yes | none |
| `crates/admin-api` | `cimmeria-admin-api` | 98 | 11 | 0 | yes | none |
| `crates/client-patches` | `cimmeria-client-patches` | 93 | 14 | 0 | no | none |
| `crates/upk-objects` | `cimmeria-upk-objects` | 78 | 9 | 0 | yes | [upk-objects.md](upk-objects.md) |
| `crates/cell-org` | `cimmeria-cell-org` | 74 | 8 | 0 | yes | none |
| `crates/cell-cover` | `cimmeria-cell-cover` | 71 | 5 | 6 | yes | none |
| `crates/wireclient` | `cimmeria-wireclient` | 61 | 16 | 0 | yes | [wireclient.md](wireclient.md) |
| `crates/auth` | `cimmeria-auth` | 55 | 11 | 9 | yes | none |
| `crates/cell-pets` | `cimmeria-cell-pets` | 53 | 7 | 0 | yes | none |
| `crates/test-support` | `cimmeria-test-support` | 52 | 7 | 8 | yes | none |
| `crates/cell-duel` | `cimmeria-cell-duel` | 50 | 11 | 0 | yes | none |
| `crates/game` | `cimmeria-game` | 48 | 12 | 0 | yes | [game.md](game.md) |
| `crates/patch-wire` | `cimmeria-patch-wire` | 47 | 6 | 0 | yes | none |
| `crates/services` | `cimmeria-services` | 45 | 13 | 20 | yes | [services.md](services.md) |
| `crates/common` | `cimmeria-common` | 43 | 5 | 0 | yes | [common.md](common.md) |
| `crates/client-launch` | `cimmeria-client-launch` | 41 | 4 | 0 | yes | none |
| `crates/minigame` | `cimmeria-minigame` | 39 | 5 | 0 | yes | none |
| `crates/occluder` | `cimmeria-occluder` | 30 | 3 | 0 | yes | none |
| `crates/commands` | `cimmeria-commands` | 29 | 3 | 0 | yes | [commands.md](commands.md) |
| `crates/wire-log` | `cimmeria-wire-log` | 28 | 6 | 0 | yes | none |
| `crates/patchset` | `cimmeria-patchset` | 26 | 6 | 0 | yes | none |
| `crates/names` | `cimmeria-names` | 23 | 6 | 4 | yes | none |
| `crates/lab-mcp` | `cimmeria-lab-mcp` | 22 | 6 | 0 | yes | none |
| `crates/client-hookgate` | `cimmeria-client-hookgate` | 20 | 2 | 0 | yes | none |
| `crates/sgw-testhost` | `cimmeria-sgw-testhost` | 12 | 2 | 0 | yes | none |
| `crates/upk` | `cimmeria-upk` | 12 | 3 | 0 | yes | none |
| `tools/ContentEditor` | `cimmeria-content-editor` | 12 | 1 | 0 | no | [tools-contenteditor.md](tools-contenteditor.md) |
| `crates/observability` | `cimmeria-observability` | 9 | 2 | 0 | yes | none |
| `src-tauri` | `cimmeria-app` | 6 | 2 | 0 | no | [tauri-app.md](tauri-app.md) |
| `crates/defs` | `cimmeria-defs` | 5 | 1 | 0 | yes | [defs.md](defs.md) |

<!-- /gen:tests-by-crate -->

The exclusions exist for build-environment reasons (GUI toolkits, Windows-only
cdylibs, linker memory), not because the tests are low-value. Run the excluded
crates locally when you touch them.

### Social-systems campaign (2026-09-27)

The mail, chat and duel campaign ([ledger](../../analysis/social-systems/README.md), PRs #880 to #937) added **392 tests net** (5.3% of the workspace), counted from the `#[test]` / `#[tokio::test]` lines each squash commit added and removed. That is over the 5% threshold in [maintenance.md](maintenance.md), but every crate it touched except `wireclient` has no catalogue file yet, so the rows land with the backfill sweep. The packet worknotes under [`docs/analysis/social-systems/worknotes/`](../../analysis/social-systems/worknotes/) name each test, its type and its revert proof.

| Crate | Net new tests | Catalogued? |
|---|---:|---|
| `cimmeria-base-methods` | 116 | ✗ (mail: send, escrow, take, COD, return, expiry, notification, system mail, GM tools; mostly live-DB) |
| `cimmeria-base` | 63 | ✗ (chat gates, tells, `chatIgnore`, duel challenge, the 0xC6-0xCE feedback arms) |
| `cimmeria-cell-world` | 52 | ✗ (the duel registry, engage and every end path) |
| `cimmeria-cell-console` | 41 | ✗ (`.announce`, `.mute`, `.mail*`, `.duel_*`, the chat split) |
| `cimmeria-base-session` | 38 | ✗ (online index, rate limiter, mutes, Ignore cache, GM broadcast) |
| `cimmeria-wire` | 32 | ✗ (mail and duel serializers, enum pins against `enumerations.xml`) |
| `cimmeria-wireclient` | 13 | ✓ [wireclient.md](wireclient.md) |
| `cimmeria-cell-combat` | 10 | ✗ (the duel harm gate and the non-lethal clamp) |
| `cimmeria-cell` | 7 | ✗ |
| `cimmeria-base-world-entry` | 5 | ✗ |
| `cimmeria-cell-methods` | 4 | ✗ |
| `cimmeria-cell-content` | 4 | ✗ (the Gate Mail Clerk chain replay) |
| `cimmeria-content-engine` | 3 | ✗ (the `send_system_mail` action) |
| `cimmeria-cell-interactions` | 2 | ✗ |
| `cimmeria-cell-catalog` | 2 | ✗ (the clerk's seed and placement guards) |
| **Total** | **392** | |

### By kind

*Snapshot as of 2026-06-12, covering the 1,351 catalogued tests only. The
live-DB figure in particular is stale — the workspace now has 1,133
live-DB tests (`require_db_or_skip!`).*

| Kind | Tests |
|---|---:|
| unit | 1078 |
| wire-format | 77 |
| live-DB | 151 |
| chain-replay | 33 |
| smoke | 6 |
| proptest | 4 |
| integration | 2 |

### By first-commit year

| Year | Tests |
|---|---:|
| 2026 | 1097 |

## Reading guide

Each per-crate file groups tests in a single GFM table (or one table per subsystem in `services.md`). Columns:

- **Test** — markdown link to `fn_name` at `file:line` in source.
- **Kind** — one of `unit` / `wire-format` / `live-DB` / `smoke` / `concurrency` / `chain-replay` / `legacy-reference` / `proptest` / `rstest` / `integration`. The first seven were the taxonomy from [TESTING.md](../../../TESTING.md) at snapshot time; that taxonomy has since grown to 12 types (adding `fan-out byte`, `Mercury session`, `network chaos`, `wire-level replay`, `negative-log`), which the catalogue rows do not yet distinguish. `proptest` / `rstest` / `integration` are extractor-level labels, not TESTING.md types.
- **System / Feature** — derived from module path (e.g. `services::cell::combat::threat` -> `Combat / Threat`).
- **Added** — first-commit date (best-effort, via `git log -S 'fn <name>' -- <file>`).
- **What it tests** — one-sentence summary, prefer the test's `///` doc comment when present, otherwise inferred from the function name and the first assert in the body.
- **Notes** — only present when there's something to flag (`#[ignore]`, smell signals, parameterized via `test_case` / `rstest`).

To find a test:

1. Pick the crate file from the table above.
2. Search the file for the function name (Ctrl/Cmd-F) or the system label (`Combat`, `Vendor`, `Mercury`, `Threat`, …).
3. Click through to source.

## Audit findings

See [review-report.md](review-report.md) for audit findings — that file is owned by the testing-validation-engineer agent and lists tests with smells (`no_assert_or_question_mark`, ignored without reason, low-signal names, etc.) that humans should triage.

## Keeping this inventory current

See [maintenance.md](maintenance.md) — a PR that adds or removes more tests than the threshold in [Workspace totals](#workspace-totals) also updates the relevant per-crate file. The totals and live counts on this page are generated; don't edit them. CI does not drift-check the catalogue tables; reviewers do.
