# Launcher telemetry

Campaign ledger. Packets: [work-packets.md](work-packets.md) (rules, contract, waves 0-2), [work-packets-coverage.md](work-packets-coverage.md) (waves 3, 4 and 7; amendment 6 moved LT-14a/b to [work-packets-install.md](work-packets-install.md) and LT-16a/b, LT-17a/b to [work-packets-repair-launch.md](work-packets-repair-launch.md)) and [work-packets-adoption.md](work-packets-adoption.md) (waves 5 and 6). Resume card: [RESUME.md](RESUME.md). Designed 2026-10-21. Amended the same day for the owner's directives 1-4, and again (amendment 2) for directives 5 (https only) and 6 (no IP in logs) ([owner-directives.md](owner-directives.md)). Amendment 3 re-based on `main`; amendment 4 takes in directive 7 (the colo https origin exists: D-LT17 resolved, LT-41 unblocked, new LT-42, LT-04 widened, ops task OPS-LT1). Wave 8 (light adoption, incremental update, the index comparison and the LT-37 completion, LA-01..LA-20) follows the owner's answers of 2026-10-21: design [light-adoption-proposal.md](light-adoption-proposal.md) (v2), packets [work-packets-light-adoption.md](work-packets-light-adoption.md).

## Purpose

Owner directive (2026-10-21): wire in all launcher telemetry, add it wherever the launcher has none, and leave no dead UI. A launcher problem must be debuggable from SigNoz alone.

Scope:

1. Make the second checkbox ("Share setup diagnostics") send something real: the launcher-summary endpoint, server ingest into SigNoz, and tests.
2. Ship the launcher's own structured events to SigNoz, honouring consent and privacy (no IPs, credentials, account names or absolute paths). Cover startup, update check, install/adopt/repair, login and OIDC, settings, errors, and the health of the telemetry pipeline itself.
3. Remove the silent localhost default: the server and the launcher both get a built-in Cimmeria default (directive 1).
4. Audit every launcher subsystem for missing telemetry, and turn each gap into a packet.
5. Every adoption refusal or fall-through names the check, the entry and expected vs found, in the UI, the log and telemetry (directive 3).
6. Publish the adoption file index under the signing and chain rules, and make its fallback visible (directive 4).
7. Add a SigNoz verification (saved view and dashboard) and a REPL-style logic UAT.
8. Summaries and the default upload URL are https, with no plain-http path (directive 5). No launcher or telemetry log, field or attribute holds an IP, and nothing is hashed (directive 6).

## How it builds on launcher-adopt-ux

This campaign extends [launcher-adopt-ux](../launcher-adopt-ux/README.md) (AU) wave 2. It does not replace it.

**Amendment 3 (2026-10-21): all of AU is on `main`.** PR #1953 (head `launcher-adopt-ux/integration`, since deleted) squash-merged at 15:48Z as `0bd5a2aec`, carrying AU wave 1, wave 2a, wave 2b and AU-09 together. #1954 was closed unmerged at 15:17Z with "Superseded by #1953". The tree of `0bd5a2aec` equals the `launcher-adopt-ux/au-09` tip `6bc891ee7` except for three test files that #1953 fixed further, so the `launcher-adopt-ux/telemetry` (`735f24b44`) and `au-09` branches hold nothing that `main` lacks. Every LT base is now `origin/main`.

