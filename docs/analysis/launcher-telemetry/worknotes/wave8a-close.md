# Wave 8a close (2026-10-21)

Status: **done**. Follows [work-packets-light-adoption.md](../work-packets-light-adoption.md) § "Starting wave 8a while PR L #1981 is open" and [wave8a-bases.md](wave8a-bases.md).

## A. Integration branch `launcher-telemetry/light-adoption`

- `git fetch origin`: `origin/launcher-telemetry/launcher` is still `e6f16395a` (L_TIP unchanged), so there was nothing from PR L #1981 to merge in. `origin/main` is still `e7487da3e`.
- Merges (`git merge --no-ff`, message `Merge LA-0x into light-adoption`, Sonnet trailers), in order, with no conflicts:

| Packet | Branch | Tip | Merge commit |
|---|---|---|---|
| LA-01 | `launcher-telemetry/la-01-patch-summary` | `94b311ebe` | `dfce4fd66` |
| LA-02 | `launcher-telemetry/la-02-tolerant-apply` | `defa88e29` | `e3fabea1f` |
| LA-06 | `launcher-telemetry/la-06-index-compare` | `6fdc959fc` | `4883cd392` |
| LA-07 | `launcher-telemetry/la-07-remove-adopt` | `12d1d734c` | `efe31ef0a` |
| LA-08 | `launcher-telemetry/la-08-stage-seed` | `25627c167` | `eeae15008` |

- Extra commit `c4126f495`: `docs/analysis/launcher-telemetry/worknotes/LA-07.md`. The LA-07 coder left its worknote untracked in the `la-07` worktree, so the force-retire would have deleted it. LA-08 wrote no worknote.
- **Integration tip: `c4126f495a1a49279238bdcae849a365afa8072b`**, pushed to `origin/launcher-telemetry/light-adoption`. No PR opened.

### Verification (lane, foreground, in `lt-light-integration`)

| Check | Result |
|---|---|
| `cargo fmt --all --check` (workspace) | ok |
| `cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all --check` | ok |
| `cargo clippy --locked -p cimmeria-launcher-core -p cimmeria-patchset --all-targets -- -D warnings` | ok |
| `cargo test --locked -p cimmeria-launcher-core -p cimmeria-patchset` | 436 passed, 0 failed, 10 ignored |
| `cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine -p cimmeria-launcher-desktop --all-targets -- -D warnings` | ok |
| `cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine -p cimmeria-launcher-desktop` | 903 passed, 0 failed, 10 ignored |
| Frontend `npm run check` / `npm test` | not run: the wave changed no frontend file. `frontend/dist` was built (`npm ci --ignore-scripts`, `npm run build`) only so the shell compiles. |

The tree was clean after the checks (dist, node_modules and launcher_targets.txt not staged).

### Retired

`la-01`, `la-02`, `la-06`, `la-07`, `la-08`: removed with `rm-worktree.ps1 --force` (no prune). Their `B:\targets\<wt>` dirs are gone. The script deletes local branches, so all five were recreated at the tips above; they are kept.

## B. PR I2 (LA-05)

- **PR: https://github.com/SandboxServers/Cimmeria/pull/1982**, `File index: accept any https blob URL in verify-file-index (LA-05)`, base `main`. Not merged; the user queues it.
- Shipped with `ship.py pr`. It added one commit, `fe7b57e29` (`docs: add LA-05 worknote`), because the coder left that worknote untracked.
- Lane checks re-run in `la-05` before shipping: fmt ok, clippy `-D warnings` ok, `cargo test --locked -p cimmeria-launcher-core` 346 passed, 0 failed, 2 ignored.
- Not tested: a real publication. That is the LT-37 re-run, which this PR unblocks.
- Worktree `la-05` is kept. Retire it after #1982 merges.
