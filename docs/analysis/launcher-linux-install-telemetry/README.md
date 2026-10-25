# Launcher: Linux install and https summaries

Campaign ledger. Packets: [work-packets.md](work-packets.md) and
[work-packets-li09.md](work-packets-li09.md). Resume card:
[RESUME.md](RESUME.md). Host: **linux** (Steam Deck), every packet.

**Amended 2026-10-24** by ruling LI-02-ruling-1 (owner/architect, 2026-10-24; not in git): LI-01..LI-04 are superseded by `cab_portable` on `main`, the
integration branch is abandoned, and LI-08 and LI-07 are re-specced to ship as
one PR from `main`. Track B (LI-05, LI-06, LI-09) does not change.

## Purpose

Two launcher fixes found on the Steam Deck:

- **A. Installing from a local archive fails on Linux** with
  `install_stage_failed reason="unpack"`. The archived client RAR holds the 2009
  MakeCAB spanning set (`Data\DATA*.CAB` plus `DATA.INF`). Owner decision: **no
  Wine on Steam Deck or Linux**. #2020 and #2023 landed the native expander
  (`unpack::cab_portable`) on `main` the same day this campaign was sliced
  (F-15), so track A keeps only the real-client check (LI-07) and two doc
  fixes (LI-08). LI-05 makes the stage failure log its detail.
- **B. Launcher summaries never send.** The summary endpoint is derived only
  from an https login server. The configured server is
  `http://play.cimmeria.app:8081`, which stays http because the game client
  cannot do TLS. Owner decision: keep the login server on http, send summaries
  only to an https endpoint, and **never send telemetry over plain http**.
  Owner decision (2026-10-24, OQ-1): the rule covers **game telemetry** too.
  LI-09 removes the plain-http path from `EndpointPolicy` and reaches an http
  login server's telemetry routes over https.

## Findings

