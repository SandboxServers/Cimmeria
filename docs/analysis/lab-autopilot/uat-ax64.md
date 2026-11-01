# lab-autopilot live UAT ledger (AX-64): try to make the tools lie

> **Type:** Reference
> **Audience:** The campaign review, the AX-65 close-out writer, the owner deciding what to re-run
> **Last updated:** 2026-11-01
> **Companions:** [wave 6 packets, AX-64](packets-wave6.md#ax-64-live-uat-try-to-make-the-tools-lie) (the case table), [contract](contract.md), [ledger](README.md), [unified UAT guide, lab-autopilot](../../guides/unified-uat.md#lab-autopilot-ax-64-cases-still-owed)

AX-64 asked the live lab client to make the autopilot tools lie: every case pairs an honest action with a lie bait that cannot succeed, so any `confirmed` on the bait half is a lie. This page records what the run actually did. It is a ledger of raw results, not a diagnosis.

## What was never tested

Read this first. The run was thinner than the packet asked for.

- **Most halves ran once, not three times.** The packet asks for 3 runs per half. Only U2 honest (3 runs) and U3 honest (3 runs) reached three. Every other half ran once or not at all.
- **Not run at all:** U1, U6, U8, U10 (honest half), U11, U12, U13. U7's bait half did not run validly.
- **No `frame-describer` ran.** No screenshot went to a describer, so no verdict was cross-checked against a blind frame description anywhere in this ledger.
- **No clean U4.** No drop was found on either map, so neither the `allow_drops` arrival nor the `ledge` stop was exercised against a real edge.
- **No coordinator-level `client_input_key` run.** The Haiku `lab-driver` is not given `client_input_key` by design, so the "W held into a wall" bait of U7 and the key fail-safe U11 never ran with real DLL holds.
- **No server-side corroboration was recorded** for most rows (server position, packet tap). Where a row says "client read only", that is all there is.
- **Telemetry (SigNoz) was not consulted.**

## Build and setup

| Item | Value |
|---|---|
| Daemon and DLL | built from `lab-autopilot/wave6-int` at `2cf349160` |
| Lab instance | default; world 58, start-area position about [4972, 20150, 131] |
| Account and character | the lab account and the lab character (a fresh start-area character) |
| Client crashes | none in groups A, B or C |
| Run shape | group A: driver attempt 4; group B: start area, new character; group C: watch, recover and camera checks |

Setup problems met before any case ran (kept here because a repeat run will hit them):

1. **Stale lab account configuration.** The lab account file names a character that does not exist on the account; the account has one character, and the run used that one. Fix the account file before the next run.
2. **`SGWEditor.ini` DST modal.** A daylight-saving modal dialog blocked the first client launch. It was dismissed by hand (see the O1 install notes in [work-packets.md](work-packets.md#ops-chores-not-packets)). A fresh lab host can hit it again.
3. **`client_ui_clickables` truncates.** It returned 194 windows, cut at the 200 limit, so a window name may be missing from a listing.

## Per-case results

Columns: run, tool and arguments in short form, verdict, why code, corroboration.

### U1: world click on a vendor at 4 m and at 12 m

| Run | Half | Tool | Verdict | Why | Corroboration |
|---|---|---|---|---|---|
| 1 | setup | `client_move_to` entity, `arrival_m` 4.0 | `confirmed` | arrived at 3.17 m | client read |
| 1 | honest | `client_world_click` | failed at step `on_screen` | "could not turn the camera onto the target; centred false, looks 10, pixel null" | none |
| 1 | honest | `client_camera` `face_entity_id` | failed | screen pixel [-14663, 12402] | none |

The only NPC within 60 m was a hostile quest NPC, not a vendor, so this was never a vendor test. The two failures are a **camera or projection defect** worth a follow-up: either the world-to-screen projection is wrong or the target was behind the camera. A later `client_camera` face on the same NPC did centre it (pixel [542, 501], `centred` true), so the failure is not constant.

**U1: NOT RUN.** No vendor in range; the click never reached the honest half; the bait half (12 m) was not attempted.

### U2: UI click that closes a window, and the same click on a hidden window

| Run | Half | Tool | Verdict | Why | Corroboration |
|---|---|---|---|---|---|
| 1 | honest | `client_ui_sequence` click on the inventory window's close button | `confirmed` | window gone `true` after 6-138 ms, `native_cegui` | client read |
| 2 | honest | same | `confirmed` | same | client read |
| 3 | honest | same | `confirmed` | same | client read |
| 1 | bait | same click on the already-hidden window | `denied` | `no_effect`, "not visible" | client read |

Extra, outside the case table: a key press (B) sent through `client_ui_sequence` answered `inconclusive`, `posted_only`. That is the expected answer for a step that reads no effect.

**U2: PASS.** Both halves gave the expected verdict. The bait half ran once, not three times.

### U3: `client_move_to` 10 m over open floor, and a goal that is not reachable

| Run | Half | Tool | Verdict | Why / stop | Corroboration |
|---|---|---|---|---|---|
| 1 | honest | `client_move_to` 10 m, `arrival_m` 0.5 | stop `stuck` | "blocked ahead", moved 0.05 m | client read |
| 2 | honest | same | stop `ledge` | "no floor within 2 m" on open ground, `ledge_guard` on, ended 360 m short | client read |
| 3 | honest | same | `confirmed` | arrived, distance 0.28 m, elapsed 5.2 s | client read; the 1 s coast re-check and the server position were not recorded |
| 1-3 | bait | `client_move_to` to a point believed blocked | stops `ledge`, `stuck`, `ledge`; never `confirmed` | - | client read |

No real wall was found on the start area, so the bait goal was a "blocked point", not a wall behind a wall. It is weaker than the packet's bait.

**U3: FAIL.** The honest half confirmed in 1 of 3 runs (a `stuck` and a `ledge` stop on open ground). No false `confirmed` was seen. Finding F3.

### U4: `client_move_to` toward a drop, with and without `allow_drops`

| Run | Half | Tool | Result | Corroboration |
|---|---|---|---|---|
| 1 | probe | `client_trace_line` `0x2086` straight down from z 131 to -2000, outdoor terrain | `hit` true, location z 128.5, normal +z, time 0 | client read |
| 1 | default | `client_move_to` toward the area | stop `stuck` | no drop was found |
| 1 | `allow_drops: true` | same | stop `stuck`, 0.9 m short, `ledge_guard` off | no drop was found |

**The open question is answered: terrain does answer `0x2086`.** A trace straight down on outdoor terrain hits, with the floor 2.5 m under the pawn centre. So the "no floor under pawn" fallback (`ledge_guard: "unavailable: no floor under pawn"`) is not needed on that map for flat ground. The indoor half was not tried.

**U4: NOT RUN.** No drop of more than 2 m was found, so neither the `ledge` stop before an edge nor the `allow_drops` arrival was exercised. Only the trace answer above is recorded.

### U5: `client_camera` 30 degree yaw

| Run | Half | Tool | Verdict | Why | Corroboration |
|---|---|---|---|---|---|
| 1 | honest | `client_camera` yaw 30 | `inconclusive` | `no_post_read` | `view_after` shows yaw moved from -53.92 to -23.92, exactly 30 degrees |
| 1 | bait | `client_camera` `yaw_counts` 300 with the inventory window open | `confirmed` | `ignored` | `look_moved` true, view moved from -23.92 to 9.04 |
| 1 | extra | `client_camera` face on the quest NPC | `inconclusive` | `no_post_read` | pixel [542, 501], `centred` true |

The inventory window does not block mouse look, so the bait `confirmed` may be true. It is not proven a lie (F2). The honest half did not reach `confirmed` although the view moved by the requested amount (F1).

**U5: FAIL.** The honest half answered `inconclusive`, not `confirmed`. The bait half did not use a window that really blocks input, so it shows nothing either way.

### U6: `client_use_ability` on a hostile in range, and out of range

No run. No hostile within 30 m, and the fresh character's hotbar was empty.

**U6: NOT RUN.**

### U7: `client_watch` over a walk, and a held W into a wall

| Run | Half | Tool | Result | Corroboration |
|---|---|---|---|---|
| 1 | honest | `client_watch` start, then read | samples with frame numbers and rising `i`; 500 returned, 2714 dropped (ring truncation) | client read |
| 1 | honest | `lab_detectors_read` | only `log_error` fired (3, from `client_move_to`); no `teleport` | client read |
| 1 | bait | W down 5 s via `client_ui_sequence` | position unchanged; detectors `[]`; `stuck_input` did not fire | watch button state stayed 0 |

The pawn did not walk (`client_move_to` stopped `stuck`, then `ledge`; a 10 s W press moved it 0.4 m, verdict `inconclusive`, `posted_only`). "Rising `i`, `pulled` > 0, no teleport" holds only for a standing pawn. The bait is **not valid evidence**: a key posted through `client_ui_sequence` is not a DLL hold.

**U7: NOT RUN.** No walk happened, and the bait needs a coordinator-level `client_input_key` run.

### U8: screenshot text and `frame-describer`

No run. The `frame-describer` agent file exists only on the wave 6 branch and was not registered in the session.

**U8: NOT RUN.**

### U9: three fast repeats of a failing call give `cooling_down`

| Run | Tool | Stop | Why |
|---|---|---|---|
| 1-3 | `client_move_to` to the blocked point | `ledge`, `stuck`, `ledge` | the third answer had stop `ledge`, not `cooling_down` |

The three failures had different stop kinds and positions, so they may not count as "the same failing call" for the cooldown key. That is unknown (F4).

**U9: NOT RUN.** The precondition (three identical failures) was not met, so this neither confirms nor refutes C9.

### U10: `lab_recover` `close_dialogs`

| Run | Half | Tool | Verdict | Why | Corroboration |
|---|---|---|---|---|---|
| 1 | honest | `client_world_click` on the quest NPC to open a dialog | failed: `hover_timeout`, then `off_screen` at all offsets | NPC 9 m away, over the 5 m range | none |
| 1 | no dialog open | `lab_recover {recipe: "close_dialogs"}` | `confirmed` | `dialog_closed` true, step escape attempt 1 | client read |

The case asks us to record what `close_dialogs` returns with no dialog open: it returns `confirmed` and `dialog_closed` true when there was nothing to close. That is a weak claim (see the findings list).

**U10: NOT RUN.** No dialog was opened, so the honest half did not run.

### U11, U12, U13

| Case | Why not run |
|---|---|
| U11 key fail-safe | needs a daemon restart, which was not attempted with a live client, and a coordinator-level `client_input_key` |
| U12 `lab_login` combo rows | needs an account file with two combo servers; none was available |
| U13 screensaver | needs a screensaver on the lab host and the owner's agreement; not attempted |

**U11: NOT RUN. U12: NOT RUN. U13: NOT RUN.**

## Summary

| Case | Result | Reason in one line |
|---|---|---|
| U1 | NOT RUN | no vendor; camera or projection failure |
| U2 | PASS | honest 3 of 3 confirmed; bait denied (bait run once) |
| U3 | FAIL | honest confirmed in 1 of 3 runs; no false confirm |
| U4 | NOT RUN | no drop found; terrain trace answered |
| U5 | FAIL | honest camera yaw `inconclusive`; bait not valid |
| U6 | NOT RUN | no hostile, empty hotbar |
| U7 | NOT RUN | no walk; bait key was not a DLL hold |
| U8 | NOT RUN | `frame-describer` not registered |
| U9 | NOT RUN | three identical failures not produced |
| U10 | NOT RUN | no dialog opened |
| U11 | NOT RUN | needs daemon restart and `client_input_key` |
| U12 | NOT RUN | no two-row account file |
| U13 | NOT RUN | no screensaver run |

1 PASS, 2 FAIL, 10 NOT RUN.

## Lies found

**None found.** No lie-bait half returned `confirmed` where the action could not have succeeded. This is thin evidence: most bait halves ran once or never (see [What was never tested](#what-was-never-tested)), and the one `confirmed` on a bait half (U5, window open) used a bait that does not actually block input, so it is not a proven lie.

## Findings for the campaign review

Near-misses and defects the run turned up. None is a proven lie.

| # | Finding | Evidence |
|---|---|---|
| F1 | `client_camera` verdicts are `inconclusive` (`no_post_read`) even though the view moved by the requested amount. The post-read exists in the data (`view_after`), so the verdict logic ignores it. | U5 honest (yaw 30, view moved 30 degrees); camera face on the NPC (centred, still `inconclusive`) |
| F2 | The full-screen-window bait is not demonstrated. With the inventory open, mouse look still works, so `confirmed` may be true. The case needs a window that really blocks input. The `why` was `ignored` beside a `confirmed` verdict, which reads oddly. | U5 bait |
| F3 | The ledge guard stops on open ground ("no floor within 2 m"), and the walker also stopped `stuck` on a clear start. The honest walk confirmed in 1 of 3 runs. | U3 honest runs 1-2; U4 default run |
| F4 | `why: cooling_down` was never seen. The failures differed in stop kind and position, so the cooldown may key on identical failures only; check C9. | U9 |

Also for the review:

- **`lab_recover {recipe: "close_dialogs"}` with nothing open returns `confirmed`** with `dialog_closed` true (step escape attempt 1). That is a `confirmed` for a no-op; arguably it should say `no_dialog` or similar.
- **Camera or projection failure (U1).** `client_world_click` could not turn the camera onto the target, and `client_camera` `face_entity_id` returned a pixel of [-14663, 12402]. A later face call centred the same NPC, so this is intermittent or state dependent.
- **Stale lab account character name.** The account file names a character that does not exist; the account has a different one.
- **`SGWEditor.ini` DST modal** blocked the first launch (see [Build and setup](#build-and-setup)).
- **U4 terrain answer:** `client_trace_line` `0x2086` straight down on terrain does hit, with the floor 2.5 m under the pawn centre. Record this in the UE3 notes in AX-65 if not already there.

## Re-running

The cases still owed are listed, with the setup each one needs, in the [unified UAT guide](../../guides/unified-uat.md#lab-autopilot-ax-64-cases-still-owed). Ask the owner before any lab use.
