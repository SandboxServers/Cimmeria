# Adoption and Play timing validation

> **Type:** Reference · **Audience:** Launcher engineers and reviewers · **Updated:** 2026-10-23
> **Companions:** [Design](../design-phase-timing.md), [ledger](../README.md), [native contract](../../../../crates/launcher/desktop/docs/launcher-telemetry.md), [SGC review](sgc-session-review-2026-10-23.md)

## Implementation coverage

The native contract adds 23 closed phases for adoption and Play, including
blocking-worker dispatch queue wait, scan workload counts and monotonic
process-start timing. It uses existing consented launcher events and changes
no Plan, anonymous summary schema, quota, secret or destination. Frontend
behavior is unchanged, so this change adds no frontend logic-UAT requirement.

There are 12 new tests: nine engine, two shell and one server. The existing
resource-failure regression also verifies phase timing. The tests cover bounded
lifecycle fields/outcomes, repeated phase pairing, real adoption fixtures,
refused Play, dispatch queue behavior, process-start timing, resource failure
and survival of the timing/workload fields through receiver scrubbing.

## Final checks

| Check | Result |
|---|---|
| Standalone workspace/all-target tests through the build lane | 1,178 passed, zero failed, 11 ignored: 997 engine, 158 shell and 23 runtime/binary tests |
| Real fixture-helper process integration | Passed |
| Native shell build | Passed with zero warnings after test-support configuration cleanup |
| Standalone workspace/all-target clippy with warnings denied | Passed |
| Admin-API all-target clippy | Passed |
| Targeted server test `native_phase_workload_and_startup_timing_survive_launcher_ingest` | Passed |
| Root and standalone formatting checks | Passed |
| Frontend dependency install and build | Passed |

An initial engine run had 995 passed, one failed and three ignored. The new
fixture test assumed one `adoption_apply` pass, while the real idempotent fixture
performs two. The assertion now accepts separate correctly paired passes; this
was a test assumption, not a production defect. Final totals above supersede
that intermediate run.

## Mutation proofs

Each deliberate mutation below failed the named assertion. Each was restored,
and the final full standalone suite passed afterward.

| Deliberately removed or changed behavior | Guard that failed |
|---|---|
| Remove `startup_ms` from the process-start event | `refused_admission_and_process_started_have_pre_game_timing` |
| Relabel the final adoption scan as an initial scan | `real_adoption_times_preview_confirmation_and_final_scans` |
| Remove the actual launch-resource timer | `a_replaced_helper_logs_launch_not_started_verify_failed` |

## Review and limits

Review pass one found an inaccurate lock-boundary description and public API
documentation gaps. Both were corrected; a fresh second pass says ship. The
subsequent configuration-only production/test-helper cleanup was also reviewed
ship. PR CI, merge queue, merge and worktree retirement are still next at this
checkpoint.

Not covered: receipt of these new events in live SigNoz, real game or lab runs,
local macOS execution, a performance benchmark or a launcher release. The SQL
recipe has not been run live. Historical player data cannot supply missing
granular timings retroactively. No owner decision is needed for the telemetry
code; gameplay findings remain separate reproduction follow-ups. The previous
release GO was fulfilled by 2026.10.2305, and no new release was requested here.