| ID | Finding | Evidence | Packet |
|---|---|---|---|
| F-01 | **Stale (F-15).** On non-Windows hosts, `cab_set::expand` returned `UnpackError::Cab("expanding the cabinet set in … needs Windows")`, so every installer-RAR install on Linux failed at the unpack stage. #2020 replaced that error with `cab_portable::expand_chain`. | `crates/launcher-core/src/unpack/cab_set.rs:131-135` on `main` | - |
| F-02 | The real set is **MSZIP only**: it has no LZX and no Quantum. It splits CFDATA blocks at cabinet boundaries (the first fragment has `cbUncomp = 0`), its folders continue across cabinets, and its blocks reference the 32 KB history of the block before. | A header and CFDATA walk of the owner's `client.rar` (architect scratch scripts) | LI-07 |
| F-03 | **Stale (F-15).** The `cab` crate reads single cabinets only, so an in-house reader looked smaller than a fork. `cab_portable` on `main` is that reader. | Crate API; `fdi.rs` semantics | - |
| F-04 | **Stale (F-15).** `flate2`'s `zlib-rs` feature would have given `set_dictionary` with no new package. `cab_portable` uses `miniz_oxide` instead, so no dependency changes. | `Cargo.lock`, `crates/launcher/desktop/Cargo.lock` | - |
| F-05 | `install_stage_failed` logs only `reason = error.code()` (`unpack`), and the inner `UnpackError` text is dropped. The forwarded launcher targets must carry no paths or arbitrary error text. | `engine/src/storage/install_worker/events.rs`, `crates/launcher/desktop/docs/launcher-telemetry.md` | LI-05 |
| F-06 | `repair_stage_failed` has the same shape and the same gap. | `engine/src/storage/repair/events.rs` | LI-05 |
| F-07 | `SummaryEndpoint::for_login_server` refuses http, and `DEFAULT_SUMMARY_BASE` is `None`, so `resolve` returns `no_https_endpoint` for `http://play.cimmeria.app:8081`, and summaries never send. | `engine/src/storage/launcher_summary/endpoint.rs:51-75`, `shell/src/host/summary.rs` | LI-06 |
| F-08 | `https://play.cimmeria.app/api/telemetry/launcher-summary` answers on 443. | The brief's investigation, unverified here | LI-06, U2 |
| F-09 | LT-41 (the [launcher-telemetry](../launcher-telemetry/README.md) campaign) owns `DEFAULT_SUMMARY_BASE`, and its test expectation conflicts with LI-06 (`resolve_falls_through_to_no_https_endpoint`). | `../launcher-telemetry/work-packets-coverage.md` LT-41 | LI-06 |
| F-10 | `real_client_rar` (`#[ignore]`, driven by env vars) checks only that `SGW.exe` exists. The real `SGW.exe` is 31,228,448 bytes. | `unpack/mod.rs` tests | LI-07 |
| F-11 | Game-telemetry `EndpointPolicy` still allows http to the host and port of an http login server. The owner resolved OQ-1: the "never plain http" rule covers this too. | `crates/launcher-core/src/telemetry/endpoint.rs:53-77` | LI-09 |
| F-12 | Game telemetry builds its mint, refresh and upload base from the raw login server (`auth_base_url`, `mint_url`). Tightening the policy alone would therefore end game telemetry for every http login server, including the colo. | `engine/src/storage/game_telemetry/session.rs:73-79,170-177`, `transport/mod.rs:191` | LI-09 |
| F-13 | The colo's https route (Caddy) forwards all five telemetry routes, including `/api/auth/dev-session` and its `/refresh`. A server behind it answers an https-proxied mint with `https://<Host>/api/telemetry` (`request_host`), unless the env value `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` overrides it. | `docs/operations/telemetry-upload-endpoint.md` § The colo route and its resolution table | LI-09, G-LI9 |
| F-14 | LI-06 writes the upgrade rule in the desktop engine. `EndpointPolicy` lives in `launcher-core`, which the engine re-exports as `telemetry_endpoint`. For both to share one rule, it has to live in `launcher-core`. | `launcher-core/src/lib.rs:35`, `engine/src/lib.rs:36` | LI-09 |
| F-15 | #2020/#2023 landed `unpack::cab_portable` on `main` the same day. It is the native expander and an every-host fixture writer with the MakeCAB boundary rule (`break_at_boundary`). It covers MSZIP with 32 KiB history across blocks and cabinets (`miniz_oxide`), stored folders, continued folders, split blocks, reserve areas, DOS stamps and the name preflight, and refuses LZX and Quantum by name. `installer_rar_from_portable_cabinets_expands_on_every_host` is the revert test, and on Windows it runs FDI on the writer's output. | `c22f36c61` (#2020), `0e2c37812` (#2023); `unpack/cab_set.rs:131-135`, `unpack/cab_portable/`, `unpack/mod.rs:361`; ruling LI-02-ruling-1 | LI-07, LI-08 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-LI1 | Linux expands the cabinet set natively, with no Wine. | Decided (owner) | Owner decision. |
| D-LI2 | ~~Write an in-house reader, `crates/launcher-core/src/unpack/mscab/`, and not use the `cab` crate.~~ | **Superseded (F-15)** | `cab_portable` on `main` is that reader. A second reader in the same crate breaks "look for an existing tool first". |
| D-LI3 | Support compression types 0 (none) and 1 (MSZIP) only. `cab_portable` refuses LZX and Quantum with an `UnpackError::Cab` that names the type. | Decided (now describes `cab_portable`, F-15) | F-02: the only set we ship is MSZIP. Writing an LZX decoder for no input is wasted work and risk. |
| D-LI4 | ~~Decompress MSZIP with `flate2` and its `zlib-rs` feature.~~ | **Superseded (F-15)** | `cab_portable` decodes MSZIP with `miniz_oxide`; no dependency changes. |
| D-LI5 | `cab_portable` does not verify CFDATA checksums. `format::read_block_header` reads the 4-byte `csum` field and discards it, and the fixture writer writes 0. | Decided (checked on `main`, 2026-10-24) | The archive is SHA-256-checked against the signed seed before unpack (`seed_source`). Any corruption left is caught by the `CK` signature, an inflate that must produce exactly `cbUncomp` bytes (`mszip.rs`), the stored-block size check and the split-block checks (`expand_folder`). |
| D-LI6 | ~~Windows keeps FDI, and `mscab` is tested against makecab and FDI.~~ Windows still keeps FDI. | **Superseded (F-15)** | #2020 adds a Windows test of the portable decoder against FDI on a MakeCAB set, and the every-host RAR test runs FDI on the writer's output on Windows (U3). |
| D-LI7 | Add a closed `unpack_kind` code to the forwarded `install_stage_failed` and `repair_stage_failed` events. The full error text goes to local-only targets `launcher.install.detail` and `launcher.repair.detail`, which are not in `LAUNCHER_EVENT_TARGETS`. | Decided | Remote triage gets a safe code. The local log and the scrubbed debug-log upload get the text, so no path is forwarded. |
| D-LI8 | **Upgrade the host to https.** An `http://<dns-name>[:port]` login server gives the summary endpoint `https://<dns-name>/api/` (port 443). This applies only to DNS names containing a dot. IP literals, `localhost`, `*.localhost`, single-label names and URLs with credentials are refused. There is never an http fallback. | Decided (owner confirmed "any public DNS name", 2026-10-24) | The game keeps http, and summaries go to the **same operator** (same DNS name) over TLS with certificate validation. This works for any operator who serves https on the name, with no destination built into the launcher. |
| D-LI9 | The `resolve` order is: https login server (`login_server`), then the upgraded http login server (`login_server_https_upgrade`), then `DEFAULT_SUMMARY_BASE` (`default`, LT-41), then `no_https_endpoint`. LI-06 does not touch the constant. Its tests compute the expected fall-through from the constant, so LT-41 can land before or after LI-06. | Decided | This removes the merge hazard in F-09. |
| D-LI10 | The integration branch is abandoned. LI-08 and LI-07 ship as **one PR** from `main` (branch `…/li-07`). LI-05, LI-06 and LI-09 are unchanged: LI-05 and LI-06 ship as one PR each, and LI-09 as its own PR after LI-06 (D-LI17). The integration branch **was pushed to `origin`** (tip `9d61134f7`, the LI-01 merge). Close-out deletes it from the remote and locally, together with the local `…/li-01`, `…/li-02` and `…/li-03`. `ops` cannot delete branches, so hands-on or the owner does it. | Decided (ruling LI-02-ruling-1) | #2020 and #2023 delivered track A. What is left is a docs fix and the real-client gate, and those form one revert unit. |
| D-LI11 | "Never plain http" also covers game telemetry. `EndpointPolicy` accepts https anywhere and http only to loopback (D-LI14). An http login server no longer vouches for http to its host and port. | Decided (owner, 2026-10-24; was OQ-1) | Owner decision. Packet LI-09. |
| D-LI12 | macOS keeps the Wine helper for the cabinet set. Changing the macOS path is out of scope. | Decided (owner, 2026-10-24; was OQ-2) | Owner decision. LI-08's docs say so. |
| D-LI13 | Does LT-41's built-in default still ship, and should it apply to third-party http servers whose host has no https? (That would send their players' summaries to a different operator.) | **BlockedDecision** (OQ-3) | Owner, in the launcher-telemetry campaign. LI-06 works either way. |
| D-LI14 | Game telemetry still accepts plain http to **loopback**: `localhost`, 127.0.0.0/8 and `::1` literals, with no resolver. | Decided (architect; the owner may confirm) | The bytes never leave the machine, so no network sees plaintext. The default auth URL (`http://localhost:8443/api`) and local dev servers keep working. Removing loopback would bring back the "every session fails before sending a byte" bug that the module docs describe. |
| D-LI15 | One upgrade rule. LI-09 moves LI-06's check into `launcher-core` as `telemetry_endpoint::https_upgrade_origin(&str) -> Option<String>`. LI-06's `upgraded_from_http_login_server` delegates to it, and game telemetry's mint, refresh and upload base use it (`session::telemetry_origin`). | Decided | Two copies of a security-relevant host classifier would drift. This is why LI-09 depends on LI-06. |
| D-LI16 | An http login server that cannot be upgraded (an IP or a single-label name) gets no game telemetry. A dotted LAN name such as `nas.lan` is tried over https, and a failed connection sends nothing. Its mint is refused with the existing `endpoint_refused` reason and `Outcome::EndpointRefused`. A plain-http upload URL that the server hands back is refused (`endpoint_refused`) and never rewritten. There is no http fallback anywhere. | Decided | This follows from D-LI11. LI-09 adds no new code, so it needs no new UI copy. Rewriting a URL the server named would hide a misconfigured server. |
| D-LI17 | LI-09 ships as its **own PR**, branched from `main` after LI-06 merges. Ship gate **G-LI9**: before the PR merges, `ops` runs a read-only colo check that the server's start line shows `upload_endpoint_source` as `request_host` or an https env value, with no `upload_endpoint_not_https` WARN. If the check fails, the owner fixes the colo env first. | Decided | Changing where game telemetry goes needs its own revert unit. With a plain-http env override, new launchers would refuse every upload URL, and game telemetry would stop with no error anyone sees. |

