---
name: reference-first-session-lab-calibration
description: 2026-10-10 first-session Praxis rows calibrated live on the colo and run by a Haiku lab-driver with a one-line brief; the traps that cost runs
metadata:
  type: reference
---

The `first-session` spec (`docs/guides/uat-specs/first-session.toml`) FS-01 to FS-P5 was calibrated live on the colo on 2026-10-10. A Haiku `lab-driver` then ran it with a one-line brief (one `lab_uat_run` call, about 42k tokens). The calibration table is in the spec header. The traps, each of which cost a run:

- **First-login movie.** The server holds entity introductions for 16 s from `onClientReady` (`cinematic_aoi_hold::HOLD_DURATION`). `lab_ensure_in_world` and `lab_play_character` return during it. `MoviePlayerWin` hides after about 2 s, so it is not the signal: wait 17 s.
- **Clicks on floor targets.** The avatar eats them. Stand to the side, pitch +200 counts from level (+21.97 deg, and positive is down), then face the target. The face also re-pitches, to -28.8 deg each time, and that is the pitch every passing click used; the first calibration wrongly credited +22. Then yaw +230 counts and click with `rotate_camera: false`. The Guard body takes only a `point` click 0.28 m above its origin.
- **Tutorial 5882 sits on top of DialogWin.** Close it with `client_window_click TutorialWin__auto_closebutton__` (works from any page) before the dialog's Done.
- **Haiku and leases.** Told to borrow a lent lease and not touch it, Haiku still checks the status, reads the coordinator as "another holder", and releases the lease at the end. Give it no lease: the run takes its own.
- **The `lab-driver` agent lacked `client_wait_for`.** Added on 2026-10-10, together with `lab_uat_run`, `lab_uat_report` and `client_window_click`.
- **The lab client logged in to a stale `Local` row.** The lab's `LoginInternal.lua` held a `Local` row for a local server on this machine, left from an earlier ability UAT. So the client played on the local server while `server_db_query` read the colo. Check the rows before trusting DB clauses.
- **Stop/start race.** Until PR #1330, `lab_client_stop` returned before SGW.exe exited, and the runner's next `lab_client_start` refused it as "outside the lab". It now waits for both the exit code and the window, up to 10 s.

Two lab clients at once (`default` and `p2`, 5 runs each on 2026-10-10) shook out three more failures:

- **The stop race, again.** `lab_client_stop` first waited only on the exit code. That is set the moment the process is terminated, but its window lingers, and the start guard enumerates windows. `stop` now waits for the window too.
- **The password box read empty after typing.** It happened twice, both on fresh p2 clients that needed 16 Escape presses to reach the login screen; the likely cause is late Escapes clearing the box's focus. The login now types the account and password again once. That has no regression test yet: it waits for the CI fake-client harness.
- **Frost never created by the client (#1341).** The server introduced him in the hold's flush (he was in the witness list), but the client dropped the rest of that bundle: a `client.mercury.request_misparse` then an `unpack_fault` at the flush. That is the client's uninitialized `Bundle::iterator` next-request offset (`docs/reverse-engineering/findings/client-mercury-receive-path.md`), and player clients show it too. The garbage-collection and server-wire-bug theories were both wrong. This is a product bug; the spec does not hide it.

One watchdog relaunch on p2 also left an orphan `SGW.exe` that no instance owned, started in the same second. A likely cause, from review: the watchdog's `after_death` terminates without waiting, `relaunch` skips `check_launch`, and nothing serialises launches. So a runner `lab_client_start` can overlap a relaunch, and the second launch overwrites the pid and orphans the first. Not yet fixed (#1342). The login password miss is #1343.

The 9-call client wire sequence for the same flow is in `docs/architecture/wireclient.md` § Praxis start.
