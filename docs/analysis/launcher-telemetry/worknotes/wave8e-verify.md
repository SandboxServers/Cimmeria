# wave8e-verify

Worktree lt-light-integration, branch launcher-telemetry/light-adoption, tip `8581a0833` (upstream still `ebf8a46f3`). **Result: failed (flaky test). Nothing committed, nothing pushed.**

## The failure

`cargo test --locked -p cimmeria-launcher-core -p cimmeria-patchset`, first run: 374 passed, **1 failed**, 2 ignored.

- `install::tolerant_tests::a_failed_retry_logs_one_warning_with_the_patch_label_and_closed_code` (N2's new test), `crates/launcher-core/src/install_tolerant_tests.rs:378`: `assert_eq!(hits.len(), 1)` got 0, and the captured row list was `[]`. The subscriber saw no events at all, not a wrong event.
- Failures file: `C:/Users/Steve/AppData/Local/cimmeria-build/logs/lt-light-integration/20261022-062940-73568.failures.txt`.
- The test passed when run alone. A second full run passed too: 449 passed, 0 failed, 10 ignored across core and patchset.
- So the test is **order- and concurrency-dependent**. The likely cause is that it uses `tracing::subscriber::set_default` with a hand-written `Subscriber` while other libtest threads hit the same callsites without a dispatcher. tracing caches callsite interest and the global max level across threads, so the WARN can be filtered out before this thread's dispatcher counts. This cause is a hypothesis; I did not prove it.
- Possible fixes, for the coder:
  - Implement `register_callsite` to return `Interest::sometimes()`, and call `tracing::callsite::rebuild_interest_cache()` after `set_default`.
  - Or serialize the test.
  - Or add `tracing-subscriber` as a dev-dependency and use its test-writer pattern. That needs a lockfile change.
- To prove a fix, run the full core suite several times. A single run does not show it.

## Checks that passed (all through the lane, `--locked`)

- **fmt `--check`:** passed on the root workspace and on the desktop workspace (`crates/launcher/desktop/Cargo.toml`).
- **Frontend build:** `npm ci --ignore-scripts && npm run build` passed (frontend/dist is needed for the shell).
- **Clippy `-D warnings`, `--all-targets`:** clean on `-p cimmeria-launcher-core -p cimmeria-patchset`, and on the desktop workspace (`--workspace`: engine, shell, runtime-probe).
- **Engine `cargo test`:** 836 passed, 0 failed, 3 ignored. That is the earlier 831 plus the 5 new tests.
- **Desktop shell `cargo test`:** 147 passed, 0 failed, 7 ignored.
- **Frontend:**
  - `npm run check`: passed.
  - `npm test`: 227 passed, 0 failed.
  - `npm run uat:adoption-light`: passed, 5 scenarios.
- **`python tools/docs-gen/regen.py --check --skip-crate-graph`:** "generated doc blocks are current". It also printed three pre-existing warnings about `cell-world` `asgard.rs` test attributes, which are unrelated.

## Line counts

- `crates/launcher/desktop/engine/src/storage/adoption/publication.rs`: 562.
- `crates/launcher/desktop/engine/src/storage/adoption/in_place.rs`: 627, which is over the 500 soft cap but under the 700 hard cap.

## Not done

- Step 2 (committing the ledger docs) and step 3 (push, then `gh pr checks 1983`) were not done because of the stop-on-failure rule.
- The worktree status is clean: nothing in dist, node_modules or `launcher_targets.txt` shows as changed.
