# Wave 8b1 merge — light-adoption integration

Date: 2026-10-21. Integration worktree `.claude/worktrees/lt-light-integration`,
branch `launcher-telemetry/light-adoption`, starting tip `c4126f495`.

## Merges (`git merge --no-ff`, in order)

| Packet | Branch | Branch tip | Merge commit |
|---|---|---|---|
| LA-03 | launcher-telemetry/la-03-incomplete-ledger | 1874468fa | 2270d2af6 |
| LA-09 | launcher-telemetry/la-09-light-plan | 3557a6b00 | eafa6dafc |
| LA-12 | launcher-telemetry/la-12-index-compare | 0f45d54b9 | f8697d630 |

No conflicts. `adoption/mod.rs` auto-merged on LA-12; nothing was resolved by hand.

**New tip: `f8697d63013871b0710a6639ef96811d0d5497e1`**, pushed to origin.

## Verification on the merged tip (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok | ok | nextest 366 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok | cargo test 777 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok | nextest 142 passed, 7 skipped |

The shell clippy run used `crates/launcher/desktop/frontend/dist`, built with
`npm ci --ignore-scripts && npm run build`; neither dist nor node_modules was staged.
Note: the patchset package is `cimmeria-patchset` (`-p patchset` is refused).

Lane logs are in `%LOCALAPPDATA%\cimmeria-build\logs\lt-light-integration\`
(runs 20261021-221542 through 20261021-222421). They went under `logs\lt-light-integration\`,
so the lane ran from the integration worktree and not from main.

## Worktree housekeeping

- Retired: la-03, la-09, la-12. The external junction was unlinked non-recursively, then
  `git worktree remove --force` and `B:\targets\<wt>` deleted. The branches were kept,
  and main `external\` is intact.
- Created from `f8697d630`, with the external junction and a warm target seeded on B:\targets:
  - `.claude/worktrees/la-04`, branch `launcher-telemetry/la-04-tolerant-install`
  - `.claude/worktrees/la-10`, branch `launcher-telemetry/la-10-light-replay`
