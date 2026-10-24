# Architect report: launcher-telemetry

Date: 2026-10-21. Status: **done** (design only, no code).

## Outputs

- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\launcher-telemetry\README.md`: purpose, findings F1-F19, decisions D-LT1..D-LT12, open owner questions, packet status, and PRs.
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\launcher-telemetry\work-packets.md`: dispatch rules, branches, merge order, rules R1-R10, the check table, contract C1-C13, and waves 0-2 (LT-00..LT-11).
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\launcher-telemetry\work-packets-coverage.md`: waves 3-5 (LT-12..LT-27).
- `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\launcher-telemetry\RESUME.md`: the resume card.

## Root cause of "no launcher rows in SigNoz"

These are stacked failures, and every one is silent:

1. On `main` the launcher has no tracing subscriber at all (F1).
2. AU 2b, still unmerged, forwards only `adoption` (F2).
3. A server without `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` advertises localhost:8443, and the launcher accepts loopback (F3).
4. Server refusals log nothing (F4).
5. The summary endpoint is `None` and https-only, while the colo is http (F13).

LT-00 will confirm item 3 from SigNoz aggregates. **Quick win, no code (OQ-LT5):** set the colo env var now.

## Overlap and sequencing

- **launcher-adopt-ux.**
  - The LT server work stacks on AU 2a (#1954) for `launcher_rows.rs` and the `dev_session` edits.
  - The LT launcher work stacks on AU 2b (`launcher-adopt-ux/telemetry`) for the subscriber, the `ForwardLayer` and the `LauncherSender`.
  - The LT launcher PR therefore reaches `main` after AU's held wave 1 and 2b (D-LT8).
- **oidc-login.** Research only. The campaign reserves `client.launcher.login` and writes no packet.

## Waves and PRs

| Wave | Packets | Ships as |
|---|---|---|
| 0 | LT-00 SigNoz triage (general-purpose, read-only) | a worknote |
| 1 | LT-01..LT-04 (server) | one PR S from integration `launcher-telemetry/server` |
| 2 | LT-05..LT-11 (pipeline) | into integration `launcher-telemetry/launcher` (PR L) |
| 3 | LT-12..LT-17 (subsystems A, parallel with wave 2 after LT-05) | into PR L |
| 4 | LT-18..LT-23 (subsystems B, summaries) | into PR L |
| 5 | LT-24 (PR F), LT-25 and LT-26 (into PR L), LT-27 (PR X) | as listed |

The merge order per wave is in work-packets § Merge order. Into `main`: #1954 → PR S → colo deploy → PR F → AU → PR L → PR X.

Wave 2 has 7 packets; LT-11 is the dependent tail. LT-01 and LT-02 get a parallel network-security-auth pass (Host-header trust, refusal-log content).

## Owner decisions (BlockedDecision)

- **D-LT2 / OQ-LT1, consent semantics.** Default A, applied: launcher events ride on "Send game diagnostics", and "Share setup diagnostics" gates only the anonymous summaries. This blocks nothing.
- **D-LT4 / OQ-LT2, plain-http summaries to the first login server.** The docs say https is required. This blocks LT-22 and LT-23 only.
- **D-LT8 / OQ-LT4, timing.** The LT launcher ships with the AU release (default). This blocks nothing.
- **OQ-LT3.** The existing dev-session refusal logs put the peer IP into SigNoz. Keep, hash or drop it? No packet is written until this is decided.
- **OQ-LT5 (ops/owner).** The colo `.env` value of `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`.

No new opcode or game wire. `ui_event` and `telemetry_status` are local Tauri commands. The service name is already decided (D-AU13).

## Requests for ops or the orchestrator

1. Whether PR #1954 has merged, which picks the base for `lt-srv-integration`.
2. The current tip of `launcher-adopt-ux/telemetry` (`au-tel-integration`), which is the base for `lt-app-integration`.
3. The board subcategory `launcher-telemetry` (`board campaign create`).
4. OQ-LT5: read the colo `.env` for `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` (yes/no/loopback only; never commit the value).

## Assumptions packets verify first (each says "grep …, else blocked")

- The exact `IngestError` variants and their status mapping (LT-02).
- An existing mint router test to copy (LT-01).
- Existing failing-path tests for updater, catalog, install worker, repair and launch, used for the R7 real-path assertions.
- Where the worker's `Event::Unknown` JSON type lives (LT-18).
- Whether `launcher-core` already has `tracing-subscriber` (LT-14).

## Amendment 2026-10-21

Folded in the owner's four directives ([owner-directives.md](owner-directives.md)). No code.

- **1. Default upload URL.** OQ-LT5's env-var quick win is replaced by code. LT-01 deletes the localhost fallback (`handlers.rs:512-521`): Host derivation, then a built-in `DEFAULT_UPLOAD_ENDPOINT`; an unset env is INFO, a loopback env WARNs. The launcher gets `effective_upload_endpoint` (C10, LT-05) and replaces, not refuses, a loopback endpoint (LT-07, LT-08; D-LT7 amended, `endpoint_misconfigured` removed). The env var stays an override; OQ-LT5 is now verification U1. D-LT4 is decided: http summaries when the login server is http.
- **2. No dead UI.** Checkbox audit F20 / D-LT13. `#telemetry` is fixed by LT-22/23 (now BlockedDependency, not BlockedDecision). `#adoption-telemetry` is a real gate with true copy, so LT-11 no longer edits `index.html:45` (F14 corrected). The other boxes are real.
- **3. Adoption refusals say why.** Wave 5, PR A, LT-28..LT-34 in [work-packets-adoption.md](work-packets-adoption.md): C14 `Rejection` record (check, entry, expected, found, http_status), `adoption_rejected` WARN on the `adoption` target (forwarded by AU-08), status `last_rejection`, a frontend text per check, and unknown codes show their code instead of platform-unavailable. R5a / D-LT14 allows folder-relative paths. Ownership D-LT16: AU keeps unsafe-file refusals, step events, the subscriber and the mode UI; LT owns every other reason plus AU F17 (`TooLarge`).
- **4. Publish the file index.** Wave 6, PR I: LT-35 `verify-file-index` (validate, ref size and digest, parse_verified, bind, append-only chain against the current manifest, parity with a local build: the launcher-verify step), LT-36 prepare script and runbook (stops before signing). LT-37 is the owner's signing and upload, **BlockedDecision D-LT15** (also picks the content release). LT-30 makes the silent fallback (`verification.rs:595-609`) a reason on screen (C15), a log line and telemetry.

