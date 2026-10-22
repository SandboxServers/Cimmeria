# Wave 8c1 merge (LA-14, LA-17)

Integration worktree `.claude/worktrees/lt-light-integration`, branch
`launcher-telemetry/light-adoption`. Start tip `7aad7719a`.

## Merges (`--no-ff`, no conflicts)

| Packet | Branch tip | Merge commit |
|---|---|---|
| LA-14 shell modes | `946a6ff42` (launcher-telemetry/la-14-shell-modes) | Merge LA-14 (shell modes) |
| LA-17 launch tag | `d847c08b6` (launcher-telemetry/la-17-launch-tag) | Merge LA-17 (launch tag) |

LA-13 not merged (still in review).

Docs commit `d273e29b1`: `work-packets-light-adoption.md` (updated),
`worknotes/wave8b-close.md` and `worknotes/wave8c1-review.md` (new), copied from the
main checkout. `wave8b-close.md` is LF in the main checkout; git normalizes it to CRLF.

**New tip: `d273e29b1`**, pushed to origin.

## Verification on the merged tip (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok (workspace `--all`) | ok (`--manifest-path crates/launcher-core/Cargo.toml`) | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`) | cargo test 815 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`) | nextest 142 passed, 7 skipped |
| frontend | `npm run check` ok | — | `npm test` 201 passed, 0 failed |

Shell clippy ran after `npm ci --ignore-scripts && npm run build`; dist, node_modules
and launcher_targets.txt were not staged (tree clean after the runs).

## Retired

`la-14` and `la-17`: external junction unlinked non-recursively, `git worktree remove
--force`, `B:\targets\la-14` and `B:\targets\la-17` deleted. Branches kept.

## Created

`.claude/worktrees/la-15` on `launcher-telemetry/la-15-light-status` at `d273e29b1`,
external junctioned.
