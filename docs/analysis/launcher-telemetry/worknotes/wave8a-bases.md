# Wave 8a bases (recorded 2026-10-21)

Recorded after `git fetch origin` in the main checkout, per [work-packets-light-adoption.md](../work-packets-light-adoption.md) § "Starting wave 8a while PR L #1981 is open".

- `L_TIP` = `e6f16395ac83e5284fb6f1b839f1202fe79f6866` (`origin/launcher-telemetry/launcher`; unchanged from the packet anchor `e6f16395a`)
- `MAIN_TIP` = `e7487da3e7d6b9711b367365c74e4ba260881017` (`origin/main`)

## Overlap check (step 2)

`git diff --stat <MAIN_TIP> <L_TIP> -- crates/launcher-core/src/file_index/check.rs docs/client/launcher-distribution-setup.md` is empty: PR I2 (LA-05) and PR L cannot conflict on those files.

## Worktrees

| Worktree | Branch | Base |
|---|---|---|
| `lt-light-integration` | `launcher-telemetry/light-adoption` (pushed, upstream set) | `L_TIP` |
| `la-01` | `launcher-telemetry/la-01-patch-summary` | `L_TIP` |
| `la-02` | `launcher-telemetry/la-02-tolerant-apply` | `L_TIP` |
| `la-06` | `launcher-telemetry/la-06-index-compare` | `L_TIP` |
| `la-07` | `launcher-telemetry/la-07-remove-adopt` | `L_TIP` |
| `la-08` | `launcher-telemetry/la-08-stage-seed` | `L_TIP` |
| `la-05` | `launcher-telemetry/la-05-index-blob-form` | `MAIN_TIP` |

Each worktree has `external\` junctioned from the main checkout.