**Owner decisions now open:** D-LT2 (default A), D-LT8 (timing), OQ-LT3 (peer IPs), D-LT15 (sign and publish). D-LT1 unchanged.

**Requests for ops:** the `launcher-adopt-ux/integration` tip and when AU wave 1 is final (base of `lt-adopt-integration`, created by merge-noff after LT-05 lands); whether AU-09 has merged into `launcher-adopt-ux/telemetry`; worktrees `lt-index-integration` and `lt-35` now.

**Correction:** the brief's `launcher-adopt-ux/.../campaign-close.md` does not exist; the file-index note is `docs/analysis/any-install-adoption/worknotes/campaign-close.md:52-54` (AI-14, D-AI8).

## Amendment 2

Folded in owner directives 5 and 6 (2026-10-21). No code.

- **5. https only (D-LT4 decided).**
  - The colo has no https telemetry endpoint on record (README F23: the login port :8081 is http; the tunnel is optional and was not running; the auth TLS listener has no certificate configured). So the built-in defaults (server and launcher `DEFAULT_UPLOAD_ENDPOINT`, launcher `DEFAULT_SUMMARY_BASE`) ship as `None`. New **LT-41** (PR H, **BlockedDecision D-LT17**) sets them once the owner names the https origin and `ops` confirms it serves the routes. There is no http fallback.
  - LT-01 is re-specified: an https-only resolution (C11), 503 `no_upload_endpoint` when nothing resolves, `loopback_dev` for a one-host dev box, and no URL in logs for a Host-derived source.
  - C10/LT-05/07/08: the loopback replacement is https or `None` (`MintFailed { reason: "no_https_endpoint" }`).
  - LT-22/23 are unblocked (BlockedDependency only). The box stays clickable and shows "saved but not sent yet: no secure (https) address" on the first press (directive 2).
  - F13, D-LT3, D-LT7, LT-04 and LT-26 no longer assume "the colo is http".
