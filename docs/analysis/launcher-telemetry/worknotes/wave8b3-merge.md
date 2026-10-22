# Wave 8b3 merge — light-adoption integration

Date: 2026-10-21. Integration worktree `.claude/worktrees/lt-light-integration`,
branch `launcher-telemetry/light-adoption`, starting tip `15537a17c`.

## Merge (`git merge --no-ff`)

| Packet | Branch | Branch tip | Merge commit |
|---|---|---|---|
| LA-11 | launcher-telemetry/la-11-light-mode | c19c2f714 | 571e3f932 |

The la-11 worktree HEAD was verified on `launcher-telemetry/la-11-light-mode` at
`c19c2f714`. No conflicts; nothing was resolved by hand.

**New tip: `571e3f9320d3a6b3850c79525571bbff62dbe7ec`**, pushed to origin.

## Verification on the merged tip (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok (`--manifest-path crates/launcher-core/Cargo.toml`) | ok | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`) | cargo test 802 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`) | nextest 142 passed, 7 skipped |
| frontend | `npm run check` ok | — | `npm test` 198 passed, 0 failed |

Deltas against wave 8b2: engine +12 (LA-11); others unchanged.
The shell clippy ran after `npm ci --ignore-scripts && npm run build` in
`crates/launcher/desktop/frontend`; nothing from dist, node_modules or
launcher_targets.txt was staged (the worktree status is clean).

Lane logs are in `%LOCALAPPDATA%\cimmeria-build\logs\lt-light-integration\`
(runs 20261021-233915 through 20261021-234153).

## Worktree housekeeping

- Retired: la-11. The external junction was unlinked non-recursively, then
  `git worktree remove --force` and `B:\targets\la-11` deleted. The branch was kept,
  and main `external\` is intact.
- Created from `571e3f932`, with the external junction and a warm target seeded on
  B:\targets from lt-light-integration:
  - `.claude/worktrees/la-11b`, branch `launcher-telemetry/la-11b-cooked-tier`