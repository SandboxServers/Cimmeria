# Wave 8b close — light-adoption integration

Date: 2026-10-22. Integration worktree `.claude/worktrees/lt-light-integration`,
branch `launcher-telemetry/light-adoption`, starting tip `571e3f932`.

## Merge (`git merge --no-ff`)

| Packet | Branch | Branch tip | Merge commit |
|---|---|---|---|
| LA-11b | launcher-telemetry/la-11b-cooked-tier | b9c5f1cef | ccc5ac1f0 |

The la-11b worktree HEAD was verified on `launcher-telemetry/la-11b-cooked-tier` at
`b9c5f1cef`. No conflicts; nothing was resolved by hand.

## Ledger docs commit

`7aad7719a` adds the ledger and worknotes from the main checkout as one docs commit,
copied byte for byte (line endings unchanged; `wave8b-setup.md` and `wave8b2-merge.md`
are LF in the source). All nine files were new on the integration branch:
`work-packets-light-adoption.md`, `worknotes/wave8b-setup.md`,
`worknotes/wave8b{1,2,3}-{merge,review}.md` and `worknotes/wave8b4-review.md`.
No other docs were touched. This note itself is not committed.

**New tip: `7aad7719af086355f05a7978255a05b830478209`**, pushed to origin.

## Verification on the merged tip (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok | ok | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`) | cargo test 809 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`) | nextest 142 passed, 7 skipped |
| frontend | `npm run check` ok | — | `npm test` 198 passed, 0 failed |

Deltas against wave 8b3: engine +7 (LA-11b); others unchanged.
The shell clippy ran after `npm ci --ignore-scripts && npm run build` in
`crates/launcher/desktop/frontend`; nothing from dist, node_modules or
launcher_targets.txt was staged (the worktree status is clean).

Lane logs are in `%LOCALAPPDATA%\cimmeria-build\logs\lt-light-integration\`
(runs 20261021-235739 through 20261021-235932).

## Worktree housekeeping

- Retired: la-11b. The external junction was unlinked non-recursively, then
  `git worktree remove --force` and `B:\targets\la-11b` deleted. The branch was kept,
  and main `external\` is intact.
- Created from `7aad7719a`, with the external junction and a warm target seeded on B:\targets:
  - `.claude/worktrees/la-13`, branch `launcher-telemetry/la-13-incremental-update`
  - `.claude/worktrees/la-14`, branch `launcher-telemetry/la-14-shell-modes`
  - `.claude/worktrees/la-17`, branch `launcher-telemetry/la-17-launch-tag`