- **Interpretation.** An explicit `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` is still used as given, even if it is http, with a boot WARN `upload_endpoint_plain_http`. Directive 1 keeps the override, and directive 5 covers the default. The owner can overturn this; if so, LT-01 refuses a non-https env at boot.
- **6. No IP (OQ-LT3 decided, D-LT18).**
  - The old LT-02 spec was stale: chunk and bundle refusals already log on `main` through `RefusalLog`, with a peer (F4 corrected). LT-02 now deletes the peer from those rows and from `admin.request` on telemetry and dev-session paths.
  - New **LT-38** drops the peer from the seven dev-session refusal arms and throttles them per (route, reason) with a `suppressed` count. New **LT-39** adds summary refusal rows that carry the reason and status only.
  - New **LT-40** (first in wave 3) adds the launcher `scrub_ips` at the forwarder, and makes the C9 capture helpers panic on an IP. LT-21 scrubs the uploaded logs.
  - Every one of these packets has a TEST-NET regression test that fails if the IP appears.
  - In-memory quota keys and `StreamRecord.peer_ip` stay, because they are never written.
- **Out of scope (noted, no packet).** The SOAP game-login rows in `crates/auth/src/auth/handlers.rs` still log `peer_ip`; they are game login, not launcher or telemetry. Replayed game-DLL rows get no IP scrub. Either could become a follow-up if the owner reads directive 6 more widely.
- **Wave sizes.** Wave 1 now has 6 packets, and wave 3 has 7 (LT-40 plus LT-12..LT-17).

**Owner decisions now open:** D-LT2 (consent, default A applied), D-LT8 (timing, default applied), D-LT15 (file-index signing and release), D-LT17 (the colo https origin; blocks LT-41 only). OQ-LT3 and D-LT4 are closed; D-LT17 is the one new decision.

**Requests for ops:** run the LT-41 unblock check against the colo (an https `HEAD`/`POST` status code for `/api/telemetry` and `/api/telemetry/launcher-summary`; never record the address); worktrees `lt-39` now and `lt-38` once LT-01 is in `lt-srv-integration`; `lt-40` after LT-05.

## Amendment 3

Re-sequenced on real bases after the ops pre-flight (2026-10-21). No code.

**What actually happened to AU (git and `gh`, read-only).**
- PR #1954 (AU 2a, `launcher-adopt-ux/launcher-session`) was closed unmerged at 15:17Z with the comment "Superseded by #1953, which now carries wave 2a/2b".
- PR #1953 (head `launcher-adopt-ux/integration`) **merged** at 15:48Z as squash commit `0bd5a2aec` ("... opt-in launcher telemetry (AU-01..16)"). Its body lists wave 1, wave 2a, and "Wave 2b + AU-09". The head branch was deleted on merge, which is why `launcher-adopt-ux/integration` no longer exists.
- `git diff 6bc891ee7 0bd5a2aec` over `crates/launcher`, `crates/admin-api` and `crates/launcher-core` touches only three test files (`launcher_kind_tests.rs` and the shell `game_update` / `repair` integration tests), which #1953 fixed further. So the `au-09` tip, and the `launcher-adopt-ux/telemetry` tip `735f24b44` behind it, hold nothing that `main` lacks. `735f24b44` is not an ancestor of `main` only because of the squash.
- `origin/main` (`5079be48f`) is 9 commits past `0bd5a2aec`; only #1966 touches a file LT names (one line of `docs/architecture/observability.md`, which LT-04 edits; no conflict expected). No open PR has a launcher, telemetry, oidc or admin head branch.

