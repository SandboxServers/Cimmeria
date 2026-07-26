# Test inventory

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25 (header figures re-counted; catalogue tables are still the 2026-06-12 snapshot)  
> **Total tests catalogued**: 1,351 *(stale snapshot; current workspace count is **2,936 tests across 461 files** — inventory regeneration is pending the next sweep)*  
> **Companion docs**: [TESTING.md](../../../TESTING.md) (the playbook for *how to write* tests), [maintenance.md](maintenance.md), [review-report.md](review-report.md) (audit findings — owned by the testing-validation-engineer agent)

> **Catalogue drift warning.** The per-crate tables below cover 1,351 tests
> against a workspace that now has 2,936 — they are missing more than half
> the suite, and several crates added since the snapshot have no file at all
> (`admin-api`, `discord`, `navmesh-extractor`, `observability`,
> `client-telemetry`). `wireclient` was catalogued separately on 2026-07-25 —
> see [wireclient.md](wireclient.md). Treat this directory as a partial index
> until the next regeneration sweep; use it to look tests up, not to reason
> about coverage totals.

Catalogue of every test in the workspace. The playbook for *how to write* tests is [TESTING.md](../../../TESTING.md); this directory is the reference complement — what tests already exist, where they live, and what each one asserts.

## Totals

### By crate

*Snapshot as of 2026-06-12 except where noted. The `Tests` column is what the
catalogue files contain, not what the crate has today — see the drift warning
above. Live per-crate counts as of 2026-07-25 are in the second table.*

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

### Live counts (2026-07-25)

Counted with `grep -rnE "^[[:space:]]*#\[(tokio::)?test(\(.*\))?\]" --include=*.rs`
over every workspace member. Crates marked ✗ have no catalogue file yet.

| Crate | Tests | Files | Catalogued? |
|---|---:|---:|---|
| `services` | 1,761 | 289 | ✓ |
| `mercury` | 260 | 45 | ✓ |
| `entity` | 246 | 23 | ✓ |
| `launcher` (`sgw-launcher`) | 176 | 22 | ✓ [launcher.md](launcher.md) |
| `content-engine` | 143 | 14 | ✓ |
| `discord` | 76 | 15 | ✗ |
| `game` | 69 | 20 | ✓ |
| `navmesh-extractor` | 54 | 10 | ✗ |
| `client-telemetry` | 51 | 11 | ✗ |
| `common` | 35 | 4 | ✓ |
| `commands` | 29 | 3 | ✓ |
| `wireclient` | 30 | 5 | ✓ [wireclient.md](wireclient.md) |
| `admin-api` | 22 | 2 | ✗ |
| `upk-objects` | 21 | 3 | ✓ |
| `tools/ContentEditor` | 12 | 1 | ✓ |
| `server` | 9 | 2 | ✓ |
| `observability` | 7 | 1 | ✗ |
| `src-tauri` (`cimmeria-app`) | 6 | 2 | ✓ |
| `defs` | 5 | 1 | ✓ |
| **Total** | **2,936** | **461** | |

Of these, **2,691** are gated on every PR — CI excludes `cimmeria-app`,
`cimmeria-content-editor`, `cimmeria-scene-editor`, `sgw-launcher`, and
`cimmeria-client-telemetry`. 247 of the gated tests are live-DB guards
(`require_db_or_skip!`), all in `cimmeria-services`.

The 245-test gap between 2,936 and 2,691 breaks down as:

| Excluded crate | Tests | Note |
|---|---:|---|
| `sgw-launcher` (`crates/launcher`) | 176 | **72% of the gap on its own.** Includes ed25519 manifest-signature verification, a path-traversal guard, hostname-injection validation, and two explicit revert-detecting regression guards — see [launcher.md](launcher.md#ci-exclusion). |
| `cimmeria-client-telemetry` | 51 | Windows-only cdylib; excluded so Linux dev hosts need no extra toolchain. |
| `cimmeria-app` (`src-tauri`) | 6 | GUI app. |
| `cimmeria-content-editor`, `cimmeria-scene-editor` | 0 catalogued | GUI apps; excluded for the same linker/OOM reasons as the others. |
| **Total** | **233** | Remainder is counting drift between the grep and nextest's collection. |

The exclusions exist for build-environment reasons (GUI toolkits, a Windows-only
cdylib, linker memory), not because the tests are low-value. Run the excluded
crates locally when you touch them.

### By kind

*Snapshot as of 2026-06-12, covering the 1,351 catalogued tests only. The
live-DB figure in particular is stale — the workspace now has 247
`require_db_or_skip!` guards.*

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
