# Test inventory

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests catalogued**: 1071  
> **Companion docs**: [TESTING.md](../../../TESTING.md) (the playbook for *how to write* tests), [maintenance.md](maintenance.md), [review-report.md](review-report.md) (audit findings — owned by the testing-validation-engineer agent)

Catalogue of every test in the workspace. The playbook for *how to write* tests is [TESTING.md](../../../TESTING.md); this directory is the reference complement — what tests already exist, where they live, and what each one asserts.

## Totals

### By crate

| Crate | Tests | File |
|---|---:|---|
| `services` | 550 | [services.md](services.md) |
| `entity` | 151 | [entity.md](entity.md) |
| `mercury` | 97 | [mercury.md](mercury.md) |
| `game` | 70 | [game.md](game.md) |
| `content-engine` | 63 | [content-engine.md](content-engine.md) |
| `common` | 31 | [common.md](common.md) |
| `commands` | 29 | [commands.md](commands.md) |
| `launcher` | 22 | [launcher.md](launcher.md) |
| `tools/SGWLauncher` | 22 | [tools-sgwlauncher.md](tools-sgwlauncher.md) |
| `tools/ContentEditor` | 12 | [tools-contenteditor.md](tools-contenteditor.md) |
| `upk-objects` | 11 | [upk-objects.md](upk-objects.md) |
| `tauri-app` | 6 | [tauri-app.md](tauri-app.md) |
| `defs` | 5 | [defs.md](defs.md) |
| `server` | 2 | [server.md](server.md) |
| **Total** | **1071** | |

### By kind

| Kind | Tests |
|---|---:|
| unit | 903 |
| wire-format | 46 |
| live-DB | 110 |
| smoke | 6 |
| proptest | 4 |
| integration | 2 |

### By first-commit year

| Year | Tests |
|---|---:|
| 2026 | 1071 |

## Reading guide

Each per-crate file groups tests in a single GFM table (or one table per subsystem in `services.md`). Columns:

- **Test** — markdown link to `fn_name` at `file:line` in source.
- **Kind** — one of `unit` / `wire-format` / `live-DB` / `smoke` / `concurrency` / `chain-replay` / `legacy-reference` / `proptest` / `rstest` / `integration`. The first seven are the taxonomy from [TESTING.md](../../../TESTING.md); `proptest` / `rstest` / `integration` are modern additions present in the codebase.
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