**What is on `main` for wave 1 and LT-05.** `routes/telemetry/launcher_rows.rs` (`LAUNCHER_TARGETS: [&str; 1]` at :17), `launcher_gate.rs`, `launcher_budget.rs`, `dev_session/launcher_kind.rs`; `dev_session/handlers.rs` still has the localhost `DEFAULT_UPLOAD_ENDPOINT` (:31), `log_refusal` with `peer` (:445-501) and `upload_endpoint_env` (:515-524); `refusal_log.rs` still writes `peer = %who.peer` at :332, :375, :401; `request_span.rs` still writes `peer`. On the launcher: `ForwardLayer` (`shell/src/telemetry.rs:114`, `FORWARD_TARGET = "adoption"` :26) and `LauncherSender` (`launcher_session.rs:69`). `launcher_targets.txt` does not exist yet. Every finding the wave-1 and LT-05 specs fix is still present, at the anchors now in work-packets § Amendment 3. No wave-1 packet touches an `otel.rs`; client rows go through `routes/telemetry/replay.rs` (D-LT1).

**What AU 2b still lacks.** Nothing LT depends on: it is merged. F1, F2 and F6 now describe `main`.

**Changes to the ledger.**
- Every integration branch (`launcher-telemetry/server`, `/launcher`, `/file-index`) is created from one recorded `origin/main` SHA. `lt-adopt-integration` comes from `lt-app-integration` after LT-05, with no AU merge. Wave 4's pre-merge is `origin/main`, not the AU 2b tip.
- LT-03 is Ready, with no AU condition; it blocks only if `LAUNCHER_TARGETS` is not `[&str; 1]`. No slice of AU had to be pulled into an LT packet.
- Ready now: LT-00, LT-01, LT-02, LT-03, LT-39 (wave 1 server, one PR S), LT-05, LT-35. LT-38 stays BlockedDependency on LT-01; LT-28 on LT-05 only.
- Merge order into `main`: PR S → colo deploy → PR F; PR I any time; PR A → PR L → PR H → PR X. The "#1954 first" and "AU wave 1 and 2b first" steps are gone. LT-33 builds on AU-09 directly.
- A line-anchor table in work-packets § Amendment 3 supersedes the drifted line numbers in LT-01, LT-38 and LT-03 (LT-01: `mint` :214, mint INFO :307, response fields :319 / :419, `log_boot_status` :130-145, the default pin `session_kind_tests.rs:153`; LT-38: `inspect_err` :226 / :242, fixed peers `tests.rs:472`, `:510`, `:553`).
- README: the AU section, F1/F2/F3/F5/F6 evidence, D-LT8 (now moot), OQ-LT4, the packet rows for LT-03, LT-05 and LT-28, and the PR table. RESUME rewritten.

**Unchanged.** D-LT17 stays BlockedDecision: `docs/operations/telemetry.md` documents only plain http :8081, and https only as a Cloudflare Tunnel placeholder. So every https default (server and launcher `DEFAULT_UPLOAD_ENDPOINT`, `DEFAULT_SUMMARY_BASE`) stays `None`, and the LT-41 status-code check is not run until the owner names an origin.

**Owner decisions now open:** D-LT2 (consent, default A applied), D-LT15 (file-index signing and release, blocks LT-37), D-LT17 (colo https origin, blocks LT-41). D-LT8 is closed as moot; D-LT4 and OQ-LT3 stay decided.

