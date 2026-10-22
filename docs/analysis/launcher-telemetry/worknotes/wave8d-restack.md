# Wave 8d restack: light-adoption onto main

Date: 2026-10-22. Branch `launcher-telemetry/light-adoption` (worktree `lt-light-integration`).

## Restack

- Fetched origin. PR L #1981 squash-merged as `b41f8681b`; PR I2 #1982 as `f2cde8244`; `origin/main` = `1928182c6`.
- `origin/launcher-telemetry/launcher` was still `e6f16395a` (last recorded L_TIP), an ancestor of the old tip `89c7c77cd`.
- `git rebase --onto origin/main e6f16395a`: 47 commits replayed, **no conflicts**. The merge commits were linearised (plain rebase drops merges).
- Check: `git diff 89c7c77cd ebf8a46f3` equals `git diff e6f16395a origin/main` (15 files, +102/-51), so the restack carried only main's changes (incl. LA-05). No conflict markers.

## Verification on ebf8a46f3 (all foreground, lane)

| Check | Result |
|---|---|
| `cargo fmt --all -- --check`, root and `crates/launcher/desktop` | pass |
| clippy `-D warnings`, all targets: launcher-core, patchset | pass |
| clippy `-D warnings`, all targets: launcher-engine, launcher-desktop (shell) | pass |
| nextest launcher-core + patchset | 448 passed, 10 skipped |
| `cargo test` engine + shell | 977 passed, 0 failed, 10 ignored |
| frontend `npm ci --ignore-scripts`, `npm run build`, `npm run check`, `npm test` | pass (227 tests) |
| `npm run uat:adoption-light` | pass (5 scenarios) |

No style commit was needed.

## Push and retirement

- Pushed with `--force-with-lease` (lease `89c7c77cd`): `89c7c77cd...ebf8a46f3`.
- Retired worktrees `la-05` and `lt-app-integration`: `external` junction unlinked non-recursively, `git worktree remove --force`, `B:\targets\<wt>` deleted. Branches kept. Nothing else deleted.

## Result

- Tip `ebf8a46f3fbd2f3f052da18a7686c64bc8fe4d8b`; `git rev-list --count origin/main..HEAD` = 47.
- `git diff --stat origin/main...HEAD`: 124 files changed, 9409 insertions(+), 345 deletions(-).