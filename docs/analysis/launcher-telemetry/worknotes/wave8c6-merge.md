# Wave 8c6 merge (LA-18)

Integration worktree `.claude/worktrees/lt-light-integration`, branch
`launcher-telemetry/light-adoption`. Start tip `5acf46eac` (LA-19 docs on top of
`54f1fd21e`; verified, tree clean, one commit ahead of origin).

## Merge (`--no-ff`, no conflicts)

| Packet | Branch tip | Merge commit |
|---|---|---|
| LA-18 light UAT | `86ac8ad5e` (launcher-telemetry/la-18-light-uat) | `d8132f146` |

Docs commit `89b8bff8d` (CRLF, copied from the main checkout; none were committed before):
new `worknotes/wave8c4-merge.md`, `worknotes/wave8c6-review.md`, `worknotes/LA-19-docs.md`
(the last was LF in the main checkout and was normalised to CRLF).

## Verification on `89b8bff8d` (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok (root `--all`) | ok (`--manifest-path crates/launcher-core/Cargo.toml`) | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`, engine rechecked) | libtest 831 passed, 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`) | nextest 146 passed, 7 skipped |
| frontend | `npm run check` ok | - | `npm test` 227 passed, 0 failed; `npm run uat:adoption-light` passed (5 scenarios) |

No fmt fix needed. Crate roots were touched before clippy. Shell clippy ran after
`npm ci --ignore-scripts && npm run build`; dist, node_modules and launcher_targets.txt
were not staged (tree clean after the runs). Engine tests 830 -> 831 (new
`light_order.rs` test); frontend 217 -> 227.

## Retired

`la-18`: external junction unlinked non-recursively, `git worktree remove --force`,
`B:\targets\la-18` deleted. Branch `launcher-telemetry/la-18-light-uat` kept (`86ac8ad5e`).
