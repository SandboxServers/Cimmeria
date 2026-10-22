# Wave 8c2 merge (LA-13)

Integration worktree `.claude/worktrees/lt-light-integration`, branch
`launcher-telemetry/light-adoption`. Start tip `d273e29b1` (verified, tree clean).

## Merge (`--no-ff`, no conflicts; `adoption/mod.rs` auto-merged)

| Packet | Branch tip | Merge commit |
|---|---|---|
| LA-13 seedless incremental update | `dbef92a7c` (launcher-telemetry/la-13-incremental-update) | `678a30dba` |

Docs commit `8b0a9b27e`: `worknotes/wave8c2-review.md` and `worknotes/wave8c1-merge.md`
(new, CRLF), copied from the main checkout. `work-packets-light-adoption.md` is
byte-identical to the committed copy, so it was not touched.

## First verification pass: blocked on fmt

`cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check` failed on
two LA-13 files (lane log `20261022-003845-105524.log`); the root-workspace check passed.

## Fix: `f8b23c890` style: fmt LA-13 files

`cargo fmt` (desktop workspace, through the lane) applied directly on the integration
branch on top of `8b0a9b27e`. The diff is layout only:

- `engine/src/storage/adoption/light_update_tests.rs:298` - one `assert_eq!` wrapped over
  four lines (non-whitespace content identical).
- `engine/src/storage/update/incremental.rs:102` - `work.join(...)` joined onto one line;
  rustfmt also dropped the trailing comma of the former multi-line argument list.

The `la-13` branch itself does not carry the fmt commit (only the integration branch).

## Verification on `f8b23c890` (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok (root `--all`) | ok (`--manifest-path crates/launcher-core/Cargo.toml`) | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`) | cargo test 830 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`) | nextest 142 passed, 7 skipped |
| frontend | `npm run check` ok | - | `npm test` 201 passed, 0 failed |

Shell clippy ran after `npm ci --ignore-scripts && npm run build`; dist, node_modules
and launcher_targets.txt were not staged (tree clean after the runs). Engine tests went
from 815 (wave 8c1) to 830 with LA-13.

**New tip: `f8b23c890`**, pushed to origin (`d273e29b1..f8b23c890`).

## Retired

`la-13`: external junction unlinked non-recursively (main `external/` intact), `git
worktree remove --force`, `B:\targets\la-13` deleted. Branch
`launcher-telemetry/la-13-incremental-update` kept.
