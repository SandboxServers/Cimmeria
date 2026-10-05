# Launcher observability implementation assignment

> **Type:** How-to
> **Audience:** Fresh Windows observability implementation session
> **Last updated:** 2026-10-04
> **Companions:** [Discovery](observability-discovery.md), [requirements](../launcher-implementation-plan.md), [acceptance](../launcher-acceptance.md)

## Base and ownership

Start from the exact integration commit containing this assignment on
`prototype/launcher-packaging-proof`; record that full SHA before creating the
isolated registered worktree `launcher-observability-implementation` and branch
`launcher/observability-implementation`. Do not use the old discovery base.
The coordinator has integrated discovery commit `56b2599aa`.

Use the discovery handoff's section 4 ownership. The observability worker now
owns its listed shared Rust integration surfaces for this phase, with changes
isolated in a separate integration commit. The coordinator reserves campaign
indexes, ledger, general desktop README and CI. Coordinate any other writer
before touching a file outside the table. No frontend edits are assigned.

The new base also has migration UI, minimum-launcher policy/admission gates and
Mac graphics-driver selection. Preserve these. In particular, `IntentError`
and shell `JobError` now include `LauncherTooOld`; do not erase those cases.
`DesktopState::open` has immutable compatibility policy and migration recovery.
The journal still provides common operation admission/transition evidence.
Revalidate sources by symbol; discovery line numbers are historical.

## Recorded integration decisions

- Endpoint stays `None` in distributed/development configuration. No production
  mint, upload, collector, VPN or live observability request is authorized.
  Local fixtures prove the complete producer-to-ingest path.
- No installation correlator in schema v1. Engine-generated opaque attempt and
  event IDs remain independent of renderer operation IDs; no identifiers,
  account/character names, paths, raw errors or URLs in export projections.
- Use the proposed journal observer, with failure isolation and consent gates.
  Treat reconciliation-required/lost observation as unknown, never success.
  Current Launch succeeds at observed process exit code zero, not merely spawn;
  retain separate phase evidence and never relabel this as login/world success.
- Implement and test the proposed scoped ingest router. Keep its public
  login-listener merge isolated in the shared integration commit for explicit
  maintainer review: the existing four-route production boundary is a recorded
  decision. Local route-exposure tests are authorized; deployment/public
  activation is not. Do not add unrelated admin surfaces.
- Smaller queue/count bounds are acceptable tuning within the plan's ceilings.
  The discovery proposal to omit all phase durations does **not** fulfill the
  requested phase-duration view. Retain one terminal attempt summary plus a
  bounded set of closed phase summaries (at most 32, no more than 2 KiB each),
  or an equivalently bounded typed phase-duration representation. Never invent
  timings after restart or infer unobserved phases. Query fixtures must support
  both terminal attempt outcomes and phase-duration distributions with sample
  counts, while avoiding double-counting attempts. Document actual coverage.
- Error enums must include actual current launch and minimum-version results.
  Pre-admission failures cannot smuggle renderer-controlled strings into rows.

## Implementation prompt

Implement the discovery's sections 2–6 subject to the decisions above. Read
repo instructions and the drift table first. Deliver schema/projection, bounded
acknowledged queue, consent and opt-out race handling, single failure-isolated
export task, scoped mint/ingest with dedup and quota, shared golden fixtures,
source-to-local-ingest proof and offline query/dashboard guards. Native Windows
checks are part of this phase; compile only through the build lane. Do not edit
dependencies without a concrete evidence-backed need and coordinator handoff.

Use the existing advisor definitions for auth, authority and validation. A
negative exporter test must await the cycle and have a positive control.
Revert-verify journal, consent and routing guards. Preserve default-off consent,
no retroactive export and independence from game/DLL telemetry. Export errors
must leave installation, Play, recovery and preferences usable.

One bounded implementation phase. Commit owned and shared changes separately;
push only the worker branch, open a draft PR targeting
`prototype/launcher-packaging-proof`, and report exact base/head, commands,
results, exclusions and remaining public-activation gate. Never merge, deploy,
force-push or publish a release. Produce `worknotes/observability.md` and project
memory; leave campaign/shared indexes to the coordinator.
