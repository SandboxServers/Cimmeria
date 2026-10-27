# Launcher telemetry on Linux (Steam Deck run, 2026-10-26)

> **Type:** Reference
> **Audience:** The orchestrator, packet agents and the owner
> **Last updated:** 2026-10-26 (close-out, LTL-08)
> **Companions:** [Work packets](work-packets.md), [resume card](RESUME.md), [Deck re-run UAT steps](../../guides/unified-uat.md#launcher-telemetry-on-linux-ltl-u), [B1 research](worknotes/LTL-B1.md), [launcher-telemetry ledger](../launcher-telemetry/README.md), [launch telemetry](../../../crates/launcher/desktop/docs/launch-telemetry.md)

Ledger for the fixes from the owner's 2026-10-26 Steam Deck (Proton) run. That run found two problems:

- the desktop launcher's own events were lost once the server throttled them (Packet A);
- world entry logged `client_telemetry="absent"` for the Deck, while Windows sessions log `present` (Packet B).

**Status (2026-10-26): code complete and reviewed.** Every code packet is integrated in the one campaign branch `launcher-telemetry-linux/forwarding` (worktree `ltl-int`, tip `69c6a1005`), and the campaign review's last pass said ship. The PR is not open yet: it waits for the client-tls-oidc PR (#2031) to merge, and ops has it queued. **Nothing in this campaign has run against a real Deck or the lab.** The owner's Deck re-run is [LTL-U1 to LTL-U6](../../guides/unified-uat.md#launcher-telemetry-on-linux-ltl-u).

Prefix `LTL`. Designed against `origin/main` `a26764cb6`. The packets are cut from main, **not** from the unmerged client-tls-oidc campaign (PR #2031). Every packet re-greps its anchors at its own base and returns `blocked` when one no longer holds.

## Findings

| # | Finding | Evidence (at `a26764cb6`) | Packet |
|---|---|---|---|
| F1 | The forwarder posts each burst it receives at once, so a busy phase (adoption, install, Play) posts about one chunk per record. The server allows 10 launcher chunks per minute per session. The 429 that follows holds the sender for at least 60 s. Every batch sent during the hold returns `Held` and is thrown away, and the batch that got the 429 is `Dropped`. | `shell/src/telemetry.rs:229-252` (`recv`, then drain ≤64, then send); `engine/.../launcher_session.rs:166-173`, `:213-222`; `admin-api/.../telemetry/upload_gate.rs` (`DEFAULT_LAUNCHER_CHUNK_PER_SESSION = 10`) | LTL-01, 02, 04, 05 |
| F2 | Nothing is flushed when the process exits. Records still in the channel, or in a coalescing wait once F1 is fixed, die with the process. `main.rs` has no `RunEvent` hook. | `shell/src/main.rs:670-671` | LTL-06 |
| F3 | Health counts sends, not events. `pipeline_recovered` cannot say how many events were lost. | `engine/.../health.rs:43-67`; `shell/src/telemetry.rs:214-225` | LTL-03 |
| F4 | The comment on `DEFAULT_LAUNCHER_CHUNK_PER_SESSION` says "a launcher posts about one every 2 s", which is 30 per minute, but the value is 10. | `upload_gate.rs` | LTL-07 |
| F5 | **Leading cause of `absent` (H1, unconfirmed).** The dev-session mint and refresh compute `rate_ip = rate_limit_ip(peer_ip, headers)`, but they record the telemetry stream under the raw socket peer. Behind the board Caddy's loopback proxy (the https route; CT-51 / #2030 added the auth proxy loopback overlay), that peer is `127.0.0.1`. World entry looks the stream up by the game connection's real address, finds no stream there, and logs `absent` because other streams exist. | `admin-api/src/routes/dev_session/handlers.rs:320`, `:362` (`record_unless_launcher(.., peer_ip)`), `:407`, `:481` (`record_stream(&new_claims, peer_ip)`), `:505-514`; `auth/src/auth/client_telemetry/mod.rs` lookup by `peer_ip` | LTL-B1, LTL-B2 |
| F6 | On Linux, `host_name()` reads only the `COMPUTERNAME` and `HOSTNAME` environment variables. `HOSTNAME` is a shell variable that is usually not exported, so a Deck or Linux launcher mints `machine_id = sha256("")[..16] = e3b0c44298fc1c14`, the same value on every such machine. The registry's `by_machine` index then folds every such machine into one entry (H3). | `engine/.../game_telemetry/mod.rs:44-50`, `:169-182` | LTL-B2b |
| F7 | Comment drift: the auth registry and world entry say `machine_id` is a hashed Windows `MachineGuid`. The desktop launcher actually hashes the host name, as [dev-session-telemetry.md](../../architecture/dev-session-telemetry.md) line ~402 already says. | `auth/.../client_telemetry/mod.rs:5-7`; `base-world-entry/.../map_loaded.rs:244` | LTL-B2 |
| F8 | Observation only: the server keeps at most 300 rows per launcher session for its whole life, while the new queue holds up to 500 events. A long session can still run out of that budget. This campaign does not change the budget; LTL-08 records what the Deck re-run shows. | `admin-api/.../telemetry/launcher_budget.rs` | LTL-08 (note) |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LTL1 | **The server default stays at 10 chunks per minute per session; the client paces itself instead.** The queue coalesces for 2 s, or until 64 events, and keeps at least 7 s between sends (MIN_SPACING), which is at most about 8.6 chunks a minute. A flush trigger (`launch_finished`, exit) skips the wait. LTL-07 fixes the wrong comment. | Decided (architect; the owner may revisit, and nothing blocks on it) | The limit can already be changed (`CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_SESSION`). Raising it widens an unauthenticated surface. Pacing on the client fixes the loss without any admin-api change. |
| D-LTL2 | **The hold queue lives only in memory, and consent is checked at every step.** Events are built (and consent checked) when each record is received, as today. Each push is stamped with a consent epoch, a process-wide counter that `set_game_telemetry` bumps on every change. Every pump re-reads the target and the epoch: no target, or a different epoch, withdraws the whole queue unsent. Nothing is ever written to disk (D-AU21 stands). | Decided | Keeps the rule `set_game_telemetry` already enforces for the DLL queue: events collected before an opt-out never ship after a later opt-in. The epoch closes an opt-out followed by an opt-in inside one 7 s wait. |
| D-LTL3 | **The disclosure text for the hold queue**, in the privacy-inventory row "Launcher forwarding" and the in-app disclosure (`frontend/ui/index.html:91`). Proposed: "512-record memory channel; sends are grouped every few seconds; while the server asks the launcher to wait, up to 500 events are kept in memory (oldest dropped first) and sent when the wait ends; never written to disk; turning diagnostics off empties it; default receiver 300 records/launcher session and 3,000/minute globally, configurable." | **Approved (owner, 2026-10-26).** LTL-07 shipped the text. | The privacy disclosure is owner-approved text. |
| D-LTL4 | **Record the forwarded caller in the in-memory stream registry.** The B2 fix records `rate_ip` (the rightmost `X-Forwarded-For` entry, trusted only from a loopback peer) as `StreamRecord.peer_ip` instead of the proxy's loopback address. It is held only in memory, never logged, stored or put in a field. This amends owner directive 7a (D-LT19: "never replaces the socket peer anywhere else"). D-LT18 already lets `StreamRecord.peer_ip` stay because it is never written. | **Yes (owner, 2026-10-26).** LTL-B2 implements it. LTL-B1 confirmed H1 and found no fix that avoids it ([worknotes/LTL-B1.md](worknotes/LTL-B1.md)) | D-LT19 is an owner directive. B1: the only alternative is sending the launcher back over plain http, which LI-09 forbids. |
| D-LTL5 | **Which failures retry.** A 429 or 503 (the server's hold), a failed mint and a transport or 5xx failure (`ChunkError::Http`, `Status`) give the unsent events back to the queue. A transport or 5xx failure also takes the 60 s floor hold. A 4xx rejection, or a JSON, IO or split error, drops the events, because the server would refuse them on every retry. | Decided | Retrying a permanent refusal would block the queue head. The 60 s floor keeps an offline launcher from retrying every 7 s. |
| D-LTL6 | **Exit flush.** On `RunEvent::Exit`, the shell sends a flush sentinel and waits at most 3 s (`EXIT_FLUSH_WAIT`) for the forwarder to acknowledge it. While a hold runs, the forwarder acknowledges at once without sending. | Decided | Never delays an exit by more than 3 s, and never blocks for the whole length of a hold. |
| D-LTL7 | **One PR for the whole campaign.** Every packet, LTL-B2b and LTL-B2 included, merges into `launcher-telemetry-linux/forwarding` (`ltl-int`); one PR ships it after LTL-08. The M/F/S/C split is dropped and PR #2038 closed. The sub-question (may the PR open before LTL-B2?) is answered: it waits for LTL-B2, which is now done, so the PR ships with B2 in it. The old M/F/S split and the earlier LTL-B2b PR #2038 are superseded; the owner closes #2038 himself, because ops cannot. | Decided (owner, 2026-10-26), sub-question answered | Owner instruction. It also removes the wave 1 hazard that LTL-01 alone would make an outage lose more events (wave 1 review D7). |
| D-LTL8 | **Wave 1 ruling on LTL-04 (review D1-D4).** `on_records` reads the epoch before `events` and marks the queue urgent only when it is not empty; `pump` checks consent before `due`; the epoch test asserts `after > before`. | Decided (architect; `worknotes/wave1.md`) | Closes a consent hole (events built while opted in sent after an opt-out), keeps withdrawn events out of memory during a hold, and removes a flaky test and a latched urgent flag. |

## Packets

Status words: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

All packets ship in one campaign PR from `ltl-int` (D-LTL7). Integrated means the commit is on `launcher-telemetry-linux/forwarding` and passed its packet review loop. Review-fix commits sit next to each packet in `git log`.

| Packet | Wave | What | Agent | Commit | Status |
|---|---|---|---|---|---|
| LTL-01 | 1 | `SendReport`: how many events were posted and where to retry from | packet-coder | `558ac882b` | Integrated |
| LTL-02 | 1 | `LauncherQueue`: pure coalescing and hold queue | packet-coder | `4dc22f075` | Integrated |
| LTL-03 | 1 | Health counts events; `pipeline_recovered` gains `events_lost` | packet-coder | `623d745ef` | Integrated |
| LTL-B2b | 1 | Linux `machine_id` from `gethostname`; repair the empty-name id | packet-coder | `62f74cbc3` (fix `e7bfea11b`) | Integrated |
| LTL-B1 | 1 | Research: the exact cause of Deck `absent` | network-security-auth (read-only) | none (research) | Done: H1 confirmed, [worknotes/LTL-B1.md](worknotes/LTL-B1.md) |
| LTL-04 | 2 | Engine `Forwarder`: queue, consent epoch, pump (re-specified by D-LTL8) | packet-coder | `9e793937b` (fix `2fafd8ee3`) | Integrated |
| LTL-05 | 2 | Shell forward loop on `Forwarder` + `HostSource` | rust-gameserver-dev | `3d59f77a0` (fixes `fc963ce64`, `0bcd659f8`) | Integrated |
| LTL-06 | 2 | Exit flush (`RunEvent::Exit`, 3 s) | rust-gameserver-dev | `080ded88a` (fix `2396b4b86`) | Integrated |
| LTL-07 | 2 | Docs and disclosure text; `upload_gate.rs` comment | packet-coder | `08e5cb8ba` (fix `a9a4219bf`) | Integrated |
| LTL-B2 | 2 | Record the stream under the forwarded caller; two boolean mint log fields; MachineGuid comment drift | packet-coder | `6228b2044` (fixes `33ba66ca9`, `d10f4ff24`, `a3495eab2`) | Integrated |
| LTL-08 | 3 | Close-out: UAT steps for the Deck re-run, ledger | documentation-writer | this commit | Done (docs only) |

Review trail: wave 1 fix `7ca4d250d`; wave 2 fix `a3495eab2`; campaign review fixes `de4d583c2` and `69c6a1005`; campaign review pass 3 verdict **ship** at `69c6a1005`. The reports are in `ltl-int` under `.claude/reports/launcher-telemetry-linux/`.

UAT rows owed after deploy: **LTL-U1 to LTL-U6** (status UATPending, owner only).

### What is not tested

- No live Steam Deck run of this build. Every behavior claim about the Deck rests on unit tests, code review and the [B1](worknotes/LTL-B1.md) reading of the earlier Deck run.
- No lab run.
- The proxy hop itself (that the container sees Caddy as loopback, ops item OPS-LT2) can only be shown by the new `caller_forwarded` and `peer_loopback` log fields after deploy.
- The 300-row server budget under a long session ([F8](#findings)) is not changed or measured here.

### Known flake

The shell test `host::repair::integration_tests::retained_host_handoff_commits_signed_seed_and_reports_current_backup_after_cleanup` fails intermittently with `Err(Busy)` (`integration_tests.rs:173`): it failed in 2 of 3 full shell runs in the wave 2 review and passes when run alone (wave 1 review 2 D6, wave 2 reviews 1 and 2). The campaign's range does not touch `host/repair`, `NativeHost` or anything it calls; the `Busy` after drop-and-recreate looks like a lock-release race in the host. It was **not** run on a clean `origin/main` tree, so "also fails on main" is unconfirmed, and no issue number was filed. If it fails in the PR's CI, rerun that job; do not read it as this campaign's fault.

## Conflict surface with PR #2031 (client-tls-oidc, open)

The PR from this campaign opens after #2031 merges, so ops rebases or merges `origin/main` into `ltl-int` first and keeps both sides of any conflict.

| This campaign | File | In #2031? | Handling |
|---|---|---|---|
| LTL-06 | `crates/launcher/desktop/shell/src/main.rs` (the `.run(..)` tail, about 6 lines) | yes | Keep the hunk at the tail only. Whichever PR merges second is rebased by `ops`; resolve by keeping both changes. |
| LTL-07 | `crates/launcher/desktop/docs/privacy-inventory.md` (row "Launcher forwarding") | yes | One row only. If #2031 merged first, the coder re-greps the row. |
| LTL-07 | `crates/launcher/desktop/frontend/ui/index.html` (line 91, the forwarding `<li>`) | yes | One `<li>` only. The main checkout also has an uncommitted edit to this file that is not this campaign's; packet worktrees come from `origin/main`, so it does not reach them. |
| all others | `engine/src/storage/game_telemetry/*`, `shell/src/telemetry*.rs`, `shell/src/host/launcher_telemetry.rs`, `crates/admin-api/src/routes/dev_session/*`, `crates/auth/src/auth/client_telemetry/mod.rs`, `crates/base-world-entry/.../map_loaded.rs` | no | #2031 touches other `crates/auth/src/auth/*` files and other engine files (`lib.rs`, `storage/mod.rs`, `sign_in/*`, `storage/launch/*`), none of these. It touches no admin-api file. |

## Parked (not sliced)

| Item | Why parked | Who |
|---|---|---|
| Deck walk test for movement | Needs the owner at the Deck | Owner (UAT) |
| First-run `launcher_started` consent question: is `launcher_started` sent before the player has chosen? | Needs a consent-design read against D-AU2/D-LT2 first | Future campaign |
| `CombatReticule00` load failure on the Deck | Client content and Proton, not telemetry | Future triage |
| Windows Lua `ScriptException`s seen in the same window | Separate client bug | Future triage |
| Adding `os` to game-client events | Schema and privacy change on the DLL rows | Future campaign (owner) |
| Optional new Proton and prerequisite launcher events | New event vocabulary; needs a design | Future campaign |

## Review plan

- Packet loop for every packet: packet-reviewer, at most 3 passes. LTL-B2 also gets a parallel `network-security-auth` pass.
- Wave 1 loop on the `ltl-int` tip after LTL-02, 01 and 03 have merged in. Pass 1 returned `decision`; the ruling is `worknotes/wave1.md`. A fresh packet-coder applies N1-N3 in `ltl-int`, then one reviewer runs the ruling check. LTL-B2b had its packet loop (`ship`) and then merges in.
- Wave 2 loop on the `ltl-int` tip after LTL-07, covering LTL-04 to 07 and LTL-B2. It checks explicitly:
  - D-LTL2's consent rules: only events built while opted in enter the queue; the epoch and the target are re-read at every pump; nothing is written to disk;
  - the 3 s exit bound;
  - that `TelemetryStatus` is unchanged;
  - the #2031 hunks.
- Campaign loop on the `ltl-int` tip after wave 2, diffed against `origin/main` (there is no earlier merge to main; D-LTL7). Its fixes are committed in `ltl-int`, then LTL-08, then the one PR.
