# LTL-B1: why the Deck was `absent` (research, read-only)

> **Type:** Reference (research finding) · **Audience:** Engineers and the owner · **Updated:** 2026-10-26
> **Companions:** [Ledger](../README.md), [work packets](../work-packets.md). Account names and addresses are removed from this copy.

Date of run: 2026-10-26. Code read at origin/main a26764cb6. Colo server build at the time: 4ed823288 (#1990); the files that matter (dev_session/*, auth client_telemetry, map_loaded.rs) are byte-identical between 4ed823288 and a26764cb6 (empty `git diff`). No IPs below.

## Verdict

**H1 is the cause (strong circumstantial; no log can show the proxy hop directly).** The Deck launcher (desktop-2026.10.2601) mints and refreshes over the https route (Caddy on loopback, LI-09 #2029). The mint handler records the stream under the socket peer (loopback), not `rate_ip`. World entry looks the stream up by the game connection's real address, finds nothing and, because other streams exist, logs `absent`. The Windows `present` sessions all came from launchers that predate LI-09 and minted over plain http straight to :8081, where the socket peer is the real address.

## Evidence

Code (all at a26764cb6):
- `crates/admin-api/src/routes/dev_session/handlers.rs:320` computes `rate_ip = rate_limit_ip(peer_ip, headers)` but `:362` (`record_unless_launcher(launcher, &claims, peer_ip)`) and `:481` (`record_stream(&new_claims, peer_ip)`, body `:505-514`, `peer_ip: Some(peer_ip)`) record the raw peer.
- `forwarded_for.rs:26-38`: the forwarded caller is only resolved when the peer is loopback.
- `crates/auth/src/auth/client_telemetry/mod.rs:171-184`: `lookup` falls to `by_machine` filtered on `s.peer_ip == Some(ip)`; no match gives `Absent` once `any_recorded` is set (`:160`).
- `map_loaded.rs:228-231` looks up with `addr.ip()` of the game socket.
- Launcher side: `crates/launcher/desktop/engine/src/storage/game_telemetry/session.rs:75-87` (`telemetry_origin`, `mint_url`) upgrades an http login server on a DNS name to `https://<name>:443`; this arrived in 0a25ef5d4 "game telemetry never goes over plain http (LI-09) (#2029)", merged 2026-10-25 11:38Z. Docs agree: `docs/operations/telemetry-upload-endpoint.md:109-113` ("Launchers from LI-09 on ... mint, refresh and upload through the https route").

SigNoz (service `cimmeria-server`, colo; `World entry complete (` rows with `client_telemetry EXISTS`, 7 d):
- Deck, 2026-10-26 12:22:36Z: `client_telemetry=absent`, no session id, world Castle_CellBlock.
- Every `present` row (10-20 12:10, 10-22 20:35 and 20:59, 10-23 00:38 to 01:01 and 16:56, 10-24 17:09 to 18:00) has a matching `Minted dev-session telemetry token` row for `session_kind=player` a few minutes earlier from a launcher at or before desktop-2026.10.2307 (git_sha de83747a1 and older; all before LI-09). Those mints have `upload_endpoint_https=false` or the attribute absent (older builds).
- The Deck's player mint: 12:13:40Z, session 1ce1e1aa, `launcher_version=desktop-2026.10.2601` (post LI-09), exp +8 h, `upload_endpoint_source=env`. It is the only player mint after LI-09 that was followed by a world entry. The Deck's launcher session 084005a4 (kind launcher, `os=linux`) minted at 12:10:01Z.
- Same public address for the Windows machine and the Deck (both runs show the same home NAT address in `addr`), so H2/NAT differences do not separate them; only the route does.
- `unknown` rows (10-20, 10-21, 10-22 early, 10-25 14:09) are "no stream recorded since boot" after server restarts, not this bug.

Caveat on one signal: `upload_endpoint_https` reflects the server's `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` value (source `env` on both), which was http on 10-24 and https by 10-26 09:31. It tells you what the server advertised for uploads, not which route the mint came in on. The route evidence is the launcher version/LI-09 date, not that field.

## Other hypotheses

- **H2 (IPv6 vs IPv4): not supported.** The game connection is IPv4; `lookup` and `record` both canonicalise. Under H1 the recorded address is loopback regardless.
- **H3 (shared empty-name machine_id): not the cause here.** A single Linux launcher minted; even with the shared id `e3b0c44298fc1c14` the stream is still recorded (`machine` non-blank) and would be found if the address matched. It remains a real collision risk for a second Linux/WSL install (LTL-B2b). The mint log does not print `machine_id` (debug only), so the Deck's actual value is not confirmed from SigNoz.
- **H4 (TTL/order): ruled out.** Mint 12:13:40Z precedes entry 12:22:36Z by about 9 min; TTL is 8 h (exp 1793045620). No refresh was needed or logged (no `Refreshed dev-session telemetry token` rows in 3 d).
- **DLL under Proton: works.** Session 1ce1e1aa (`session_kind=player`, `cimmeria-client` rows incl. native packet traces) has `upload-chunk accepted` rows at 12:24:58Z, so the DLL read the session file and uploaded. No session-file path problem.
- **by_account without an IP (D-TG19): not reachable today.** The 12:13:40Z mint log has no `account_name`; the launcher never sends one (no `account_name` in `game_telemetry` non-test code), and the player types it only at game login. Making `present` IP-free would need the DLL or upload path to tell the server the account (client-claimed), a new trust/design decision, larger than D-LTL4.

## Does the LTL-B2 pre-spec fix it?

Yes for the cause found, provided the container sees Caddy as loopback. If it sees the bridge gateway (OPS-LT2, still unverified in `telemetry-upload-endpoint.md:154-157`), `rate_limit_ip` ignores the header and the stream is recorded under the gateway address: B2 would then not help, and the quota key would also be shared. B2 is also the only unit-testable half: it cannot prove the colo topology.

## Changes to the B2 spec

1. Add a log field to the mint/refresh `Minted`/`Refreshed` rows: `caller_forwarded: bool` (true when `rate_ip != peer_ip`) and `peer_loopback: bool`. Booleans carry no address (R5 holds) and give the Deck re-run direct, SigNoz-visible proof of the proxy hop and of OPS-LT2. Without this, H1 can only be inferred, as here.
2. Add a unit test that a loopback peer with no `X-Forwarded-For` records under loopback (documents the fallback), and one for a malformed header.
3. Uat note for LTL-08: the Windows launcher and the Deck share a public address; after the fix, if both launchers have live streams (8 h TTL) the answer is `ambiguous` (candidate count 2), not `present`. Run the Deck re-run with the Windows launcher closed, or accept `ambiguous`. Also note any other machine on that NAT.
4. The module-doc edit already planned (MachineGuid vs host name) stays.

## D-LTL4

**Needed.** The fix records the forwarded caller in the in-memory registry, which amends owner directive 7a / D-LT19 ("never ... `StreamRecord`"). No fix avoids it: the only alternative that keeps the registry on socket peers would be changing the launcher back to plain http, which LI-09 forbids. The amendment is narrow (memory only, never logged), as D-LTL4 already states.

## Tests that would distinguish, if the owner wants proof before D-LTL4

- Cheapest: a one-line server deploy of the two boolean fields above, then one Deck launch; expect `peer_loopback=true caller_forwarded=true` on its mint and `absent` before B2.
- Or run the Windows launcher 2601 (post LI-09) on the same NAT for one entry: H1 predicts `absent` there too; a `present` would refute H1.
