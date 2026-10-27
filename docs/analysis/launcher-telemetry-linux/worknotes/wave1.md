# Wave 1 ruling (review pass 1 = decision)

> **Type:** Reference · **Audience:** The orchestrator and the ruling-check reviewer · **Updated:** 2026-10-26
> **Input:** `ltl-int` `.claude/reports/launcher-telemetry-linux/wave1-review-1.md` (tip `f3cedb7c3`)

Ruling type: **re-spec** of LTL-04 and LTL-05 (work-packets.md), plus the wave 1 fix notes N1-N3 as written. Recorded as D-LTL8; the single-PR plan is D-LTL7.

| Item | Ruling | Reason | Where |
|---|---|---|---|
| N1-N3 | Accept as written | Doc and message text only; each matches the code. | Fresh packet-coder in `ltl-int` |
| D1 epoch read order | **Accept** | Reading the epoch after `events` lets an opt-out land between the two calls and tag opted-in events with the new epoch, so they are sent after consent was withdrawn (R6). Reading it first costs at most a spurious drop. Added test 7 (an `events` that bumps the epoch). | C4 `on_records`; LTL-04 Do, test 7 |
| D2 consent before `due` | **Accept, adjusted** | R6 says the queue is cleared when consent changes, not when it is next due. Adjustment: step 2 reads the target first, then the epoch, and checks the epoch only when the queue is not empty; the target read there is the one used for the send. Tests 2 and 3 pump at `t0` and assert `queued() == 0` (new `#[cfg(test)] queued()`). | LTL-04 pump steps 2-3, tests 2-3; C4 |
| D3 flaky epoch test | **Accept** | `CONSENT_EPOCH` is process-global and other engine tests bump it in parallel. Assert `after > before`. | LTL-04 test 6 |
| D4 latched `urgent` | **Accept** | A caller rule (mark urgent after the push, only if the queue is not empty; same in `pump` step 1) keeps `LauncherQueue` unchanged. Added test 8 (an opted-out `launch_finished`, then a normal event, must still wait 2 s). | C4 `on_records`; LTL-04 Do, test 8 |
| D5 `record_pumped` allow, `take_lost` | Accept | LTL-05 step 5 deletes `#[allow(dead_code)]` and the `// LTL-05 calls this` comment. LTL-04 states that steps 2-3 return without `take_lost`, by design. | LTL-05 Do 5; LTL-04 Do |
| D6 shell test failure | Accept as a process item | Not caused by the range by inspection; run once on `origin/main` before wave 2's E2E. LTL-06 reports it as pre-existing instead of fixing `host/repair`. | RESUME step 4; LTL-06 Checks; LTL-08 Do 5 |
| D7 PR F not splittable | Moot | The owner's single campaign PR (D-LTL7) ships LTL-01 only with LTL-05. Kept as a "never ship one packet alone" rule. | work-packets Dispatch |
| D8 chunk quota | Accept as a UAT note | A 429 here holds and retries; nothing is lost. | LTL-08 LTL-U1 |

Also folded in (from LTL-B1, H1 confirmed): LTL-B2 gains the two boolean mint/refresh log fields `caller_forwarded` and `peer_loopback` and two tests (fallbacks, log fields); LTL-08's LTL-U2 runs with the Windows launcher closed (`ambiguous` otherwise). LTL-B2 now branches from the `ltl-int` tip after LTL-B2b and merges before LTL-07.

Owner calls (not decided here): D-LTL3, D-LTL4, and whether the campaign PR may open without LTL-B2 while D-LTL4 is open (D-LTL7).

Ruling check: one fresh packet-reviewer, report `wave1-ruling-check.md`, scope = the N1-N3 fix commit on `ltl-int` plus the LTL-04/05 text in work-packets.md (does it close D1-D4 without new holes?).
