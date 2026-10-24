# Wave 4 merge into launcher integration

Date: 2026-10-21. Integration worktree `.claude/worktrees/lt-app-integration`, branch
`launcher-telemetry/launcher`. Not pushed, no PR.

## Merges (`git merge --no-ff`, no conflicts)

| Lane | Branch | Lane tip | Merge commit |
|------|--------|----------|--------------|
| LT-18 | launcher-telemetry/lt-18-launch-stage | d851d1884 | 95a46b9f1 |
| LT-19 | launcher-telemetry/lt-19-settings | 4b50c16a9 | 05cf16cf7 |
| LT-20 | launcher-telemetry/lt-20-frontend-ui | c443ce1c1 | 1a80df6be |
| LT-21 (21a, 21b, fixes) | launcher-telemetry/lt-21-log-upload | dc37df960 | 252f4c1a2 |
| LT-22 (22, 23a, 23b, fixes) | launcher-telemetry/lt-22-summary-endpoint | cf9a7c874 | 5c7f82b81 |

Base 3ca280eb8, new tip **5c7f82b81**. All five merged cleanly. The expected overlap in
`engine/src/commands.rs` (LT-19 vs LT-23b) merged without a conflict.

## Verification on 5c7f82b81 (build lane, foreground)

| Check | Result |
|-------|--------|
| `cargo fmt --check --all` (desktop workspace) | ok |
| `cargo clippy --locked --all-targets -D warnings` for engine + shell (`cimmeria-launcher-desktop`) | ok |
| `cargo test --locked -p cimmeria-launcher-engine` | 761 passed, 0 failed, 3 ignored |
| `cargo test --locked` launcher-core | 348 passed, 0 failed, 2 ignored |
| `cargo test --locked -p cimmeria-launcher-desktop` (shell), run 1 | 141 passed, **1 failed**, 6 ignored |
| shell, run 2 | 141 passed, **1 failed**, 6 ignored |
| frontend `npm ci --ignore-scripts`, `npm run build`, `npm run check` | ok |
| frontend `npm test` | 198 passed, 0 failed |

Note: the shell package is named `cimmeria-launcher-desktop`, not `cimmeria-launcher-shell`.

## The suspect shell test

`host::repair::integration_tests::retained_host_handoff_commits_signed_seed_and_reports_current_backup_after_cleanup`
panics at `shell/src/host/repair/integration_tests.rs:99` with `install_status failed: Io`.

- It failed in both full runs, alone (`-- retained_host_handoff`) and with only the
  `host::repair` module. On this machine today it fails every time.
- **It fails the same way on the pre-wave-4 tip 3ca280eb8** (checked out detached,
  run alone). So wave 4 didn't cause it. The failure already existed, most likely from
  the wave-3 LT-16 repair merge.
- No wave-4 merge touches `shell/src/host/repair`, `shell/src/host/install` or the
  repair engine. The only wave-4 shell change is `shell/src/host/summary.rs`.
- No other panic shows in the log, so this is probably not a poisoned mutex. It is
  most likely a `StorageError::Io` from a store read (`install_intent`,
  `install_outcome`, `uninstall_target`) that races the repair apply path while files
  are being swapped. Windows sharing violations would fit that. The poll loop
  in `observe()` retries only `JobError::Busy`; any other error panics.
- I didn't fix it, as the brief asked. Follow-up: an owner of the LT-16 repair code
  should decide whether `install_status` should map that transient read to `Busy`,
  or whether the test's poll should tolerate it.

## Clean-up

- Copied untracked worknotes LT-18.md, LT-19.md and LT-20.md to the main checkout's
  `docs/analysis/launcher-telemetry/worknotes/` (untracked there). The LT-21a/21b,
  LT-22, LT-23a and LT-23b notes are already committed on the lane branches.
- Retired worktrees lt-18, lt-19, lt-20, lt-21 and lt-22: unlinked the external junction
  with a non-recursive rmdir, ran `git worktree remove --force`, and deleted the
  `B:\targets\<wt>` folders (lt-20 had none). The main `external/` is intact. I didn't
  use `rm-worktree.ps1` because it deletes the branch. **The branches are kept.**
  I didn't run `git worktree prune`.
- Nothing staged beyond the merges. `launcher_targets.txt`, dist and node_modules were
  never staged.
