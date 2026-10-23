# Adoption and Play phase timing

> **Type:** Explanation · **Audience:** Launcher engineers · **Updated:** 2026-10-23
> **Companions:** [Ledger](README.md), [launcher telemetry](../../../crates/launcher/desktop/docs/launcher-telemetry.md)

## Evidence and goal

Existing launcher events arrive in SigNoz under `client.native` with a
`client.launcher.*` target, separately from the anonymous summary stream. A
consented Windows 2026.10.2304 session measured about 29m24s from adoption
confirmation to completion, for 31,605 files (25,619 extras). Its admitted Play
reached `game_started` in about 3.3s. Terminal events cannot identify which
adoption scan, lock wait, preparation step or helper handshake consumed time.

Add bounded phase start/finish records using the existing opt-in forwarding
pipeline and rotating local logs. Do not change consent, summary schema,
network destinations, verification policy, scheduling, or any serialized Plan.

## Implementation

Use a monotonic native timer with closed phase/outcome enums, a process-local
operation number and a unique phase number. Paired start/finish records let an
operator distinguish work still running from missing terminal data. A dropped
timer reports interruption (or panic); explicit Result completion reports
success/error. No paths, account names, identifiers, tokens or arbitrary errors
are added. Scan completions add file/byte counts, so duration can be compared
with workload. Preview work receives a temporary native operation number;
confirmation and Play reuse their existing operation UUID only to obtain its
nonpersistent number, never exporting the UUID.

Adoption phases cover preview, initial scan, cleanup rescan, comparison/index,
patch replay, confirmation rescan, content application, setup and final tree
verification. Play phases cover native request/admission, resource verification,
telemetry session mint, client setup, previous-session settlement, helper spawn
and its process-start handshake. `game_started` gains startup milliseconds from
the first native Play request, independently of eventual game lifetime.

Use existing `adoption` and `launcher.launch` targets so the sender, server scalar
scrubber and existing quotas work without a server deployment. Record an exact
forwarding/ingest fixture test so the new fields cannot silently disappear.

## Validation and limits

Pick log-capture unit tests and real fixture-path integration tests: successful,
failed/cancelled scans; matched final verification; refused launch admission;
resource failure and helper process-start response. Prove representative guards
fail when timing calls are removed, then restore and run standalone engine/shell
tests, format and clippy through the build lane. No frontend behavior changes.
Document JSON `fields` queries, incomplete pairs and lossy opt-in forwarding.
Published player data cannot retrospectively provide the new phase timings.
No live game, laptop, lab, server deployment or release is part of this change.