### Trust and privacy of D-LI8

- **Same operator.** The upgraded endpoint uses the login server's own DNS name,
  so whoever controls that name already receives the player's login. Summaries
  already require consent, and their content does not change.
- **TLS only.** `reqwest` validates the certificate for the name, so the request
  is encrypted and the server authenticated. If there is no https on 443, or the
  certificate is bad, the upload fails and the exporter's existing retry and
  backoff applies. Plain http is never used.
- **The residual risk** is a shared or dynamic-DNS name whose port 443 is a
  different tenant. Summaries are consented and scrubbed (no paths, no IPs, the
  240-byte field cap), so this is accepted. `privacy-inventory.md` states the rule.
- **IPs and single-label names are refused**: there is no certificate for them.
  Dotted LAN names (`nas.lan`, `steamdeck.local`) are tried over https like any
  other dotted name, and a failed connection sends nothing (owner ruling on
  review decision D1, 2026-10-24).

### Game telemetry after LI-09

- **Same operator, TLS only.** The same rule (D-LI15) sends game telemetry to
  `https://<name>` on 443, the name the player already logs in to.
- **IP and single-label servers lose game telemetry** until they serve https.
  Dotted LAN names are tried over https, and a failed connection sends nothing.
  The mint says `endpoint_refused` for the first two. This is the cost of D-LI11, and LI-09's docs state it.
