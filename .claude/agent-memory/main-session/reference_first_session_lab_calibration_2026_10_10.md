---
name: reference-first-session-lab-calibration
description: 2026-10-10 first-session Praxis rows calibrated live on the colo and run by a Haiku lab-driver with a one-line brief; the traps that cost runs
metadata:
  type: reference
---

The `first-session` spec (`docs/guides/uat-specs/first-session.toml`) FS-01 to FS-P5 was calibrated live on the colo on 2026-10-10. A Haiku `lab-driver` then ran it with a one-line brief (one `lab_uat_run` call, about 42k tokens). The calibration table is in the spec header. The traps, each of which cost a run:

- **First-login movie.** The server holds entity introductions for 16 s from `onClientReady` (`cinematic_aoi_hold::HOLD_DURATION`). `lab_ensure_in_world` and `lab_play_character` return during it. `MoviePlayerWin` hides after about 2 s, so it is not the signal: wait 17 s.
- **Clicks on floor targets.** The avatar eats them. Stand to the side, pitch +200 counts from level (+21.97 deg, and positive is down), face the target, yaw +230 counts, click with `rotate_camera: false`. The Guard body takes only a `point` click 0.28 m above its origin.
- **Tutorial 5882 sits on top of DialogWin.** Close it with `client_window_click TutorialWin__auto_closebutton__` (works from any page) before the dialog's Done.
- **Haiku and leases.** Told to borrow a lent lease and not touch it, Haiku still checks the status, reads the coordinator as "another holder", and releases the lease at the end. Give it no lease: the run takes its own.
- **The `lab-driver` agent lacked `client_wait_for`.** Added on 2026-10-10, together with `lab_uat_run`, `lab_uat_report` and `client_window_click`.
- **The lab client logged in to a stale `Local` row.** The lab's `LoginInternal.lua` held a `Local` row (127.0.0.1:18081) from an earlier ability UAT, so the client played on a local server while `server_db_query` read the colo. Check the row before trusting DB clauses.
- **Stop/start race.** Before the fix in this PR, `lab_client_stop` returned before SGW.exe exited, and the runner's next `lab_client_start` refused it as "outside the lab". `stop` now waits for the exit, which needs a daemon rebuild.

Two lab clients at once (`default` and `p2`, 5 runs each on 2026-10-10) shook out three more failures:

- **The stop race, again.** `lab_client_stop` first waited only on the exit code. That is set the moment the process is terminated, but its window lingers, and the start guard enumerates windows. `stop` now waits for the window too.
- **The password box read empty after typing.** It happened twice, both on fresh p2 clients that needed 16 Escape presses to reach the login screen; the likely cause is late Escapes clearing the box's focus. The login now types the account and password again once. That has no regression test yet: it waits for the CI fake-client harness.
- **Frost dropped by the client under load.** The server introduced him (he was in the witness list), but the client never created him. The 16 s hold timer released while the slower client was still in its post-movie garbage collection: the #838 class. This is a product bug; the spec does not hide it.

One watchdog relaunch on p2 also left an orphan `SGW.exe` that no instance owned, started in the same second. The cause is unexplained.

The 9-call client wire sequence for the same flow is in `docs/architecture/wireclient.md` § Praxis start.
