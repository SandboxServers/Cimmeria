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
`tools/extract_tests.py`, and the `regen-docs` workflow reruns it on `main` every
six hours, so don't edit it by hand or bump it in a PR. "Gated in CI" leaves out
the crates on CI's exclude list (`WORKSPACE_EXCLUDES` in
[test.yml](../../../.github/workflows/test.yml)). The last row is the threshold in
[maintenance.md](maintenance.md#when-to-update): a PR that adds or removes that
many tests also updates the catalogue.

<!-- gen:tests-totals -->

| Metric | Count |
|---|---:|
| Tests (`#[test]` / `#[tokio::test]`) | 15,227 |
| Files with tests | 2,503 |
| Gated in CI (every crate but CI's exclude list) | 12,720 |
| Live-DB tests (`require_db_or_skip!` in the body) | 2,238 |
| Inventory threshold (5% of the tests) | 761 |

<!-- /gen:tests-totals -->

### By crate

*Snapshot as of 2026-06-12 except where noted. The `Tests` column is what the
catalogue files contain, not what the crate has today — see the drift warning
above. Live per-crate counts are in [Live counts by crate](#live-counts-by-crate).*

| Crate | Tests | File |
|---|---:|---|
| `services` | 773 | [services.md](services.md) |
| `entity` | 160 | [entity.md](entity.md) |
| `mercury` | 123 | [mercury.md](mercury.md) |
| `content-engine` | 85 | [content-engine.md](content-engine.md) |
| `game` | 70 | [game.md](game.md) |
| `common` | 31 | [common.md](common.md) |
| `commands` | 29 | [commands.md](commands.md) |
| `wireclient` | 30 | [wireclient.md](wireclient.md) — catalogued 2026-07-25 |
| `tools/ContentEditor` | 12 | [tools-contenteditor.md](tools-contenteditor.md) |
| `upk-objects` | 11 | [upk-objects.md](upk-objects.md) |
| `tauri-app` | 6 | [tauri-app.md](tauri-app.md) |
| `defs` | 5 | [defs.md](defs.md) |
| `server` | 2 | [server.md](server.md) |
| **Total** | **1337** | |

> **Launcher rows removed, 2026-10-11.** The egui launcher crate
> (`sgw-launcher`) and its 176-row catalogue, `launcher.md`, were deleted when
> the launcher consolidation retired that launcher (LX-18, LX-23), so the total
> dropped from 1,513 to 1,337. The Tauri tree (`tools/SGWLauncher`) and its 22
> tests had gone in LX-19 (2026-10-10). Most of the egui crate's tests moved
> with their code into `cimmeria-launcher-core` (LX-01); that crate, and the
> desktop launcher's own workspace under `crates/launcher/desktop/`, are not
> catalogued yet.

### Live counts by crate

Generated from `tools/extract_tests.py`, like the workspace totals above. `In CI`
is `no` for the crates on CI's exclude list; `Catalogue` is `none` for crates
with no file in this directory yet.

<!-- gen:tests-by-crate -->

| Crate | Package | Tests | Files | Live-DB | In CI | Catalogue |
|---|---|---:|---:|---:|---|---|
| `crates/lab` | `cimmeria-lab` | 1,613 | 226 | 0 | no | none |
| `crates/cell-content` | `cimmeria-cell-content` | 1,352 | 212 | 932 | yes | none |
| `crates/lab-cli` | `cimmeria-lab-cli` | 1,202 | 108 | 0 | yes | none |
| `crates/cell-combat` | `cimmeria-cell-combat` | 936 | 170 | 48 | yes | none |
| `crates/client-telemetry` | `cimmeria-client-telemetry` | 776 | 144 | 0 | no | none |
| `crates/cell` | `cimmeria-cell` | 683 | 162 | 27 | yes | none |
| `crates/cell-world` | `cimmeria-cell-world` | 674 | 132 | 90 | yes | none |
| `crates/base-methods` | `cimmeria-base-methods` | 665 | 145 | 466 | yes | none |
| `crates/cell-console` | `cimmeria-cell-console` | 570 | 94 | 1 | yes | none |
| `crates/base-session` | `cimmeria-base-session` | 521 | 90 | 197 | yes | none |
| `crates/entity` | `cimmeria-entity` | 463 | 62 | 0 | yes | [entity.md](entity.md) |
| `crates/navmesh-extractor` | `cimmeria-navmesh-extractor` | 449 | 52 | 0 | yes | none |
| `crates/mercury` | `cimmeria-mercury` | 366 | 61 | 0 | yes | [mercury.md](mercury.md) |
| `crates/launcher-core` | `cimmeria-launcher-core` | 334 | 42 | 0 | yes | none |
| `crates/wire` | `cimmeria-wire` | 325 | 60 | 0 | yes | none |
| `crates/base-crafting` | `cimmeria-base-crafting` | 323 | 59 | 142 | yes | none |
| `crates/content-engine` | `cimmeria-content-engine` | 316 | 33 | 1 | yes | [content-engine.md](content-engine.md) |
| `crates/cell-methods` | `cimmeria-cell-methods` | 315 | 50 | 9 | yes | none |
| `crates/base` | `cimmeria-base` | 284 | 59 | 20 | yes | none |
| `crates/base-world-entry` | `cimmeria-base-world-entry` | 278 | 68 | 55 | yes | none |
| `crates/admin-api` | `cimmeria-admin-api` | 275 | 39 | 0 | yes | none |
| `crates/cell-catalog` | `cimmeria-cell-catalog` | 235 | 60 | 141 | yes | none |
| `crates/cell-interactions` | `cimmeria-cell-interactions` | 220 | 38 | 1 | yes | none |
| `crates/resources` | `cimmeria-resources` | 183 | 27 | 3 | yes | none |
| `crates/wireclient` | `cimmeria-wireclient` | 166 | 34 | 19 | yes | [wireclient.md](wireclient.md) |
| `crates/cell-effect-scripts` | `cimmeria-cell-effect-scripts` | 162 | 19 | 21 | yes | none |
| `crates/server` | `cimmeria-server` | 128 | 24 | 0 | yes | [server.md](server.md) |
| `crates/discord` | `cimmeria-discord` | 116 | 24 | 0 | yes | none |
| `crates/client-patches` | `cimmeria-client-patches` | 99 | 16 | 0 | no | none |
| `crates/auth` | `cimmeria-auth` | 96 | 17 | 11 | yes | none |
| `crates/lab-mcp` | `cimmeria-lab-mcp` | 90 | 17 | 10 | yes | none |
| `crates/minigame` | `cimmeria-minigame` | 84 | 12 | 0 | yes | none |
| `crates/patchset` | `cimmeria-patchset` | 78 | 14 | 0 | yes | none |
| `crates/upk-objects` | `cimmeria-upk-objects` | 78 | 9 | 0 | yes | [upk-objects.md](upk-objects.md) |
| `crates/cell-org` | `cimmeria-cell-org` | 74 | 8 | 0 | yes | none |
| `crates/cell-cover` | `cimmeria-cell-cover` | 71 | 5 | 6 | yes | none |
| `crates/services` | `cimmeria-services` | 64 | 20 | 26 | yes | [services.md](services.md) |
| `crates/cell-duel` | `cimmeria-cell-duel` | 53 | 12 | 0 | yes | none |
| `crates/cell-pets` | `cimmeria-cell-pets` | 53 | 7 | 0 | yes | none |
| `crates/test-support` | `cimmeria-test-support` | 52 | 7 | 8 | yes | none |
| `crates/game` | `cimmeria-game` | 48 | 12 | 0 | yes | [game.md](game.md) |
| `crates/patch-wire` | `cimmeria-patch-wire` | 47 | 6 | 0 | yes | none |
| `crates/common` | `cimmeria-common` | 44 | 5 | 0 | yes | [common.md](common.md) |
| `crates/client-launch` | `cimmeria-client-launch` | 41 | 4 | 0 | yes | none |
| `crates/upk` | `cimmeria-upk` | 37 | 5 | 0 | yes | none |
| `crates/occluder` | `cimmeria-occluder` | 30 | 3 | 0 | yes | none |
| `crates/commands` | `cimmeria-commands` | 29 | 3 | 0 | yes | [commands.md](commands.md) |
| `crates/wire-log` | `cimmeria-wire-log` | 28 | 6 | 0 | yes | none |
| `crates/names` | `cimmeria-names` | 24 | 6 | 4 | yes | none |
| `crates/client-hookgate` | `cimmeria-client-hookgate` | 20 | 2 | 0 | yes | none |
| `tools/ContentEditor` | `cimmeria-content-editor` | 13 | 2 | 0 | no | [tools-contenteditor.md](tools-contenteditor.md) |
| `crates/sgw-testhost` | `cimmeria-sgw-testhost` | 12 | 2 | 0 | yes | none |
| `crates/observability` | `cimmeria-observability` | 11 | 3 | 0 | yes | none |
| `crates/cell-chatter` | `cimmeria-cell-chatter` | 10 | 3 | 0 | yes | none |
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
