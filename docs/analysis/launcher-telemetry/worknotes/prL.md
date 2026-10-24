# PR L worknote

Status: done. PR https://github.com/SandboxServers/Cimmeria/pull/1981 is open and not merged; the user queues merges.

Branch `launcher-telemetry/launcher` (worktree `.claude/worktrees/lt-app-integration`), tip `e6f16395a`.

## Steps

1. Merged `launcher-telemetry/lt-25` (`bb5203de0`) into the branch as `875999069`, then `launcher-telemetry/lt-26` (`00d5b3be7`) as `e6f16395a`. Both used `--no-ff` and had no conflicts.
2. Fetched origin. origin/main (`e7487da3e`, PR A #1978) was already an ancestor of the tip, so nothing needed merging.
3. Checks on the tip, run in the foreground through the lane:
   - fmt --check: ok.
   - clippy -D warnings: ok for the engine, and ok for the shell, which was built against a fresh `frontend/dist` (`npm ci --ignore-scripts` and `npm run build`).
   - Tests:
     - engine: 761 passed, 0 failed, 3 ignored
     - shell, run twice: 142 passed, 0 failed, 7 ignored both times
     - launcher-core: 348 passed, 0 failed, 2 ignored
   - Frontend:
     - `npm run check`: exit 0
     - `npm test`: 198 passed, 0 failed
     - `npm run uat:launcher-telemetry`: all checks passed (`LAUNCHER_TELEMETRY_UAT_BINARY` set to the shell test binary under `B:\targets\lt-app-integration\debug\deps\`).
4. The LT-25 and LT-26 worknotes were already committed, so nothing was copied. Retired lt-25 and lt-26 with `rm-worktree.ps1 --force`, which also removed `B:\targets\lt-25` and `B:\targets\lt-26`. Both branches were kept.
5. Shipped with `ship.py pr`, which returned `status=ok pr=1981`.

## Not done or not tested

- There was no live lab run and no UAT on a real machine.
- LT-37 (publish) is still pending.
- LT-24, LT-41 (the `https://play.cimmeria.app` default) and LT-27 follow in PRs F, H and X.
