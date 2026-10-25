# Launcher native phase timing

> **Type:** Reference · **Audience:** Launcher engineers and telemetry operators · **Updated:** 2026-10-23
> **Companions:** [Desktop overview](../README.md), [dev-session architecture](../../../../docs/architecture/dev-session-telemetry.md), [operator queries](../../../../docs/operations/telemetry.md#launcher-adoption-and-play-phase-timing), [design](../../../../docs/analysis/launcher-uat-fixes/design-phase-timing.md)

## Purpose and transport

Native adoption and Play timers distinguish slow disk work, state admission,
setup, telemetry preparation and helper startup. They describe launcher work;
process start does not mean login, world entry or a successful game session.
The timers do not change verification, scheduling, operation persistence or the
serialized Plan.

Local records use the existing rotating launcher log. With the existing launcher
event opt-in, target `adoption` forwards as `client.launcher.adoption`, and target
`launcher.launch` forwards as `client.launcher.launch`. In SigNoz both appear as
`scope_name = client.native`, service `cimmeria-client`; their structured payload
is JSON text in `attributes_string['fields']`. These records are separate from
`launcher.summary`; no summary schema or anonymous export contract changes.

## Reading a lifecycle

The timer is defined in `engine/src/phase_timing.rs`. Its field contract is:

| Field | Values and meaning |
|---|---|
| `event` | `phase_started` or `phase_finished` |
| `phase` | One of the closed phase codes below |
| `operation` | Process-local numeric operation correlation; preview and confirmed adoption are separate operations |
| `phase_sequence` | Process-local sequence identifying one start/finish pair, including repeated scans |
| `duration_ms` | Saturating unsigned monotonic milliseconds; finish only |
| `outcome` | Finish only: `succeeded`, `failed`, `unavailable`, `cancelled`, `interrupted` or `panicked` |
| `files`, `bytes` | Successful full-tree scan only; number of scanned entries and saturating sum of their sizes |

Start rows omit outcome, duration and workload. Failed/partial scans omit counts
rather than publishing a partial total. The `cancelled` outcome is an explicit
completion code; dropping an unfinished asynchronous timer reports `interrupted`.

| Phase code | Native boundary |
|---|---|
| `adoption_preview` | Overall preview |
| `adoption_initial_scan` | Initial source-tree inventory |
| `adoption_cleanup_scan` | Rescan after clearing an unbegun admission |
| `adoption_comparison` | Comparison with the reference |
| `adoption_index` | Reference index preparation |
| `adoption_replay` | Expected patch/setup replay |
| `adoption_review_scan` | Review inventory before comparison |
| `adoption_confirmation` | Overall confirmed adoption |
| `adoption_confirmation_scan` | Source verification at confirmation |
| `adoption_apply` | Approved content application |
| `adoption_publication_lock` | Wait for the state lock before publication |
| `adoption_setup` | Client setup at the selected root |
| `adoption_final_scan` | Final actual-tree inventory before tree comparison |
| `play_request` | Native Play request, including pre-admission work |
| `play_dispatch_queue` | Native Tauri IPC receipt through blocking-worker start; Play only |
| `play_admission` | Launch admission while holding the state lock |
| `play_resources` | Native launch-resource verification |
| `play_telemetry_mint` | Optional game-telemetry session preparation |
| `play_client_setup` | Client setup before starting the game |
| `play_previous_session` | Settlement of previous game-session telemetry |
| `play_helper_start` | Helper spawn and process-start handshake |
| `play_helper_spawn` | Operating-system helper spawn |
| `play_helper_handshake` | Request transfer and first process-start reply |

Each native phase emits a start and finish, with a monotonic elapsed duration on
the finish. Closed phase/outcome codes and numeric correlation values keep the
records bounded. Correlation is process-local: never combine matching numbers
from different launcher runs. It is neither a durable operation identifier nor
an account, installation or session identity. Preview uses a temporary native
operation number; confirmation and Play map their existing operation to a
nonpersistent number without exporting the operation UUID.
The operation-number cache retains 64 entries; an operation forgotten after
that many other operations receives a new number if seen again. Use
`phase_sequence` to identify an individual pair.

The existing `game_started` event adds `startup_ms`, elapsed since this native
process first registered that Play operation. On the packaged Tauri path the
queue timer registers it at native IPC receipt, before `spawn_blocking`, so
startup includes blocking-worker queue wait. Inspect emits no queue timer.
Direct host entry starts at `play_request`. Neither includes browser/UI time
before native IPC receipt. Completion means the helper's `ProcessStarted`
response, not a first rendered frame, login or world entry. The short
`play_request` timer ends when the native request returns; asynchronous worker
and helper phases can continue afterward.
The shell acquires the state lock before entering the engine's admission
timer. That host lock wait is included in aggregate `play_request`, not
`play_admission`, and has no separately timed phase.

Explicit completion records success or error; a guard dropped before completion
records interruption, or panic while unwinding. Known helper startup failures
and login-server refusal explicitly report `failed`. An unfinished future
dropped before completion reports `interrupted`; remaining early returns can
also report `interrupted`. Cancellation can leave only a start if the process
terminates abruptly. A missing finish can also mean lost
forwarding, opt-out, quota exhaustion or an upload failure; it is not proof that
the operation is still running. Nested durations overlap and must not be added
into a wall-clock total.
An idempotent adoption can legitimately execute `adoption_apply` more than once.
Keep each `phase_sequence` pair; do not discard a repeated phase name as a
duplicate or infer a second adoption from it.

Scan completions report file and byte counts for workload comparison. These are
scan workloads, not downloaded bytes or proof of relocation. No per-file event
is emitted. The timers add no path, account name, machine identity, token,
operation UUID or arbitrary error text. The `launcher.install.detail` and
`launcher.repair.detail` targets stay in the local log and are never forwarded.

## Delivery limits

This uses the existing launcher sender and server scalar scrubber, with no new
endpoint, secret or server deployment. Event sessions retain their 300-row
lifetime budget, 200-row chunk maximum and 16 scalar keys per row; all existing
rate limits still apply. Events are not durably queued by the launcher sender.
Compare received observations within their known coverage; do not calculate an
all-player success rate or interpret an absent phase as zero milliseconds.
Manual Upload debug logs retains its separate one-upload consent and logs-purpose
budget. See [operations](../../../../docs/operations/telemetry.md#launcher-sessions)
for the authoritative limits and [launcher summaries](launcher-summaries.md) for
the separate summary pipeline.