- **The colo's Caddy route carries all public game telemetry** from LI-09 on.
  OPS-LT1 (the Caddyfile surviving a `board-proxy` redeploy) becomes
  load-bearing. Older launchers keep using plain `:8081`.

## Packets

| Packet | Wave | Summary | Depends on | Status |
|---|---|---|---|---|
| LI-01 | - | ~~`mscab` header parser and MSZIP decoder~~ (F-15) | - | Superseded (merged only into the abandoned integration branch) |
| LI-02 | - | ~~Test writer for a spanning MSZIP set~~ (F-15) | - | Superseded (closed at `664e59001`, never merged) |
| LI-03 | - | ~~`mscab` plan~~ (F-15) | - | Superseded (`8d31da787`, never merged) |
| LI-04 | - | ~~`mscab` extraction and wiring~~ (F-15). Its `SGW.exe` size assert moved to LI-07. | - | Superseded |
| LI-05 | 1 | `unpack_kind` code and local-only detail events on install and repair stage failures | - | Done (#2025) |
| LI-06 | 1 | https upgrade of an http login server for the summary endpoint | - | Done (#2026) |
| LI-09 | 2 | Game telemetry never over plain http: one shared https-upgrade rule in `launcher-core`, a tightened `EndpointPolicy`, and mint, refresh and upload over https | LI-06 merged to `main` (D-LI15); ship gate G-LI9 | Review: 3d2c72a34, packet review ship; waits for G-LI9 |
| LI-08 | A | Docs: the desktop README and wine-validation say Linux expands natively (`cab_portable`) | - | Done (#2027) |
| LI-07 | A | The `SGW.exe` size assert in `real_client_rar`, then the real-client run on the Deck, written to a worknote | The LI-08 commit on `…/li-07`. Precondition: the owner's real client RAR on this Deck, path in `SGW_CLIENT_RAR` | Done (#2027) |

Wave A is track A: LI-08 then LI-07 on one branch from `main`, shipped as one PR.

Status values: Ready / BlockedDependency / BlockedDecision / Writing / Review /
Integrated / UATPending / Done / Superseded.

## Open decisions and gates

- **OQ-3 (D-LI13), BlockedDecision.** LT-41's default, in the launcher-telemetry
  campaign. No packet here is blocked by it.
- **D-LI14, confirmable.** Loopback http stays allowed for game telemetry. This
  does not block anything.
- **G-LI9, ship gate.** Before LI-09 merges, the colo must advertise https
  upload URLs (D-LI17).
- **LI-07 precondition.** The owner's real client RAR must be on this Deck,
  with its path in `SGW_CLIENT_RAR`. Without it LI-07 returns `blocked`.
- **Owner note (no decision):** "track A was already delivered by
  #2020/#2023; the campaign drops LI-01..LI-04".
- Resolved 2026-10-24: OQ-1 (it became D-LI11 and LI-09), OQ-2 (D-LI12, macOS
  keeps Wine) and the scope of D-LI8 (any public DNS name).

## UAT (close-out, doc-update-map row 53)

| Row | Step | Who |
|---|---|---|
| U1 | On the Steam Deck, install from the local `client.rar` in the launcher. Install completes, Play is offered, and the game starts to the login screen. | Owner |
| U2 | With summary consent on, run the launcher on the Deck. The local log shows `summary_endpoint_resolved outcome=login_server_https_upgrade`, and rows arrive at `launcher.summary` (the `telemetry-triage` skill). | Owner, then a triage agent |
| U3 | Windows CI on `main` runs #2020's portable-against-FDI test, and `installer_rar_from_portable_cabinets_expands_on_every_host` runs FDI on the writer's output. `ops` confirms both are green. | `ops` (CI read) |
| U4 | On the Deck, with game diagnostics on, a build with LI-09 and the login server `http://play.cimmeria.app:8081`, press Play. The session mints, with no `game_session_mint_failed`. Game-telemetry rows for that session arrive in SigNoz. The server logs the mint with `upload_endpoint_source=request_host`, so it came through the https route. | Owner, then a triage agent |

## Close-out

Status docs change once (row 43): `docs/project-status.md` notes that Linux
installs work from the archive without Wine, and that summaries and game
telemetry send only over https.
The campaign review loop ran three passes and closed with campaign-ruling-1 (2026-10-24): the pass-3 copy fixes and the project-status entry ship in the final PR.
Remove the worktrees first, since `git branch -D` refuses a branch that is checked
out: `git worktree remove --force .claude/worktrees/li-01` (and li-02, li-03,
li-integration; li-09 after its PR merges).
Delete the abandoned track-A branches: the local
`launcher-linux-install-telemetry/integration`, `…/li-01`, `…/li-02` and
`…/li-03`, and `…/integration` on `origin` (pushed; tip `9d61134f7`). `ops`
cannot delete branches, so hands-on or the owner does it (D-LI10).