- **AU wave 1** (in #1953) adds `UnsafeReport`, `Error::Unsafe` (so `Error` is no longer `Copy`) and status `unsafe_entries`.
- **AU 2a (server)** (in #1953) adds the `launcher` session kind and `launcher_rows.rs`, and it accepts exactly one target, `client.launcher.adoption` (`launcher_rows.rs:17`, `LAUNCHER_TARGETS: [&str; 1]`).
- **AU 2b (launcher)** (in #1953) adds the first tracing subscriber the launcher has ever had (a rolling file plus a `ForwardLayer`, `shell/src/telemetry.rs:114`, with `FORWARD_TARGET = "adoption"` at :26), a `LauncherSender` (`engine/src/storage/game_telemetry/launcher_session.rs:69`) that mints a 15-minute session and posts at once, and (AU-09) the six adoption step events.
- AU's owner decisions stand: D-AU7 (relative paths only), D-AU13 (service `cimmeria-client`), D-AU19 (session kind), D-AU20 (server row rules), D-AU21 (no disk queue), D-AU22 (gate), D-AU24 (caps).

Because of that:

- The LT server packets branch from `origin/main` (amendment 3).
- The LT launcher packets branch from `origin/main`; PR L has no AU prerequisite left.
- Wave 5 (adoption reasons) stacks on LT-05 only, because AU wave 1 and AU-09 are already on `main`. The split of adoption work between AU and LT is D-LT16.

[oidc-login](../oidc-login/) is research only; it has no code and no packets. This campaign only reserves the target `client.launcher.login` for it (D-LT5).

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | On `main` the launcher installs **no tracing subscriber**, so its ~150 `tracing::*` calls go nowhere. The release build is windowed, so `eprintln!` is invisible too. AU-08 added the first subscriber; it reached `main` in #1953 (amendment 3). | Pre-#1953 grep over the launcher crates: no `tracing_subscriber`; `shell/src/main.rs:4` `windows_subsystem="windows"` | LT-05 |
| F2 | Even with AU-08, the forwarder ships only the target `adoption`. Its file filter keeps only module-path targets, so the 24 structured `launcher.update` events never reach the file either. | `main` `shell/src/telemetry.rs:26` (`FORWARD_TARGET = "adoption"`) and the filter in the same file (was au-tel `:85-90`, `:127`; re-grep); engine `storage/updater/apply/zip_swap.rs:21` | LT-05 |
| F3 | **Silent localhost default.** With `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` unset, every token advertises `http://localhost:8443/api/telemetry`. The boot line logs at INFO, the mint INFO omits the endpoint, and the launcher's endpoint policy accepts loopback. So a remote player's launcher uploads to their own machine and fails without a word. | `main` at `5079be48f`: `dev_session/handlers.rs:31`, `:307`, `:515-524`; `dev_session/mod.rs:130-145`; `game_telemetry/session.rs:135` | LT-01, LT-05, LT-07, LT-08 |
| F4 | **Corrected (amendment 2).** Chunk and bundle refusals are already logged on `main`, by a throttled `RefusalLog` (`upload_refused` / `upload_auth_refused` on `launcher.ingest`), but each row writes the peer IP (F16). Whole-request launcher-summary refusals still **log nothing**. | `routes/telemetry/refusal_log.rs:301-405`; `launcher_summary/handlers.rs` has no `tracing::` call | LT-02, LT-39 |
| F5 | The server accepts only `client.launcher.adoption`. | `main` `telemetry/launcher_rows.rs:17` `LAUNCHER_TARGETS: [&str; 1]` | LT-03 |
| F6 | `LauncherSender::send` returns a `SendOutcome` (`Empty`/`Sent`/`Held`/`MintFailed`/`Dropped`) that the forwarder ignores. Nothing records the health of the pipeline. | `main` `game_telemetry/launcher_session.rs:69` | LT-07 |
| F7 | Every game-session mint failure collapses to `SessionUnavailable` or `EndpointRefused`, with no HTTP status and no log. | `game_telemetry/session.rs:87-136` | LT-08 |
| F8 | The game-telemetry transport, queue and session loop log free strings with `?code`. Tail, chunk upload, auth and bundle log nothing. | `transport/mod.rs` (9 sites), `queue.rs` (3), `session_loop/mod.rs` (7), `install_event.rs` (3), `patch_events.rs` (1); core `telemetry/{tail,chunk,auth,bundle}.rs` (0) | LT-09a, LT-09b |
| F9 | The frontend cannot log to Rust: there is no command and no plugin. Five empty `.catch(()=>{})` handlers swallow errors, and several handlers flatten errors to `transport`/`schema`. **No dead buttons were found**: every button id has a listener. | `main.rs:576-596`; `client-patches-view:62`, `server-list-view:97`, `client-maintenance-view:93`, `patch-notes:62`, `seed-archive-view:90`, `install-view:137/145`, `view.ts:100-103`, `launch-workflow.ts:17` | LT-10, LT-20 |
| F10 | No panic hook and no crash marker. A launch-worker panic becomes `Observation::Unknown` silently. | `storage/launch/worker.rs:78-81` | LT-06, LT-18 |
| F11 | Subsystems with **zero** logging: game update and the release catalog, repair, settings and consent, seed archive, migration, prerequisites, and the shell install/adoption/repair hosts. Error causes are erased by `map_err(\|_\| …)` throughout. | audit tables in the architect report | LT-12 to LT-17, LT-19 |
| F12 | A DLL-inject failure is invisible: `inject_dll` failure maps to `Event::Unknown` with no Win32 error. | `runtime-probe/src/game_launch/windows.rs:57-77` | LT-18 |
| F13 | **The setup-diagnostics checkbox is dead.** The summary exporter has `endpoint: None`. `SummaryEndpoint::parse` requires https, which directive 5 keeps. The login-port router already serves `/api/telemetry/launcher-summary`, and a test pins it, but on the only public login port that is plain http (F23). LT-22/23 send over https to an https login server or the built-in https default, and show a "not sent yet" reason until that default exists (LT-41). | `shell/src/host/summary.rs:22-24`, `:330`; `launcher_summary/endpoint.rs:15`; `admin-api/src/login_port.rs:63`, `:180` | LT-22, LT-23, LT-41 |
| F14 | Stale copy: `index.html:13` and `view.ts:133` say "This build sends nothing". **Corrected 2026-10-21:** `index.html:45` ("cannot send game telemetry") is true whenever it is visible, so it stays (F20). | frontend | LT-23 |
| F15 | The player log upload zips only the game's logs, never the launcher's. | `storage/log_upload/store.rs:21` | LT-21 |
| F16 | **Peer IPs in telemetry logs (directive 6 drops them).** Three sets of rows write the peer IP: the dev-session refusal logs (seven arms, unthrottled, so a retrier writes one row per request), the ingest `RefusalLog` rows, and `admin.request` on telemetry paths. In-memory quota keys and `StreamRecord.peer_ip` (the game-session stream match) are functional state and never written; they stay. The SOAP login rows (`crates/auth/src/auth/handlers.rs:109-379`, `peer_ip`) are game login, not launcher or telemetry, so they are out of scope. | `dev_session/handlers.rs:442-498`; `telemetry/refusal_log.rs:332`, `:375`, `:401`; `request_span.rs:73-101` | LT-02, LT-38, LT-40, LT-21 |
| F17 | OIDC has no code anywhere. | grep `oidc` over crates: 0 | reserved target only |
| F18 | SigNoz has no view or dashboard for the launcher's own events. | `docs/operations/signoz/` has only the summary/journey fixtures | LT-24 |
| F19 | ~~Quick win: set the env var in the colo `.env`.~~ **Superseded by directive 1:** LT-01 gives the server a built-in default and LT-05/07/08 replace a loopback endpoint in the launcher. The env var stays as an explicit override; OQ-LT5 is now only the verification step LT-27 U1. | `docs/operations/telemetry.md:195` | LT-01, LT-27 U1 |
| F20 | **Checkbox audit (directive 2).** `#telemetry` ("Share setup diagnostics", `disabled` in the HTML) is dead today: LT-22/23 make it real. `#game-telemetry` is real (AU-13 and game sessions; LT-11 adds a status line). `#adoption-telemetry` (`index.html:45`) is a real consent gate, shown only when `requires_telemetry_acceptance` (old launcher opted in and this build cannot send); its copy is accurate. `adoption-replace`, `adoption-normalize` and `adoption-closed` are real consent gates. The mode radios are AU-05's. | engine `adoption/mod.rs:127-131`, `:149`; frontend `index.html` | LT-11, LT-22, LT-23 (D-LT13) |
| F21 | **Adoption refusals do not say why (directive 3).** About 70 refusal sites in `storage/adoption/` return a bare variant. The shell collapses them to the `Copy` enum `AdoptionError` (Corrupt, TooLarge and InvalidServerList all become `corrupt_state`); `adoption-view.ts` shows fixed text and maps unknown codes to platform-unavailable; `import_required` has no text; several sites (no reusable files, the copy-mode comparison error) are silent; the shell adoption host logs nothing. | engine `adoption/{source,inventory,mod,verification,artifacts,replay,in_place_preview,reference,worker}.rs`; shell `contract.rs:74-121`, `:156-182`; `adoption-view.ts:7-39`; `adoption-workflow.ts:4-5`, `:24` | LT-28..LT-34 |
| F22 | **No file index is published, and the fallback is silent (directive 4).** Every adopt downloads the 4.1 GB seed. A malformed reference, a download failure or a verify failure falls back with no reason on screen and only a free-string warn; the shell drops the progress label, so the player sees an unexplained download. Nothing checks a candidate manifest plus index offline before publishing. | engine `verification.rs:528-557`, `:575-618`; shell `status.rs:53-67`; any-install-adoption AI-14 and `worknotes/campaign-close.md:52-54` | LT-30, LT-33..LT-37 |
| F23 | **Superseded by directive 7 (amendment 4): the colo now serves `https://play.cimmeria.app`** through the board's Caddy proxy, which forwards only the five telemetry routes to loopback :8081 (everything else 404). The original finding follows. **The colo has no https telemetry endpoint on record (directive 5).** The login port 8081 is plain HTTP. An https route is documented only as an optional Cloudflare Tunnel hostname, and the colo ran no tunnel as of 2026-10-04. The auth TLS listener (#566) exists in code, but compose configures no certificate, and nothing shows that it serves the telemetry routes. So the https default cannot be chosen yet: the constants ship as `None`, an unconfigured server refuses mints with 503 `no_upload_endpoint`, and LT-41 sets the value. The architect cannot check the live colo; `ops` verifies it before LT-41. | `docs/operations/container.md:58`; `docs/operations/telemetry.md:145-202`; `crates/auth/src/auth/tls.rs`; `docker/compose.yml` | LT-01, LT-05, LT-22, LT-41 (D-LT17), LT-04 |
| F24 | **Behind the https proxy every caller shares one quota bucket (directive 7a).** All per-IP counters key on the socket peer through `ip_key`: summary 12/min, chunk 600/min, bundle 30/h, mint 120/h, refresh 480 and bad-refresh 30, and the one-upload-per-peer slot. Through Caddy that peer is the proxy, so players 429 and 503 each other at scale. Nothing reads `X-Forwarded-For` today. | `launcher_summary/handlers.rs:42`, `:171`; `upload_gate.rs:71`, `:77`, `:329`, `:342`; `dev_session/handlers.rs:38-47`, `:250`, `:345`, `:355`; `routes/telemetry/mod.rs:194` | LT-42 |
| F25 | **The colo server may not see the proxy as loopback.** `docker/compose.yml:45-47` publishes 8081 from a bridge-network container. A connection from the host's loopback to a published port normally reaches the container from the bridge gateway, not 127.0.0.1, so a loopback-only trust rule would leave F24 in place on the colo. The architect cannot check the live host. | `docker/compose.yml:45-47` | D-LT20, OPS-LT2, LT-27 U10 |
| F26 | **The game-session telemetry match by address degrades behind the proxy.** `client_telemetry::lookup` matches the account first, then falls back to a live stream whose `peer_ip` equals the game login's address (`crates/auth/src/auth/client_telemetry/mod.rs:159-184`). A mint through Caddy records the proxy's address, so that fallback finds nothing for https launchers that send no account. Directive 7(a) limits `X-Forwarded-For` to quota keys, so LT-42 leaves it alone. | `handlers.rs:293`, `:403`, `:427-434`; `client_telemetry/mod.rs:171-177` | D-LT21 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LT1 | Launcher events stay in `service.name = cimmeria-client` with `cimmeria.session_kind = launcher`; there is no `cimmeria-launcher` service. LT-24 adds a saved view and a dashboard that filter on it. | Decided (owner, AU OQ-5 / D-AU13) | One client index; the replay path through `client.native` needs no `otel.rs` change. |
| D-LT2 | Consent. **Default A:** the launcher's own events ride on "Send game diagnostics" (the D-AU22 gate), and "Share setup diagnostics" gates only the anonymous summaries. B: setup diagnostics gates launcher events. C: merge the two boxes. | **BlockedDecision** (owner; default applied, blocks nothing) | A matches the shipped AU design and the current copy. |
| D-LT3 | **Amended twice (directives 1 and 5).** The server's localhost fallback is deleted. Resolution order (C11): (1) `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT`, used as given (a plain-http value WARNs `upload_endpoint_plain_http`, a loopback value WARNs `upload_endpoint_loopback`); (2) `https://<Host>/api/telemetry` only with `X-Forwarded-Proto: https`; (3) a loopback Host from a loopback peer gives the http loopback URL (`loopback_dev`, a one-host dev box only); (4) the built-in `DEFAULT_UPLOAD_ENDPOINT`, which is `None` until LT-41 (D-LT17). If nothing resolves, the mint fails with 503 `no_upload_endpoint` before signing, and boot WARNs `upload_endpoint_unset`. The mint INFO carries `upload_endpoint_source` and `upload_endpoint_https`, and never the URL for a Host-derived source. | Decided (owner directives 1 and 5); LT-01 gets a network-security-auth pass | The login port serves the telemetry routes (`login_port.rs:63`). A forged Host only misdirects the forger's own uploads. No path sends a player's telemetry over plain http by default. |
| D-LT4 | **Decided by directive 5: https only.** The summary endpoint is `https://<host:port>/api/telemetry/launcher-summary` when the first login server (`default_servers()` when the list is empty) is https. Otherwise it is the built-in `DEFAULT_SUMMARY_BASE`, which is `None` until LT-41. With neither, the summary is saved but not sent, and the box says so on the first press (directive 2). `SummaryEndpoint::parse` keeps its https check. LT-26 updates `docs/architecture/launcher-summary-telemetry.md`. | **Decided** (owner directive 5, replacing the directive 1/2 reading) | The owner wants no plain-http path. The colo has no https endpoint yet (F23), so the default waits on D-LT17. |
| D-LT5 | A closed table of 12 launcher targets (contract C1), pinned by the canonical file `crates/launcher-core/src/telemetry/launcher_targets.txt` and by golden tests in both the engine and admin-api. Tracing target `launcher.<area>` maps to wire `client.launcher.<area>`; `adoption` keeps its AU name. `client.launcher.login` is reserved for OIDC. | Decided | Exact list per D-AU20; one file stops server and launcher drifting apart without a shared crate. |
| D-LT6 | Pipeline health is counters, not log lines. The sender records each `SendOutcome` in `TelemetryHealth` (C7). On the first `Sent` after any failures, it emits one `launcher.telemetry` `pipeline_recovered` event with the counts. The sender, the forwarder and the mint never log to a forwarded target; they use the file-only target `launcher.pipeline` (C4). | Decided | Logging a send failure into the queue that just failed loops. |
| D-LT7 | **Amended twice (directives 1 and 5).** The launcher **replaces** (does not refuse) a loopback upload endpoint advertised by a non-loopback login server (`effective_upload_endpoint`, C10). The replacement is the login server's origin plus `/api/telemetry` when that origin is https, else the built-in `DEFAULT_UPLOAD_ENDPOINT` (`None` until LT-41). If neither exists, the mint fails with `MintFailed { reason: "no_https_endpoint" }` and nothing is sent. It logs `upload_endpoint_overridden` / `endpoint_overridden` once per session. No new `Outcome`, no `endpoint_misconfigured`. | Decided (owner directives 1 and 5) | The player gets working telemetry from an old or misconfigured server, never over a plain-http fallback; SigNoz still shows the override. |
| D-LT8 | ~~The LT launcher work ships stacked on AU 2b, with the AU release.~~ **Moot (amendment 3):** AU shipped to `main` in #1953 on 2026-10-21, so PR L branches from `origin/main` and ships with the next launcher release. | Closed (overtaken by #1953) | No hold is left to time against. |
| D-LT9 | Event rules (work-packets § Rules R5, R6): forward INFO and above only, with no per-tick or per-progress events. At most 12 fields, closed codes only, and never a path, URL, host, IP, account name or error Display (R5a allows folder-relative paths on `adoption`). One start event and one outcome event per operation. | Decided | 300 rows per 15-minute session (D-AU24); privacy. |
| D-LT10 | `ui_event` is a Tauri command, not wire. It takes a closed area list and snake_case codes, at most 60 per minute; excess events are counted in the next accepted one. | Decided | Lets the frontend's swallowed errors reach SigNoz with no new protocol. |
| D-LT11 | The player log upload adds the newest 3 `launcher.*.log` files, capped at 4 MiB in total, with every line passed through AU's `Redaction` (home path, scrub) and then LT-40's `scrub_ips` (amendment 2). | Decided | It is user-initiated, and the same path already uploads game logs. |
| D-LT12 | No packet touches the lab, a live client, a live server, SigNoz or a release asset. The live checks are owner UAT steps written by the close-out (LT-27). LT-00 is a read-only SigNoz query. | Decided | Owner rule: ask before lab use. |
| D-LT13 | Checkbox audit (F20): no checkbox stays dead. `#telemetry` becomes clickable with first-press feedback (LT-23); `index.html:45` keeps its copy; the other boxes need no change. | Decided (owner directive 2) | Each box either gates real behaviour or is fixed by a named packet. |
| D-LT14 | R5a: on the `adoption` target, `entry` may hold a folder-relative path or a manifest label (`seed`, `patch:<id>`, `file_index`, `manifest`); `expected`/`found` hold sizes, counts, digests in the C14 `digest()` form or closed phrases. Never an absolute path, URL or host. | Decided | D-AU7 already allows relative paths; a refusal is useless without the file. |
| D-LT15 | Publishing the file index: the key holder signs the candidate manifest with the offline release key, uploads `file-index.json`, and swaps `content-current` (manifest plus `.sig`) after LT-36's checks pass. Owner also picks whether the index is added to the current immutable `content-YYYY-MM-DD-NNN` release or a new release is cut (this decides `blob`). Settles any-install-adoption D-AI8 / AI-14. | **BlockedDecision** (owner; blocks LT-37 only) | Only the owner holds the release key; release assets are owner-only. |
| D-LT16 | Ownership with AU: AU keeps `unsafe_file` refusals (`UnsafeReport`, `unsafe_entries`), the mode UI, the six step events (AU-09), the subscriber and `adoption` forwarding (AU-08/13) and the server row rules. LT owns every other refusal reason (`last_rejection`, `adoption_rejected`), AU F17 (`TooLarge` shown as `corrupt_state`), the index fallback reason and the file-index publication. Table: work-packets-adoption.md § Ownership. | Decided | AU sliced only the unsafe-file path; the rest had no owner. |
| D-LT17 | The https origin the colo serves for `/api/telemetry` and `/api/telemetry/launcher-summary`, which becomes the built-in `DEFAULT_UPLOAD_ENDPOINT` (server and launcher) and `DEFAULT_SUMMARY_BASE` in LT-41. **Resolved (directive 7): `https://play.cimmeria.app`**, the board's Caddy proxy. `ops` confirms it with LT-41's status-code probe right before dispatch: a plain HTTP check from the workstation, with no lab, ssh or credentials. Until LT-41 merges the constants stay `None`, and there is no http fallback. | **Decided** (owner directive 7) | Directive 5 asked for an https origin the colo actually serves; one now exists. |
| D-LT19 | **`X-Forwarded-For` as a quota key only (directive 7a).** The server reads the header only when the socket peer is loopback, takes the rightmost entry, and uses it only as the key of the per-IP quotas (summary, chunk, bundle, mint, refresh, bad refresh) and of the per-peer upload slot (C18, LT-42). It is never logged, stored, hashed into a row or put in a field, and it never replaces the socket peer anywhere else. A non-loopback peer's header is ignored. | Decided (owner directive 7) | One shared proxy bucket 429s players at scale. Loopback-only trust means a remote caller cannot pick its own key. |
| D-LT20 | **If the colo server sees the proxy as a non-loopback address (F25)**, LT-42 changes nothing there. Options: A) a follow-up packet adds an explicit trusted-proxy list, exact addresses from an env var, default empty; B) run the server container on host networking, or have the proxy reach it some other way that looks loopback to the container (compose or ops change); C) accept the shared bucket and raise the quotas. Raised only if OPS-LT2 or U10 shows the problem. | **BlockedDecision** (owner; blocks nothing now) | Directive 7(a) says loopback only. Widening the trust set is the owner's call. |
| D-LT21 | **The game-session stream match behind the proxy (F26).** Should `StreamRecord.peer_ip` take the forwarded address for a loopback peer (in memory only, never written)? Default: no. Directive 7(a) limits the header to quota keys, and the account match still works. A yes is one small follow-up packet in `dev_session/handlers.rs` `record_stream`. | **BlockedDecision** (owner; default applied, blocks nothing) | It widens the use of the header beyond the directive. |
| D-LT18 | **No IP in any launcher or telemetry log, field or attribute (directive 6).** Deleted, never hashed: the `peer` field on the dev-session refusal logs (LT-38, which also throttles them per route and reason with a `suppressed` count), on the ingest `RefusalLog` rows (LT-02), and on `admin.request` rows for `/api/telemetry*` and `/api/auth/dev-session` (LT-02). Summary refusals log the reason and status only (LT-39). On the launcher, `scrub_ips` replaces IPv4 and IPv6 literals with `[ip]` in forwarded fields and uploaded logs, and the C9 capture helpers panic on a captured IP (LT-40, LT-21). In-memory quota and throttle keys and `StreamRecord.peer_ip` stay, because they are never written. Each packet has a test that fails if a TEST-NET IP appears. | Decided (owner directive 6) | Refusal logs keep the reason and a count, which is enough for triage. A hash of an IPv4 is reversible by enumeration. |

