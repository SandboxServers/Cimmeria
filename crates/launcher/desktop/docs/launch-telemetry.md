# Launch contract: opt-in game telemetry

> **Type:** Reference
> **Audience:** Launcher integrators and playtesters
> **Last updated:** 2026-10-26
> **Companions:** [Native game launch contract](launch.md), [client telemetry architecture](../../../../docs/architecture/client-telemetry.md), [operations](../../../../docs/operations/telemetry.md)

How the launcher injects the game-telemetry DLL with a server session, uploads the bundle and logs the result. The shared admission and lifecycle rules are in [launch.md](launch.md).

## Opt-in game telemetry

The launch helper injects `cimmeria-client-telemetry` after the client patches
when the player has opted in. It is off by default. The DLL observes the
running game and uploads events and engine logs to the server; what it
captures is described in [client-telemetry.md](../../../../docs/architecture/client-telemetry.md).

**The choice is its own record.** `game-telemetry.json` in the state directory
holds `opted_in` and, from the first opt-in, an install identity. The
`game_telemetry_command` (`inspect`, `set`) reads and saves it. Launcher-summary
consent (`preferences.json`) is a different choice and never turns it on, and
saving one does not touch the other. Opting out keeps the identity, so a later
opt-in is the same install to the server.

**The DLL is an optional bundle resource.** A build that pins no
`CIMMERIA_CLIENT_TELEMETRY_SHA256`, or whose DLL no longer matches, cannot offer
game telemetry: `set` with `opted_in: true` fails with `platform_unavailable`
and saves nothing. Play is unaffected. A `lab-bridge` build is refused three
times: by `tools/stage-helper.py --kind client-telemetry`, when the shell
resolves its bundle, and at admission. It is built natively by the
`launcher runtime probe` workflow, which logs its SHA-256 next to the patch
DLL's.

**What Play does when it is on:**

1. `resolve_play_resources` keeps the DLL in the launch resources. Admission
   refuses it for a player who is not opted in, even from a host that skipped
   resolution.
2. The worker asks the installation's first login server for a session:
   `POST <login server>/api/auth/dev-session` (`https://<name>/api/...` when the login server is http on a DNS name, LI-09), with the identity and the tags
   `desktop-launcher`, the host OS and `wine`, `proton` or `native`. The wait is bounded
   to 8 seconds. Addresses follow the Windows launcher's rule
   (`telemetry/endpoint.rs`, compiled into the engine): https anywhere, plain
   http only to this machine; an http login server on a dotted DNS name is asked over https on 443. The rule
   is applied to the upload address the server returns as well.
3. The session marker is written to `Working/Binaries/sessions/current-session.json`,
   the file the DLL reads at boot. It is the same shape the Windows launcher
   writes (`telemetry/session.rs`, compiled into the engine).
4. The DLL is added to the helper's request after the patch DLL. The helper
   already accepts two DLLs; it is unchanged.

No session, a refused address or an unwritable marker means the DLL is left
out and the game starts as it would with telemetry off. `game-telemetry-status.json`
records which of those happened for the last launch (`attached`,
`session_unavailable`, `endpoint_refused`, `session_not_written`), and the
main-page Game diagnostics status shows it. `attached` means the DLL was on the injection list with a marker on
disk. It does not claim the DLL loaded or that an upload arrived; the DLL's own
`cimmeria-client-telemetry.log` beside `SGW.exe` and the server are the evidence
for those.

**The launcher's half of the session** (`game_telemetry/session_loop/`). With a
session minted, the launcher also uploads, as the Windows launcher's runner
does, through its own queue (`telemetry-queue.jsonl` in the state directory):

- Before the helper spawns the game, every tailed file already on disk is taken
  in at its end, so an earlier session's lines never go again. A file created
  later is read from its first line. An older file that only now matches (a log
  rotated by rename) starts at its end, and a renamed tracked file keeps its
  offset. A tick reads at most 1 MiB of each file and carries on next tick.
- On the server's flush cadence it sends each new complete line of
  `Binaries/SGWDebugLog*` (as `debug_log`) and every `.log` and `.txt` under
  `Binaries/sessions` at any depth (as `client_log`; the files the bundle takes,
  case ignored), with the Windows launcher's fields, then flushes.
- One session at a time: the queue is shared, so a new Play waits for the
  previous session's bundle and last flush before it queues anything (at most
  35 seconds, after which the previous session is stopped and its unsent
  events go with the new one). Without the wait, the old session's last flush
  would send the new session's first events under the old session id.
- The supervisor's exit observation ends it: a `session_meta` event of kind
  `game_exited` with `exit_code`, `pid` (the game's), `host_pid` (the helper's)
  and `early`, then the end-of-session bundle, then a last flush bounded to 30
  seconds. It runs after the launch result is recorded and never delays it. An
  ending the supervisor did not observe sends no `game_exited`.
- Closing the launcher while the game runs ends the session there. What was
  queued stays on disk and goes with the next session's first flush. Offsets
  are not saved: a restarted launcher does not resume the session, and the next
  Play's baseline means nothing is sent twice.
- Opting out while the game runs stops it at once: the transport checks the
  choice before and after each queue write and send, nothing more is sent, no
  bundle goes, and no queue is left behind. A chunk already being posted at
  that moment completes.

A plan that does not carry the DLL serializes without a `client_telemetry` key,
so plans written before this field existed keep their digests.

