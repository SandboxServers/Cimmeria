# Wave 8b2 merge — light-adoption integration

Date: 2026-10-21. Integration worktree `.claude/worktrees/lt-light-integration`,
branch `launcher-telemetry/light-adoption`, starting tip `f8697d630`.

## Merges (`git merge --no-ff`, in order)

| Packet | Branch | Branch tip | Merge commit |
|---|---|---|---|
| LA-04 | launcher-telemetry/la-04-tolerant-install | 1731d1526 | d0a661101 |
| LA-10 | launcher-telemetry/la-10-light-replay | 68521fca4 | 15537a17c |

No conflicts; nothing was resolved by hand.

**New tip: `15537a17c351b243f149641bbcd28d8c4aa79eb2`**, pushed to origin.

## Verification on the merged tip (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok (`--manifest-path crates/launcher-core/Cargo.toml`) | ok | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`) | cargo test 790 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`) | nextest 142 passed, 7 skipped |

Deltas against wave 8b1: launcher-core +6 (LA-04), engine +13 (LA-10).
The shell clippy used the existing `crates/launcher/desktop/frontend/dist`; nothing from
dist, node_modules or launcher_targets.txt was staged (the worktree status is clean).
Note: launcher-core is not a member of the root workspace (`-p launcher-core` is refused);
use its manifest path.

Lane logs are in `%LOCALAPPDATA%\cimmeria-build\logs\lt-light-integration\`
(runs 20261021-224804 through 20261021-225057; the first, 224804, was a refused
`-p launcher-core` fmt call, not a format failure).

## Worktree housekeeping

- Retired: la-04, la-10. The external junction was unlinked non-recursively, then
  `git worktree remove --force` and `B:\targets\<wt>` deleted. The branches were kept,
  and main `external\` is intact.
- Created from `15537a17c`, with the external junction and a warm target seeded on
  B:\targets from lt-light-integration:
  - `.claude/worktrees/la-11`, branch `launcher-telemetry/la-11-light-mode`
