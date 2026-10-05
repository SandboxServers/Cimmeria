---
name: lab-daemon-lease-gate
description: Where the cimmeria-lab lease gate lives (hand-written call_tool/list_tools), why the UAT runner touches the lease itself, and test traps for the shared HTTP daemon (2026-10-04, PRs #1193/#1195 + PR 3)
metadata:
  type: project
---

**Gate placement.** `#[tool_handler]` skips generating `call_tool` / `list_tools` /
`get_tool` when the impl already has them (rmcp-macros 3.4 `has_method`). The lab writes
`call_tool` (runs `LabServer::gate_call`, then `tool_router.call`) and `list_tools`
(adds a required `lease_id` to guarded tools' schemas) by hand in `server/mod.rs`.
Classification is `crates/lab/src/lease/policy.rs` (`OPEN` / `LEASED` / `OWN_LEASE`);
anything unlisted is guarded and `every_routed_tool_is_classified` fails until a new
tool is placed. A new lab tool therefore needs a policy line.

**Revocation is enforced at the action, not the gate** (review fix, 2026-10-04). Guarded
tools run inside `lease::permit::scope`; `Supervisor::bridge_call`, `process::post_message`
(releases exempt) and `launch_client` call `permit::ensure` and fail "lease revoked".
A new client-acting path must go through one of those, or call `ensure` itself.
`input_release` over the bridge is exempt (cleanup). The task-local does not cross
`tokio::spawn`/`spawn_blocking`: an action in a spawned task is unchecked.

**The runner bypasses the gate.** `lab_uat_run`'s `RouterInvoker` dispatches through
`tool_router` in-process, so it never passes `call_tool`. It carries a `RunLease` and
touches it before every step, plus a `KeepAlive` task renews every ttl/3 and wakes on
`LeaseBook::subscribe` (holder changes); `Runner::with_revocation` races each row
against it and BLOCKs the rest. Any other in-process dispatcher must do the same.

**One lease book per process** (`lease::global()`), shared by the default supervisor and
the in-process p2 supervisor. Tests must give each supervisor its own book
(`Supervisor::with_leases`), or parallel tests see each other's leases.
`daemon::http_tests::test_server()` already does.

**Watchdog.** `after_death` returns `IdleNoLease` when no lease is held; test it with a
fresh supervisor per death, or the 3-in-10-minutes recovery cap decides the outcome
on the third call.

**Daemon edge.** `StreamableHttpServerConfig::enforce_origin_validation()` with an empty
origin list refuses every `Origin`; Claude Code sends none. Exit codes: 2 refused
config, 3 mutex held or port taken (`tools/lab/daemon.ps1 run` stops its restart loop
on both).

Related: [[lab-uat-runner-in-process-tool-calls]], [[worktree-shell-and-external-binary-tests]]