## Open owner questions

The orchestrator asks these; the architect does not.

- **OQ-LT1 = D-LT2** consent semantics. The default (A) is applied.
- ~~OQ-LT2 = D-LT4~~ resolved by owner directive 5: https only, no plain-http path.
- ~~OQ-LT3~~ resolved by owner directive 6: drop the IP everywhere, never hash it (D-LT18; LT-02, LT-38, LT-39, LT-40).
- ~~OQ-LT4 = D-LT8~~ moot: AU merged in #1953 (amendment 3).
- **OQ-LT5** now a verification only (LT-27 U1): after the colo deploy, a mint shows `upload_endpoint_source` and a non-loopback endpoint. Never commit the address.
- ~~OQ-LT6 = D-LT15~~ resolved by D-LA11 (2026-10-21): the index goes to a new GitHub release `content-file-index-<yyyymmdd>`, published by LA-20 (hands-on) after LA-05 loosens the blob check.
- ~~Per-file adoption policy (accept, replace or refuse each differing file)~~ **superseded** by D-LA9 (tolerate skipped patch ops per op, never block) and D-LA11 (report differences vs the published index, never block). See [light-adoption-proposal.md](light-adoption-proposal.md) § 6.
- ~~**D-LA14** whether incremental update covers every install or only adopted ones~~ **decided by the owner (2026-10-21):** the seedless Update covers light-adopted installs only. The discriminator is the installed-content record's adoption provenance with `check = light`. Downloaded installs keep today's seed rebuild byte for byte (hard rule R-LA 10; guard tests in LA-04 and LA-13). Extending it to every install is the later option D-LA14b. See [light-adoption-proposal.md](light-adoption-proposal.md) § 3.
- **D-LA12** whether to delete the verified in-place path after the light-adoption UAT. Asked at close-out.
- ~~OQ-LT7 = D-LT17~~ resolved by owner directive 7: `https://play.cimmeria.app` (amendment 4).
- **OQ-LT8 = D-LT20** whether to widen `X-Forwarded-For` trust past loopback, asked only if OPS-LT2 or U10 shows the colo sees the proxy as a non-loopback address.
- **OQ-LT9 = D-LT21** whether the game-session stream match may use the forwarded address. The default (no) is applied.