**Requests for ops or the orchestrator.**
1. Worktrees from one recorded `origin/main` SHA: `lt-srv-integration`, `lt-app-integration`, `lt-index-integration`, `lt-01`, `lt-02`, `lt-03`, `lt-39`, `lt-05`, `lt-35`; then `lt-38` after LT-01 is in `lt-srv-integration`. `board campaign create "launcher-telemetry"` if it is missing.
2. Whether the colo has had a deploy since #1953. Without one, LT-00 reads zero launcher rows, which is expected and not a finding.
3. For the launcher-adopt-ux ledger, not LT: retire `au-tel-integration`, `au-09` (and `au-srv-integration` if present) with `rm-worktree.ps1 --merged-into 1953 ...`, and refresh the stale AU RESUME (it still says #1953 is a held draft).
## Amendment 4 (2026-10-21, owner directive 7)

**Input.** Directive 7: hands-on stood up `https://play.cimmeria.app` on the board's Caddy proxy (`board-proxy`), forwarding only the five telemetry routes (dev-session mint and refresh, upload-chunk, upload-bundle, launcher-summary) to loopback :8081; everything else 404; http :8081 unchanged.

**Changed.**
- **D-LT17 decided:** the origin is `https://play.cimmeria.app`. LT-41 moves from BlockedDecision to BlockedDependency (PR S and PR L merged, then an `ops` status-code probe). The probe is three `curl.exe` calls from the workstation: POST mint, POST launcher-summary (an HTTP status, not a TLS or connect error), and GET `/` (404). It needs no lab, ssh, SigNoz or credentials, sends no body and mints nothing. LT-41's constants now name the origin.
- **New LT-42 (wave 1, PR S, contract C18, D-LT19).** `forwarded_for::rate_limit_ip(peer, headers)`: the rightmost `X-Forwarded-For` entry when the socket peer is loopback, otherwise the peer. It is used only inside `ip_key(..)` for the summary, chunk, bundle, mint, refresh and bad-refresh quotas and the per-peer upload slot (`upload_gate.rs:329`, `:342`; `launcher_summary/handlers.rs:171`; `dev_session/handlers.rs:250`, `:345`, `:355`). It is never logged, stored or put in a field. Tests: a unit table, per-client allowance tests (summary, upload gate chunk and bundle, mint), spoof tests (a non-loopback peer's header is ignored), and an IP-leak regression with a proof that it can fail. LT-42 branches after LT-01, LT-02, LT-38 and LT-39 merge, because it edits lines next to theirs in `dev_session/handlers.rs`, `mod.rs`, `launcher_summary/handlers.rs` and the `login_port.rs` doc comment. LT-01, LT-02, LT-03, LT-38 and LT-39 are unchanged. It gets a network-security-auth pass. Wave 1 is now seven packets in one PR: LT-01 → LT-02 → LT-03 → LT-39 → LT-38 → LT-42 → wave-1 review → LT-04 → PR S.
- **LT-04 widened** (it was BlockedDependency and already owned `docs/operations/telemetry.md`, so no second packet edits that file). It now deletes the never-deployed Cloudflare Tunnel steps (~145, ~165-202), describes the Caddy route and its rollback, and rewrites the proxy-quota caveats (~382-401, ~703-711) for LT-42. It depends on LT-42.
- **LT-27** gains U10: no `mint/ip` quota refusals or summary 429s through the https origin at normal load.
- **Owner and ops tasks (not packets):** OPS-LT1 mirrors the host Caddyfile edit into the private agent-board repo before any `board-proxy` redeploy (rollback: newest `Caddyfile.bak-*`, `docker restart board-proxy`). OPS-LT2 checks, after the PR S deploy, whether the server container sees Caddy as loopback (yes/no, no addresses).

**New findings.**
- F24: every per-IP counter keys on the socket peer, so all https callers share the proxy's bucket.
- F25: `docker/compose.yml:45-47` publishes 8081 from a bridge-network container. Loopback-to-published-port traffic usually reaches the container from the bridge gateway, so the loopback-only rule may do nothing on the colo. This is unverified (OPS-LT2, U10). If so, the owner picks under D-LT20 (an explicit trusted-proxy list, host networking, or a raised quota).
- F26: `client_telemetry::lookup`'s fallback by address (after the account match) cannot match a mint made through the proxy. Directive 7(a) limits the header to quota keys, so this is D-LT21 with the default "no change".

**Owner decisions now open:** D-LT2 (default A), D-LT15 (blocks LT-37), D-LT20 (raised only if OPS-LT2 or U10 shows it), D-LT21 (default no; blocks nothing). D-LT17 is closed.

**Requests for ops or the orchestrator.** Create `lt-42` from the `lt-srv-integration` tip after LT-01, LT-02, LT-38 and LT-39 merge. Run OPS-LT1 and, after the PR S deploy, OPS-LT2. Run the LT-41 probe before dispatching LT-41.

## Amendment 5 (2026-10-21, LT-09 re-spec)

**Input.** The LT-09 Haiku coder returned blocked with no edits (`lt-09\.claude\reports\launcher-telemetry\LT-09-coder.md`). The packet listed 14 sites but named only 5 events. It also gave no source for `attempt` and no way to get `http_status`, and it left `bound_s` outside C6. Owner directive: "wire all launcher telemetry, nothing silent".

**Result.** LT-09 is split into two packets, both `Host: windows`, both about 40k working tokens, specified in work-packets § LT-09a and § LT-09b:
- **LT-09a:** the transport and its queue. It reuses the unedited `lt-09` worktree and branch.
- **LT-09b:** the session loop, `install_event.rs` and `patch_events.rs`. It needs a new `lt-09b` worktree, branched after LT-09a merges, because it calls `transport::events::reason`.

New wave-2 order: LT-05 → LT-06 → LT-07 → LT-08 → LT-09a → LT-09b → LT-10 → LT-11 → wave-2 review.

**Decisions.**
- **`attempt`.** Core does not retry a chunk upload: `flush.rs` posts each chunk once and stops at the first error, and `chunk.rs` has no loop. So `attempt` is the number of consecutive failed flushes, counted by a `Streak` in `transport/events.rs`.
  - The first failure is forwarded at warn. Repeats stay local at debug.
  - The first flush that sends something logs `chunk_upload_resumed` (info, `count`).
  - The proactive token refresh uses the same pattern.
  - `chunk_upload_gave_up` is dropped, because nothing is ever given up: events stay queued. At game exit, LT-09b logs `final_flush_failed`.
  - No core file is touched.
- **`http_status`.** It is read at the call site from the raw `TelemetryError` by `events::http_status() -> Option<u16>`, which binds `..` so no body is ever read. The public `TransportError` is not widened.
  - `Status` and `Rejected` give their own status.
  - `TokenRejected` gives 401, `RateLimited` 429 and `KillSwitch` 503.
  - Anything else gives no key.
- **`queue_dropped`.** It fires once, at the bundle point in `upload_bundle`'s Ok arm when `dropped > 0`, before the counter reset. A reset failure is its own event, `dropped_counter_reset_failed`.
- **`bundle_upload_failed`.** This is a new warn, because a failed bundle used to log nothing at the transport. The session loop's own "bundle not sent" drops to debug, so the failure is forwarded once.
- **No C6 change.**
  - The previous-session bound is now `duration_ms` (it was `bound_s`).
  - The session finish uses `count` (it was `events`), `bundle_sent` and `exit_observed`.
  - `exit_code` is `Option<i64>`, so the key is absent when no exit was observed.
  - No pid is logged.
- **No silent site.**
  - Per-event queue failures in the session loop are counted in a new `SessionEnd.not_queued`. The first one warns and the rest log at debug, and a `session_events_lost` row is written at the end.
  - `patch_events` failures are counted and warned once per batch.
- **Test fix the coder would have hit.** The capture filter at `transport/tests.rs:793` keeps only `cimmeria*` targets. LT-09a widens it to `launcher.*`, or the existing no-server-text test would see nothing. LT-09a keeps the substrings that test asserts.

**Site map.** Every event is on `launcher.telemetry`. "warn → debug" means the first failure in a run warns and repeats log at debug.

| Site at `53acbc2dd` | Packet | Event | Level |
|---|---|---|---|
| transport :272 | 09a | `token_renew_started` | info |
| transport :292 (`sent > 0`) | 09a | `chunk_upload_resumed` | info |
| transport :299 | 09a | `chunk_upload_failed` | warn → debug |
| transport :341 (new) | 09a | `queue_dropped` | warn |
| transport :344 | 09a | `dropped_counter_reset_failed` | warn |
| transport :349 (new) | 09a | `bundle_upload_failed` | warn |
| transport :375, :381, :406, :413 | 09a | `token_refresh_failed` (`action` = remint, refresh, renew, upload_after_renew) | warn |
| transport :386 | 09a | `token_refresh_failed` (`action` = refresh_proactive, `held=false`) | warn → debug |
| transport :441 | 09a | `session_reminted` | info |
| queue :93, :168 | 09a | `queue_discard_failed` | warn |
| queue :163 | 09a | `queue_discarded` | info |
| session_loop :179 | 09b | `previous_session_stopped` | warn |
| session_loop :266 | 09b | `bundle_not_sent` | debug |
| session_loop :268-271 | 09b | `final_flush_failed` (a reason, or `timeout`) | warn |
| session_loop :272 | 09b | `game_session_finished`, plus `session_events_lost` | info, warn |
| session_loop :284 | 09b | `game_session_withdrawn` | info |
| session_loop :337 | 09b | `client_patches_read_failed` | warn |
| session_loop :359 | 09b | `event_queue_failed` | warn → debug |
| install_event :64, :84 | 09b | `install_result_not_queued` | warn |
| install_event :75 | 09b | `install_result_queued` | info |
| patch_events :107 | 09b | `client_patch_events_not_queued` (once per batch) | warn |

**Conflict check against the in-flight packets.**
- **LT-05:** merged as `53acbc2dd`, which is the base of both packets.
- **LT-07 and LT-08:** they edit `game_telemetry/mod.rs`, `launcher_session.rs`, `health.rs`, `session.rs` and possibly `transport/mod.rs::minted()`. LT-09a and LT-09b do not edit `game_telemetry/mod.rs`, `session.rs` or `minted()`, and LT-09a touches `remint()` only at its log line (:441). If LT-08 has edited near `minted()` by the time LT-09a merges, ops resolves the textual conflict in favour of both changes.
- **LT-06 and LT-10:** shell files only.
- **LT-40:** owns `test_capture.rs` and the forwarder. Neither packet edits them, and their tests hold no IP literal.

**Follow-up (not packeted).** The `launcher-core` telemetry logs (`flush.rs`, `runner.rs`, `queue.rs`) still use module targets and are never forwarded. That is outside LT-09's file set. It could be a wave-4 packet, or part of LT-21 if its scope allows, and the wave-2 review should rule which.

**Requests to ops and the orchestrator.**
1. Re-dispatch LT-09a to packet-coder in the existing `lt-09` worktree. This is a fresh dispatch, not retry 1/3.
2. After LT-09a merges into `lt-app-integration`, create `lt-09b` (branch `launcher-telemetry/lt-09b-session-events`) and dispatch LT-09b.

**Owner decisions.** None new.

**Files changed.**
- `work-packets.md`: the amendment 5 note, the dispatch rule, the wave-2 merge order, the C5 row, and § LT-09a and § LT-09b, which replace § LT-09.
- `README.md`: row F8, and the packet table, where LT-09 becomes two rows.
- `RESUME.md`: rewritten to 40 lines or fewer.
