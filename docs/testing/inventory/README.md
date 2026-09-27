# Test inventory

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-09-27 (header figures and live counts re-counted with `tools/extract_tests.py`; catalogue tables are still the 2026-06-12 snapshot, plus `wireclient.md`'s social-systems rows)  
> **Total tests catalogued**: 1,351 *(stale snapshot; current workspace count is **7,337 tests across 1,192 files** (2026-09-27, `python tools/extract_tests.py`) — inventory regeneration is pending the next sweep)*  
> **Companion docs**: [TESTING.md](../../../TESTING.md) (the playbook for *how to write* tests), [maintenance.md](maintenance.md), [review-report.md](review-report.md) (audit findings — owned by the testing-validation-engineer agent)

> **Catalogue drift warning.** The per-crate tables below cover 1,351 tests
> against a workspace that now has 7,337 — they are missing more than half
> the suite, and several crates added since the snapshot have no file at all
> (`admin-api`, `discord`, `navmesh-extractor`, `observability`,
> `client-telemetry`, and every `base-*` and `cell-*` crate from the services
> split). `wireclient` was catalogued separately on 2026-07-25 —
> see [wireclient.md](wireclient.md). Treat this directory as a partial index
> until the next regeneration sweep; use it to look tests up, not to reason
> about coverage totals.

Catalogue of every test in the workspace. The playbook for *how to write* tests is [TESTING.md](../../../TESTING.md); this directory is the reference complement — what tests already exist, where they live, and what each one asserts.

## Totals

### By crate

*Snapshot as of 2026-06-12 except where noted. The `Tests` column is what the
catalogue files contain, not what the crate has today — see the drift warning
above. Live per-crate counts as of 2026-09-27 are in the second table.*

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

### Live counts (2026-09-27)

Counted with `python tools/extract_tests.py --list-crates` over every workspace
member. The services split moved most tests out of `cimmeria-services` into
the `base-*` and `cell-*` crates, none of which has a catalogue file yet (✗).

| Crate | Tests | Files | Live-DB | In CI | Catalogued? |
|---|---:|---:|---:|---|---|
| `cimmeria-cell-content` | 763 | 110 | 460 | yes | ✗ |
| `cimmeria-cell-combat` | 532 | 84 | 38 | yes | ✗ |
| `cimmeria-cell-world` | 492 | 82 | 29 | yes | ✗ |
| `cimmeria-cell` | 477 | 99 | 17 | yes | ✗ |
| `cimmeria-base-session` | 465 | 81 | 184 | yes | ✗ |
| `cimmeria-navmesh-extractor` | 449 | 52 | 0 | yes | ✗ |
| `cimmeria-entity` | 363 | 43 | 0 | yes | ✓ [entity.md](entity.md) |
| `cimmeria-cell-console` | 362 | 54 | 0 | yes | ✗ |
| `cimmeria-base-methods` | 353 | 73 | 265 | yes | ✗ |
| `cimmeria-cell-methods` | 339 | 50 | 5 | yes | ✗ |
| `cimmeria-mercury` | 287 | 48 | 0 | yes | ✓ [mercury.md](mercury.md) |
| `cimmeria-wire` | 259 | 41 | 0 | yes | ✗ |
| `cimmeria-content-engine` | 258 | 23 | 0 | yes | ✓ [content-engine.md](content-engine.md) |
| `cimmeria-cell-interactions` | 154 | 27 | 0 | yes | ✗ |
| `sgw-launcher` | 150 | 19 | 0 | no | ✓ [launcher.md](launcher.md) |
| `cimmeria-base` | 146 | 24 | 9 | yes | ✗ |
| `cimmeria-base-world-entry` | 141 | 39 | 29 | yes | ✗ |
| `cimmeria-cell-catalog` | 138 | 31 | 71 | yes | ✗ |
| `cimmeria-client-telemetry` | 134 | 30 | 0 | no | ✗ |
| `cimmeria-resources` | 122 | 19 | 0 | yes | ✗ |
| `cimmeria-discord` | 76 | 15 | 0 | yes | ✗ |
| `cimmeria-upk-objects` | 76 | 8 | 0 | yes | ✓ [upk-objects.md](upk-objects.md) |
| `cimmeria-cell-cover` | 71 | 5 | 6 | yes | ✗ |
| `cimmeria-game` | 64 | 17 | 0 | yes | ✓ [game.md](game.md) |
| `cimmeria-server` | 60 | 12 | 0 | yes | ✓ [server.md](server.md) |
| `cimmeria-wireclient` | 60 | 15 | 0 | yes | ✓ [wireclient.md](wireclient.md) |
| `cimmeria-client-patches` | 59 | 11 | 0 | no | ✗ |
| `cimmeria-lab` | 57 | 14 | 0 | no | ✗ |
| `cimmeria-admin-api` | 52 | 4 | 0 | yes | ✗ |
| `cimmeria-auth` | 52 | 10 | 8 | yes | ✗ |
| `cimmeria-patch-wire` | 45 | 5 | 0 | yes | ✗ |
| `cimmeria-minigame` | 37 | 5 | 0 | yes | ✗ |
| `cimmeria-common` | 36 | 4 | 0 | yes | ✓ [common.md](common.md) |
| `cimmeria-occluder` | 30 | 3 | 0 | yes | ✗ |
| `cimmeria-services` | 30 | 9 | 12 | yes | ✓ [services.md](services.md) |
| `cimmeria-commands` | 29 | 3 | 0 | yes | ✓ [commands.md](commands.md) |
| `cimmeria-client-launch` | 26 | 3 | 0 | yes | ✗ |
| `cimmeria-wire-log` | 24 | 6 | 0 | yes | ✗ |
| `cimmeria-test-support` | 19 | 3 | 0 | yes | ✗ |
| `cimmeria-content-editor` | 12 | 1 | 0 | no | ✓ [tools-contenteditor.md](tools-contenteditor.md) |
| `cimmeria-upk` | 12 | 3 | 0 | yes | ✗ |
| `cimmeria-observability` | 8 | 2 | 0 | yes | ✗ |
| `cimmeria-lab-mcp` | 7 | 2 | 0 | yes | ✗ |
| `cimmeria-app` | 6 | 2 | 0 | no | ✓ [tauri-app.md](tauri-app.md) |
| `cimmeria-defs` | 5 | 1 | 0 | yes | ✓ [defs.md](defs.md) |
| **Total** | **7,337** | **1,192** | **1,133** | | |

Of these, **6,919** are gated on every PR. CI excludes the crates marked "no":

| Excluded crate | Tests | Note |
|---|---:|---|
| `sgw-launcher` | 150 | The egui launcher. Includes ed25519 manifest-signature verification, a path-traversal guard and hostname-injection validation — see [launcher.md](launcher.md#ci-exclusion). |
| `cimmeria-client-telemetry` | 134 | Windows-only cdylib; excluded so Linux dev hosts need no extra toolchain. |
| `cimmeria-client-patches` | 59 | Windows-only injected DLL (Black Market client patch). |
| `cimmeria-lab` | 57 | Live research lab supervisor (GUI / Windows host). |
| `cimmeria-content-editor` | 12 | GUI app. |
| `cimmeria-app` | 6 | GUI app (`src-tauri`). |
| **Total** | **418** | |

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

See [maintenance.md](maintenance.md) — when you add or remove a test, you also update the relevant per-crate file and the totals on this page in the same PR. CI does not yet drift-check the inventory; reviewers do.