## Owner and ops tasks (not packets)

- **OPS-LT1 (owner or ops).** The Caddyfile edit that adds the `play.cimmeria.app` site lives only on the colo host. Mirror it into the private agent-board repo, or the next `board-proxy` redeploy undoes it and LT-41's default stops answering. Rollback: restore the newest `Caddyfile.bak-*` and run `docker restart board-proxy`. No address goes into this repo.
- **OPS-LT2 (ops, after PR S is deployed).** Find out whether the cimmeria server container sees the Caddy connections as loopback (F25): its network mode, and whether the published 8081 is reached through the bridge. Report yes or no, with no addresses. A "no" raises D-LT20.
- **LT-41 probe (ops, before LT-41 dispatch):** the three status-code checks in work-packets-coverage § LT-41.

## Packet status

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

| Wave | Packet | Title | Agent | Depends on | Status |
|---|---|---|---|---|---|
| 0 | LT-00 | SigNoz triage of today's launcher rows (aggregates only) | general-purpose | — | Ready |
| 1 | LT-01 | https-only upload endpoint resolution, no localhost fallback | packet-coder | base (see work-packets § Branches) | Ready |
| 1 | LT-02 | No IP in ingest refusal rows or telemetry request rows | packet-coder | base | Ready |
| 1 | LT-03 | Server accepts the 12 launcher targets, with a golden test | packet-coder | base (`launcher_rows.rs` is on `main`) | Ready |
| 1 | LT-39 | Launcher-summary refusal rows (reason and status, no IP) | packet-coder | base | Ready |
| 1 | LT-38 | Dev-session refusals: no peer, throttled, with a count | packet-coder | LT-01 merged into `lt-srv-integration` | BlockedDependency |
| 1 | LT-42 | Per-IP quota key from `X-Forwarded-For`, loopback peers only (amendment 4) | packet-coder | LT-01, LT-02, LT-38, LT-39 merged into `lt-srv-integration` | BlockedDependency |
| 1 | LT-04 | Server docs for LT-01..03, LT-38, LT-39, LT-42, and the Caddy https route | packet-coder | LT-01..03, LT-38, LT-39, LT-42 integrated | BlockedDependency |
| 2 | LT-05 | Forward every launcher target, capture helpers, effective endpoint | packet-coder | `lt-app-integration` (from `origin/main`) | Ready |
| 2 | LT-06 | Startup event, panic hook, crash marker | packet-coder | LT-05 | BlockedDependency |
| 2 | LT-07 | Pipeline health counters and the recovery row | packet-coder | LT-05 | BlockedDependency |
| 2 | LT-08 | Game-session mint causes; replace a loopback upload endpoint | packet-coder | LT-05 | BlockedDependency |
| 2 | LT-09a | Structured game-telemetry transport and queue events (amendment 5) | packet-coder | LT-05 | Ready (re-dispatch in `lt-09`) |
| 2 | LT-09b | Structured session-loop and queue-write events (amendment 5) | packet-coder | LT-09a | BlockedDependency |
| 2 | LT-10 | `ui_event` bridge (Rust command and TS helper) | packet-coder | LT-05 | BlockedDependency |
| 2 | LT-11 | `telemetry_status` command and the status line | packet-coder | LT-07, LT-08, LT-10 | BlockedDependency |
| 3 | LT-40 | `ip_scrub` at the forwarder; C9 helpers panic on an IP | packet-coder | LT-05 | BlockedDependency |
| 3 | LT-12 | Updater check, download and verify events | packet-coder | LT-05 | BlockedDependency |
| 3 | LT-13 | Game update and release catalog events | packet-coder | LT-05 | BlockedDependency |
| 3 | LT-14a | Core install error codes and core `launcher.install` events (amendment 6) | packet-coder | LT-40 | Ready |
| 3 | LT-14b | Install lifecycle, `operation_numbers`, engine `error_codes` (amendment 6) | packet-coder | LT-14a | BlockedDependency |
| 3 | LT-15 | Seed archive, PhysX, migration, prerequisites events | packet-coder | LT-05 | BlockedDependency |
| 3 | LT-16a | Repair and uninstall events (amendment 6) | packet-coder | LT-14b merged | BlockedDependency |
| 3 | LT-16b | Client maintenance and client-changes events (amendment 6) | packet-coder | LT-16a | BlockedDependency |
| 3 | LT-17a | Launch and Play lifecycle events (amendment 6) | packet-coder | LT-14b merged | BlockedDependency |
| 3 | LT-17b | Game-telemetry session, helper stop, DLL artifact events (amendment 6) | packet-coder | LT-17a | BlockedDependency |
| 4 | LT-18 | Launch-worker stage and Win32 code (CreateProcess, inject, resume, wait) | packet-coder | wave 3 and its review integrated | BlockedDependency |
| 4 | LT-19 | Settings and consent change events | packet-coder | wave 3 and its review integrated | BlockedDependency |
| 4 | LT-20 | Frontend swallowed errors through `ui_event` (five testable sites) | packet-coder | wave 3 and its review integrated | BlockedDependency |
| 4 | LT-21a | Log-upload events on `launcher.telemetry`, blob name dropped | packet-coder | wave 3 and its review integrated | BlockedDependency |
| 4 | LT-21b | Log upload includes the launcher logs (redacted, IPs scrubbed) | packet-coder | LT-21a (same branch) | BlockedDependency |
| 4 | LT-22 | https-only summary endpoint from the login server or the default (engine) | packet-coder | wave 3 and its review integrated | BlockedDependency (D-LT4 decided) |
| 4 | LT-23a | Shell resolves the summary endpoint from the first login server | packet-coder | LT-22 (same branch) | BlockedDependency |
| 4 | LT-23b | Consent box copy and "not sent yet" first-press feedback (`NativeSnapshot.setup_diagnostics_endpoint`) | packet-coder | LT-23a (same branch); merges after LT-19 | BlockedDependency |
| 5 | LT-28 | Adoption rejection record, log and plumbing (C14) | packet-coder | LT-05 merged into `lt-app-integration` (AU wave 1 is on `main`) | BlockedDependency |
| 5 | LT-29 | Source, inventory and comparison checks | packet-coder | LT-28 | BlockedDependency |
| 5 | LT-30 | Verification refusals and the index fallback reason (C15) | packet-coder | LT-28 | BlockedDependency |
| 5 | LT-31 | Artifact and patch-replay checks | packet-coder | LT-28 | BlockedDependency |
| 5 | LT-32 | In-place, reference, worker and consent refusals | packet-coder | LT-28 | BlockedDependency |
| 5 | LT-33a | Engine `refuse`, `ConfirmationWorker::last_rejection`, re-exports (C14) | packet-coder | LT-28..LT-32 | Ready (re-sliced, [LT-33-amendment.md](LT-33-amendment.md)) |
| 5 | LT-33b | Shell status `last_rejection`, review `index_fallback`, progress `item`, TS schema (C16) | packet-coder | LT-33a | BlockedDependency |
| 5 | LT-33c | Shell-originated refusals logged with their check; migration doc (C14, C16) | packet-coder | LT-33b | BlockedDependency |
| 5 | LT-34 | Frontend reasons and the player guide (C17) | packet-coder | LT-33c | BlockedDependency |
| 6 | LT-35 | `verify-file-index`: chain, binding and parity checks | packet-coder | — | Ready |
| 6 | LT-36 | Publish runbook and prepare script | packet-coder | LT-35 | BlockedDependency |
| 6 | LT-37 | Owner signs and publishes the file index | owner | PR I merged, D-LT15 | Superseded by LA-05 + LA-20 (run blocked at `blob_form_matches_seed`, [worknotes/LT-37.md](worknotes/LT-37.md)) |
| 7 | LT-24 | SigNoz saved view, dashboard, offline guard | packet-coder | wave 1 merged | BlockedDependency |
| 7 | LT-25 | REPL logic UAT `uat:launcher-telemetry` | packet-coder | waves 2-4 integrated | BlockedDependency |
| 7 | LT-26 | Launcher and operations docs | documentation-writer | LT-25 | BlockedDependency |
| 7 | LT-41 | Built-in https default `https://play.cimmeria.app` | packet-coder | PR S and PR L merged, then the ops status-code probe (D-LT17 resolved) | BlockedDependency |
| 7 | LT-27 | Close-out: status docs, owner UAT steps U1-U9, retire | documentation-writer | every PR merged | BlockedDependency |
| 8a | LA-01 | Patch source and output summaries (core) | packet-coder | `lt-light-integration` | Ready |
| 8a | LA-02 | Tolerant per-op apply with a skipped list (patchset) | packet-coder | `lt-light-integration` | Ready |
| 8a | LA-05 | `index_blob_form` accepts any https blob; publish runbook (PR I2) | packet-coder | `origin/main` | Ready |
| 8a | LA-06 | Folder vs file-index comparison by class; cooked-data alias (core) | packet-coder | `lt-light-integration` | Ready |
| 8a | LA-07 | Remove the unused `adopt_existing_install` (core) | packet-coder | `lt-light-integration` | Ready |
| 8a | LA-08 | Stage seeding with hard links outside a write set (core) | packet-coder | `lt-light-integration` | Ready |
| 8b | LA-03 | Incomplete-patch ledger and tolerant `unpack_with` (core) | packet-coder | LA-02 | BlockedDependency |
| 8b | LA-04 | Tolerant `install_all` with incomplete retry (core) | packet-coder | LA-03, LA-07 | BlockedDependency |
| 8b | LA-09 | Light plan and `adoption_light_planned` (engine) | packet-coder | LA-01 | BlockedDependency |
| 8b | LA-10 | Tolerant light replay into replacements (engine) | rust-gameserver-dev (Sonnet) | LA-03, LA-09 | BlockedDependency |
| 8b | LA-11 | Light mode, digest, provenance `check`, incomplete ledger (engine) | rust-gameserver-dev (Sonnet) | LA-10, LA-12 | BlockedDependency |
| 8b | LA-12 | Published-index comparison, `adoption_index_compared`, scan progress (engine) | packet-coder | LA-06 | BlockedDependency |
| 8c | LA-13 | Seedless incremental update for light-adopted installs only; downloaded installs unchanged (D-LA14) (engine) | rust-gameserver-dev (Sonnet) | LA-01, LA-04, LA-08, LA-11 | BlockedDependency |
| 8c | LA-14 | Shell modes `[Light, Copy]`, one-level-down search, `several_game_folders` | packet-coder | LA-11 | BlockedDependency |
| 8c | LA-15 | Shell status and TS schema: light review, comparison, incomplete count | packet-coder | LA-14 | BlockedDependency |
| 8c | LA-16 | Adoption view copy, skipped list, first-press status | packet-coder | LA-15 | BlockedDependency |
| 8c | LA-17 | `launch_started` gains `adoption_check`, `patches_incomplete` | packet-coder | LA-11 | BlockedDependency |
| 8c | LA-18 | Index comparison panel, incomplete hint, `uat:adoption-light` | packet-coder | LA-16 | BlockedDependency |
| 8 close | LA-19 | Docs: player guide, telemetry fields, update doc | documentation-writer | LA-01..LA-18 integrated | BlockedDependency |
| 8 close | LA-20 | Re-run the LT-37 publish (sign, new release, swap, rollback ready) | hands-on | PR I2 merged with green CI | BlockedDependency |