**Capture switches.** The game's environment is built from scratch, so the
DLL's developer switches are copied on purpose, and only when telemetry is
attached: `CIMMERIA_CLIENT_CAPTURE`, `CIMMERIA_CLIENT_HOOKS_ENABLE` and
`CIMMERIA_CLIENT_HOOKS_DISABLE`, each a short comma-separated list. Set them in
the launcher's own environment, for example on macOS:

```bash
open --env CIMMERIA_CLIENT_CAPTURE=unfilter,firehose "<launcher bundle>"
```

**Launcher events.** For an opted-in player the launcher also sends the
Windows launcher's own events, built by the same core code:
`client.launcher.install_result` once per Install, Repair or game Update run,
and the client-patches DLL's `client.patches.boot` and `client.patches.counts`
for each Play. The boot event's `injection` follows the policy Play used, so
an adopted copy whose reviewed choice is patches off reports `opted_out`.
When Play's own telemetry session runs, the client-patches events go through
it: the boot verdict on the first tick after the DLL logs it, the counts at
most once a minute while they move, and whatever is still owed (the verdict
if none was logged, the final counts) after the last tail tick and before
`game_exited`. Otherwise, and always for the install result, they go to the
disk queue and ship with the next session's first flush. Either way they are
reported only for a game that ran.

**Adoption steps.** The shell's subscriber (`shell/src/telemetry.rs`) writes a
rolling `logs/launcher.<date>.log`, 7 kept, with the home folder written as
`~`. The `adoption` forward layer hands each step of **Use an existing game
folder** to a bounded channel that drops when full, and
`launcher_session::LauncherSender` sends it as a `client.launcher.adoption`
event in a `launcher` session. That session is minted on the first step, lasts
15 minutes and is re-minted when it runs out; events are grouped and sent as described under "Forwarding pace, hold and flush" below, and
are kept in memory, never on disk, while the network or the server is unavailable. The six steps are C10 of
`docs/analysis/launcher-adopt-ux/work-packets-wave2.md`.
Nothing leaves without the game-diagnostics opt-in, in a build with the module.
`client.launcher.install_result` is unchanged and still waits for Play.

**Other launcher events.** The shell writes its own events as tracing events, one
target per subsystem: `launcher.startup`, `launcher.crash`, `launcher.update`,
`launcher.game_update`, `launcher.install`, `launcher.repair`, `launcher.launch`,
`launcher.settings`, `launcher.ui` and `launcher.telemetry` are emitted;
`launcher.login` has no emitter yet. Each event has a snake_case `event` name:
`<op>_started` and `<op>_finished` are info, `<op>_failed` is warn. A refusal
(`launch_refused`) and `launch_not_started` are warn too. Wire rows use the same name with
a `client.` prefix (`client.launcher.install`). The full subsystem table is in
[observability.md](../../../../docs/architecture/observability.md). `launcher.launch` carries `launch_refused`, `launch_started`, `launch_not_started`,
`launch_failed`, `game_started`, `game_exited`, `launch_finished`,
`display_settings` ([Display settings before each Play](launch-display.md#display-settings-before-each-play)) and `display_placement` ([Opening the game on the remembered monitor](launch-display.md#opening-the-game-on-the-remembered-monitor)), each with the
operation number (except a `launch_refused` before any operation, see above); refusals, failures and not-started launches add a closed `reason`.
`launch_started` also carries `adoption_check` (`light`, `full`, or `none` for an
install not adopted from a folder) and `patches_incomplete` (how many patches the
copy's `launcher-installed.json` lists in `incomplete_patches`). When the installed
copy cannot be read for those tags, the launcher logs a WARN `launch_tags_unavailable`
(the event name only, no error text) and the launch goes on untagged (`none`, 0).
Two events on `launcher.telemetry` are named here: `pipeline_recovered`, one row when the
upload pipeline comes back after failures, and `game_session_mint_failed`, when
a game session could not be minted and Play went on with no game diagnostics.
Their fields are not listed in this file, apart from `events_lost` below.

**Forwarding pace, hold and flush.** The forwarder sends a batch once the oldest queued event has waited 2 s, or 64 events are queued, and at least 7 s after the previous send. A 429 or 503 keeps the unsent events and holds sends for the wait the server sets, at least 60 s and at most an hour (60 s when it sets none). A failed mint keeps them too. A transport or 5xx failure, or a 408, keeps them and holds sends for 60 s. A 401 mints a new token once and tries again; a second 401, a 4xx other than 401, 408 and 429, or a JSON, IO or split error drops the events, because the server would refuse them on every retry. A dropped batch with no retry point (`retry_from: None`) is final. While a hold runs, up to 500 events wait in memory, oldest dropped first, and go out when the hold ends. Nothing is written to disk, and turning diagnostics off empties the queue. `pipeline_recovered` carries `events_lost`, the events dropped since the last successful send. The queue is flushed on `launch_finished` and on exit; a flush skips the 2 s and 7 s waits but sends nothing while a hold runs. The exit flush waits at most 3 s for the forwarder to acknowledge it, so an exit is never delayed longer than that.

**Not covered:**

- An install result is queued only by the journal commit that ends the run.
  A run ended any other way sends none: one finished by reconciliation after
  a restart, one whose launcher stopped between its terminal commit and the
  queue append, and a Repair or Update preparation that finished after its
  owner went away (left for reconciliation). Nothing
  records which runs were reported, so reporting them later could not be
  made exactly-once.
- An adopted copy still launches without game telemetry. Its imported choice
  is reported as unavailable, as before.
- The session token is minted once per Play and is not refreshed. A session
  that outlives the token stops uploading.
- Lines the game writes after the launcher closed are not streamed, and that
  session's bundle is not sent (see below).
