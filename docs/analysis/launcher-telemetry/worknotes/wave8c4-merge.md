# Wave 8c4 merge (LA-16)

Integration worktree `.claude/worktrees/lt-light-integration`, branch
`launcher-telemetry/light-adoption`. Start tip `b33093898` (verified, tree clean).

## Merge (`--no-ff`, no conflicts)

| Packet | Branch tip | Merge commit |
|---|---|---|
| LA-16 light UI | `fd19480cb` (launcher-telemetry/la-16-light-ui) | `3dfe4e585` |

Docs commit `54f1fd21e` (CRLF, copied from the main checkout): architect amendments to
`work-packets-light-adoption.md` (+91/-2), new `worknotes/wave8c3-merge.md`,
`worknotes/wave8c4-review.md`, `worknotes/wave8c5-review.md`.

## Verification on `54f1fd21e` (build lane, foreground)

| Target | fmt --check | clippy -D warnings (--all-targets) | tests |
|---|---|---|---|
| launcher-core | ok (root `--all`) | ok (`--manifest-path crates/launcher-core/Cargo.toml`) | nextest 372 passed, 2 skipped |
| cimmeria-patchset | ok | ok | nextest 74 passed, 8 skipped |
| cimmeria-launcher-engine | ok (desktop `--all`) | ok (desktop `--workspace`) | libtest 830 passed (828 + 2), 0 failed, 3 ignored |
| cimmeria-launcher-desktop (shell) | ok (desktop `--all`) | ok (desktop `--workspace`; re-run `-p` verbose confirmed shell rechecked) | nextest 146 passed, 7 skipped |
| frontend | `npm run check` ok | - | `npm test` 217 passed, 0 failed |

No fmt fix needed. Crate roots were touched before clippy so the cached target could not
mask the merge. Shell clippy ran after `npm ci --ignore-scripts && npm run build`; dist,
node_modules and launcher_targets.txt were not staged (tree clean after the runs).
Frontend tests 206 -> 217 with LA-16; shell stays at 146.

**New tip: `54f1fd21e`**, pushed to origin (`b33093898..54f1fd21e`).

## Retired

`la-16`: external junction unlinked non-recursively (main `external/` intact), `git
worktree remove --force`, `B:\targets\la-16` deleted. Branch
`launcher-telemetry/la-16-light-ui` kept (`fd19480cb`).

## Next worktree

`la-18` created at `54f1fd21e` on new branch `launcher-telemetry/la-18-light-uat`,
`external/` junctioned. Target dir not pre-seeded (first build is cold).