## PRs and merge order

| PR | Contents | Ships from | Base |
|---|---|---|---|
| S | Wave 1: LT-01 → LT-02 → LT-03 → LT-39 → LT-38 → LT-42 → LT-04 | `launcher-telemetry/server` (worktree `lt-srv-integration`) | `origin/main` |
| L | Waves 2-4, plus LT-25 and LT-26 | `launcher-telemetry/launcher` (worktree `lt-app-integration`) | `origin/main` |
| A | Wave 5: LT-28 → LT-29 → LT-30 → LT-31 → LT-32 → LT-33a → LT-33b → LT-33c → LT-34 | `launcher-telemetry/adoption` (worktree `lt-adopt-integration`) | `lt-app-integration` at the LT-05 merge (no AU merge) |
| I | Wave 6: LT-35 → LT-36 (LT-37 is the owner's, after the merge) | `launcher-telemetry/file-index` (worktree `lt-index-integration`) | `origin/main` |
| F | LT-24 | its own branch | `origin/main` after PR S |
| H | LT-41 (after the ops status-code probe; D-LT17 resolved) | its own branch `launcher-telemetry/lt-41-https-default` | `origin/main` after PR S and PR L |
| LA | Wave 8: 8a, 8b, 8c, then LA-19 (merge orders in [work-packets-light-adoption.md](work-packets-light-adoption.md) § Branches) | `launcher-telemetry/light-adoption` (worktree `lt-light-integration`) | the `launcher-telemetry/launcher` tip; `ship.ps1` restacks onto `main` after PR L |
| I2 | LA-05 alone (then LA-20 by hands-on, after the merge) | `launcher-telemetry/la-05-index-blob-form` (worktree `la-05`) | `origin/main` |
| X | LT-27 | its own branch | `origin/main` after every other PR |

Into `main` (amendment 3; #1953 already carries all of AU): PR S (LT-42 merges into it last before LT-04) → colo deploy (owner; it also deploys #1953's launcher session kind if the colo has not had it yet) → OPS-LT2 → PR F; PR I at any time (then LT-37 by the owner); PR A → PR L → LT-41 probe → PR H (before the launcher release) → PR X. If the probe fails, PR X may ship with LT-41 still blocked; U5 then stays an owner step. OPS-LT1 (mirror the Caddyfile) any time, but before the next `board-proxy` redeploy. Whichever of PR A and PR L lands first carries LT-05; `ship.ps1` restacks the other onto `main` after it. AU-09 is already on `main`, so LT-33's edits to `shell/src/host/adoption/mod.rs` build on it directly. PR S and PR L both add `launcher_targets.txt` byte-identical, so the second to land merges it cleanly.

Wave 8: PR I2 at any time (then LA-20); PR LA after PR L, before the launcher release that carries LT-41 if both are ready. Light adoption, incremental update and the index comparison all need the owner UAT on the laptop (the lab is asked first).

Waves 5 and 6 run beside waves 2-4. Wave 6 can start at once; wave 5 starts when LT-05 has merged into `lt-app-integration`.
