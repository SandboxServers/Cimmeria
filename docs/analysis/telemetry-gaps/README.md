# Telemetry Gaps Campaign

> Type: how-to (campaign ledger). Audience: the coordinator session and packet workers.
> Updated: 2026-10-11 (close-out: every packet Integrated, UATPending, Merge pending, Dropped or Superseded; D-TG17 and D-TG19 recorded; [leftovers](#leftovers) listed). Status: **closed**, with the UAT owed in [unified UAT § Telemetry gaps](../../guides/unified-uat.md#telemetry-gaps) and the [close-out handoff](handoffs/2026-10-11-closeout.md). Companions: [work-packets.md](work-packets.md) (rules, contract, W0-W2), [work-packets-w3-w4.md](work-packets-w3-w4.md) (W3-W4), [reviews/](reviews/), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [negative-logging-convention.md](../../architecture/negative-logging-convention.md), [observability.md](../../architecture/observability.md), [client-telemetry.md](../../architecture/client-telemetry.md).

## Goal

Anything that went wrong in a session can be explained from SigNoz alone, without a lab replay or a guess. The trigger was the 2026-10-10 triage of seven parallel lab sessions. Several faults there could only be explained by correlating three services by hand, and some could not be explained at all.

**Packet size rule:** every packet is sized for a Haiku `packet-coder`. That means one concern, at most about 3 production files, a named test, and under about 100k tokens. Anything bigger is split before dispatch. Design-heavy or RE packets go to a domain agent instead and are marked as such.

## Findings from the 2026-10-10 triage

| ID | What we couldn't see | Fix direction | Side | Packet |
|---|---|---|---|---|
| T1 | Which messages and entities a client bundle drop lost. The server logs only `packet_bytes`. | Per-message offset, msg id and entity id on multi-message flush bundles (deferred flush and hold release only) | Server | TG-AOI-09 (after TG-AOI-04) |
| T2 | Entities the client never created. 9.4k free-text `svidFollow ... is unknown` BigWorld errors. | One `client.viewport.unknown_entity {entity_id, count, first_ts}` per entity; an ingest-side join against server introductions | Client DLL + ingest | TG-CLI-01 (the join is a SigNoz query, not code) |
| T3 | Errors caused by a misparse look like real faults ("Unexpected key!" was a misparsed tail) | Tag BigWorld messages in the same bundle as an `unpack_fault` with `after_unpack_fault` | Client DLL | TG-CLI-02, TG-MER-12 |
| T4 | Why a cooked category resyncs (ErrorStrings resent on 30 of 35 logins) | `client_version` on `cooked_data.version_reply`; DLL logs the pushed pak's cache write result and tier. Root cause fixed separately in PR #1347 | Both | TG-MER-13 (`client_version` is already on the reply; this adds the exe version and numeric `server_version`), TG-CLI-03 |
| T5 | How a session ended: clean exit, lab kill, crash or orphan. 0 `logOff` out of 27 session ends. | `client.session.end {reason}`; labd ships its log (D-TG2); UAT specs log out and disconnect end to end (S1) | Client, lab, specs | TG-NET-01, TG-NET-02, TG-MER-07, TG-PIPE-03, TG-CLI-04, TG-PIPE-12, TG-PIPE-17 |
| T6 | Lab runs and players look the same; all 8 `playtest.friction` stalls were lab runs that stop on purpose | `session_kind` on server events, client-declared and server-verified (D-TG3) | Server | TG-NET-16, TG-NET-17 |
| T7 | What a 6-15 s client hitch was doing (five clients froze during a simultaneous world load) | Main-thread phase on `client.engine.hitch` | Client DLL | TG-CLI-05 |
| T8 | A player session with server logs and zero client telemetry (2026-10-09) | At world entry, log whether the session has a client telemetry stream | Server | TG-NET-18 (join key D-TG19), TG-NET-13 |
| T9 | Noise: about 7k audio "stop `<unknown>`" WARNs, and 7,245 INFO single-packet AoI bundle rows | Resolve the cue name at start or drop the level; demote single-packet bundle rows to DEBUG | Both | TG-AOI-10, TG-CLI-06 |
| T10 | Minigame junk traffic logged with empty fields | Raw prefix and rate limit. The port itself is closed to unexpected peers in the minigame fix (D-TG1) | Server | PR #1348, TG-MG-10 |
| T11 | `hooks.iat.slot_mismatch` on `recvfrom` in every session; unknown whether receive-side telemetry is blind | Name the module that owns the slot; confirm the coverage | Client DLL | TG-CLI-07 |
| S1 | UAT specs never log out, so no run ends a session cleanly | Every spec ends with a logout and disconnect. **Corrected assertion:** `session.end disconnect_reason=logoff_character_select`, then `Client entities cleaned up disconnect_reason=logoff`; the draft's `logOff` value doesn't exist (see [shared contract](work-packets.md#session-end-one-emitter-reconciled-reasons)) | Specs + runner | TG-PIPE-07, TG-PIPE-08 (after TG-NET-01) |

Fixed outside the campaign, same triage: ErrorStrings resync (PR #1347; GitHub shows it merged, and any follow-up waits on RE), minigame port admits only expected peers (PR #1348, branch `fix/minigame-expected-peers`), #1341 residue RE (branch `re/1341-iterator-residue`), bundle replay allowlist (branch `fix/bundle-replay-allowlist`, packet TG-PIPE-01), Discord webhook URL kept out of errors (PR #1349, packet TG-SOC-01).

## System reviews

Each domain agent adversarially reviewed its own system's telemetry footprint, positive (what fires when things work) and negative (what fires when an expectation fails, per the negative-logging convention). Each review also covers the system's intersections up to two hops away. The reviews are in [reviews/](reviews/), one file per agent. The coordinator merged their candidate packets into [work-packets.md](work-packets.md) and [work-packets-w3-w4.md](work-packets-w3-w4.md).

| Review | Agent | Status |
|---|---|---|
| AoI and witness | aoi-witness-broadcast | Done |
| Engine and Mercury | bigworld-engine-advisor | Done |
| Combat | combat-systems-advisor | Done |
| Items | items-systems-advisor | Done |
| Minigames | minigame-systems-advisor | Done |
| Missions and content | mission-systems-advisor | Done |
| Movement and teleport | movement-teleport-advisor | Done |
| NPC AI and spawns | npc-ai-spawn-advisor | Done |
| Social | social-systems-engineer | Done |
| Auth and network | network-security-auth | Done |
| Persistence | database-persistence | Done |
| Telemetry pipeline and lab | general-purpose | Done |

**Collation notes.** Duplicates merged: TG-PIPE-09 into TG-NET-01; TG-DB-10 into TG-NET-02; TG-NET-11 into TG-MER-10; the `connect_loop/mod.rs:88` half of TG-NET-06 into TG-MER-11, and TG-NET-07 into TG-NET-06; TG-MOV-13 into TG-CMB-10; TG-AOI-03 into TG-ITM-01; TG-ITM-08's bandolier half into TG-CMB-05. Split: TG-ITM-01's behaviour change became TG-ITM-14. Added for findings no review packeted: TG-AOI-09 (T1), TG-AOI-10 (T9), TG-MER-13 (T4), TG-MER-14, TG-NET-16/17 (T6), TG-NET-18 (T8), TG-PIPE-17 (D-TG2), TG-CLI-01..07 (T2-T5, T7, T9, T11). Deferred: TG-MIS-04, TG-NPC-14, TG-SOC-16 (see [Not packeted](work-packets.md#not-packeted)).

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-TG1 | The minigame listener admits only peers whose IP matches a registered (pending or connected) minigame session | Decided 2026-10-10 | Owner: nobody calls the port directly who isn't already talking to us |
| D-TG2 | labd ships its log to the colo SigNoz as `service.name=cimmeria-lab`, with local paths reduced to the instance name | Decided 2026-10-10 | Makes #1342 (orphan SGW.exe) and unexplained short sessions visible |
| D-TG3 | `session_kind` is client-declared and server-verified: trusted only for accounts flagged as test accounts | Decided 2026-10-10 | A player can't hide from friction alerts by claiming to be the lab |
| D-TG4 | Client DLL packets are in scope and ship with the next signed launcher release, after a lab load check | Decided 2026-10-10 | Server packets merge independently |
| D-TG5 | Purge the SigNoz rows that hold leaked secrets: 541 `*-keys.txt` lines and 14 `current-session.json` token lines from bundle replay (2026-09-29 to 10-06), and the 2 Discord error rows with the webhook URL (2026-10-04)? | Decided 2026-10-10: purge | Owner chose to purge. A targeted `ALTER TABLE signoz_logs.logs_v2 DELETE` was run that day for `launcher.client_log` rows whose `source_file` ends in `-keys.txt` or `current-session.json` (1,164 rows over all time) and the 2 webhook rows |
| D-TG6 | Rotate the Discord webhook, whose token sits in retained logs? | Owner action | The owner rotates the webhook; PR #1349 only stops future leaks |
| D-TG7 | TG-MER-09: reverse NA25 and move per-datagram `mercury.packet` to a TRACE firehose with a 1-in-53 sample and a counter? | Decided 2026-10-10: sample, keep anomalies | Keep every anomalous row (retransmit, duplicate, gap, error). Sample routine per-datagram rows per channel, with a per-minute count summary so rates stay exact |
| D-TG8 | TG-DB-11: demote `sqlx::query` to INFO once TG-DB-06's slow-statement WARN lands, losing per-statement `elapsed_secs` history? | Decided 2026-10-10: demote after TG-DB-06 | Ship the slow-statement WARN first, then move routine statement rows off the colo export |
| D-TG9 | TG-SOC-03: on character delete, refund held bids and return mail attachments by mail first, or accept destruction with TG-SOC-02 as the audit? | Decided 2026-10-10: accept destruction, audit it | Owner keeps the cascade. TG-SOC-02 logs every destroyed listing, held bid and attachment so it can be restored by hand; TG-SOC-03 is dropped |
| D-TG10 | TG-SOC-04: should presence (login, level, death, gate) skip players who put the subject on their Ignore list, and should trade honour the Ignore list too? | Decided 2026-10-10: yes to both | Presence skips ignorers. A trade request from an ignored player is refused, with visible feedback to the requester |
| D-TG11 | TG-ITM-14: when the player load fails, abort the appearance refresh instead of caching and broadcasting the default "naked human male" model? | Decided 2026-10-10: retry once, then keep last good | On a failed load, retry once after a short delay. If it fails again, keep broadcasting the previous appearance and WARN with the player and the load error |
| D-TG12 | TG-ITM-07: is a free repair or recharge with no trailing vendor template id legitimate, or an authority hole to close first? | Decided 2026-10-10: authority hole | The stock client never sends a template id (static RE, `inventory-wire-formats.md`), so the server parser was wrong for all five vendor methods and the free path was reachable only by a crafted packet. Fixed outside the campaign on branch `fix/vendor-wire-stock-layout`: stock layouts parsed, vendor taken from the server session, free branches deleted. TG-ITM-07 is superseded |
| D-TG13 | Server-side `session_kind` comes from the account's test flag alone, since the game client has no way to declare it on the wire without a client patch. Accept? | Decided 2026-10-10: account flag alone (revises D-TG3) | A seeded test-account flag marks lab accounts; server events for their sessions carry `session_kind=lab`. Client telemetry keeps its own declared kind |
| D-TG14 | TG-NET-18: which join key ties a game session to its client telemetry stream (account on the dev-session mint, or something else)? | Decided 2026-10-10: account name in the launcher token. **Superseded by D-TG19** | The launcher adds the account name to the telemetry session token it mints, and ingest joins on it. Needs a launcher release; the account name enters the telemetry claims. Unreachable as decided: the launcher mints its token before the game client logs in, so it never knows the account name |
| D-TG15 | TG-DB-02: how long may shutdown wait to drain position and ammo saves inside the container stop grace, and in what stop order? | Decided 2026-10-10: up to 10 s | Save every in-world player in parallel. After 10 s, shut down anyway and WARN once per player not saved, with player_id and name |
| D-TG16 | Add a periodic counter of OTLP records the exporter dropped, accepting an exporter-side change? | Decided 2026-10-10: yes; never packeted, see [leftover L2](#leftovers) | A periodic dropped-records counter row, written locally and to SigNoz once the exporter recovers |
| D-TG17 | TG-MOV-09 (after the RE): keep sending method 116 to `SGWGmPlayer` clients, skip it, or route the streaming hint another way? | Decided 2026-10-11 (owner): option A, keep sending it | The RE ([method-116-on-teleport.md](../../reverse-engineering/findings/method-116-on-teleport.md), #1611) found method 116 unbound on every client class, not only `SGWGmPlayer`, so the GM framing was wrong and skipping it for one class buys nothing. The wire is unchanged; the forced-position row's hint is relabelled `streaming_hint = "method_116_unbound"`, and the admin-api ingest replays a dropped `onPlayerTeleport` at INFO (`drop_class = "known_unbound"`) instead of WARN. Code in #1642 |
| D-TG18 | When the player load degrades (TG-DB-07), refuse world entry or suppress mission persistence for that session? | Decided 2026-10-10: enter with missions locked | Retry the saved-missions read a few times. If it still fails, enter the world with those missions marked unknown, refuse accepting or advancing them with visible feedback and a WARN, and keep retrying in the background. Unlock once the read succeeds. Nothing overwrites completed progress |
| D-TG19 | TG-NET-18: D-TG14's join key is unreachable (the launcher mints its telemetry token before the game client logs in, so it never knows the account). Bind the account after login with a new client-to-launcher signal, join on something else, or accept `unknown`? | Decided 2026-10-11 (owner): machine id plus minting address (supersedes D-TG14) | Join pre-account launcher telemetry on a machine identifier, a hash of the Windows `MachineGuid` sent as the `mid` claim of the telemetry token, plus the address that minted it. No new client signal and no account name in the telemetry claims. Shipped in #1628; the launcher half reaches players with the next signed launcher release |

## Packet status

States at close-out (2026-10-11): **Integrated** (merged to `main`, with its PR), **UATPending** (merged, but it reaches players only with the next signed launcher release, after a lab load check, D-TG4), **Merge pending** (in an open PR at close-out), **Dropped**, **Superseded**. Lab UAT was skipped for this campaign by the owner; what a tester still owes is in [unified UAT § Telemetry gaps](../../guides/unified-uat.md#telemetry-gaps). Specs: W0-W2 in [work-packets.md](work-packets.md), W3-W4 in [work-packets-w3-w4.md](work-packets-w3-w4.md).

**Totals:** 161 packets (the 160 collated, plus TG-DB-12). 143 Integrated, 9 UATPending (client DLL and launcher, merged), 7 Merge pending in #1645 (the 5 DLL packets among them then become UATPending), 1 Dropped, 1 Superseded.

| ID | Title | Wave | Sev | State |
|---|---|---|---|---|
| TG-PIPE-01 | Bundle replay allowlists log files | W0 | high | Integrated (#1351) |
| TG-SOC-01 | Strip the webhook URL from Discord send errors | W0 | high | Integrated (#1349) |
| TG-NET-08 | `admin.request` row for every admin API request | W0 | high | Integrated (#1378, #1384) |
| TG-DB-01 | Shutdown logs each in-world player's unsaved state | W0 | high | Integrated (#1372) |
| TG-DB-02 | Flush positions and ammo on shutdown | W0 | high | Integrated (#1388) |
| TG-DB-03 | Cell position-save skips and queues are logged | W0 | high | Integrated (#1367, #1384) |
| TG-DB-04 | `PersistPosition` rows: identity, Pattern B, reason | W0 | med | Integrated (#1370) |
| TG-DB-07 | Player-load fallbacks say they degraded | W0 | med | Integrated (#1375) |
| TG-DB-12 | Lock unreadable missions instead of overwriting them (D-TG18) | W0 | high | Integrated (#1394) |
| TG-ITM-06 | Stale bandolier ammo writeback to WARN with counts | W0 | med | Integrated (#1377) |
| TG-ITM-11 | Bandolier flush summary and dropped-slot trace | W0 | med | Integrated (#1385) |
| TG-ITM-09 | Loot silent drops and lost cash | W0 | med | Integrated (#1376) |
| TG-ITM-03 | Outbox replay row and identity on outbox WARNs | W0 | med | Integrated (#1380) |
| TG-SOC-02 | `character.delete_cascade` row | W0 | high | Integrated (#1382) |
| TG-SOC-03 | Character delete: refund or destroy | W0 | high | Dropped (D-TG9: accept destruction; TG-SOC-02 is the audit) |
| TG-MIS-07 | Log dropped deferred content actions | W0 | med | Integrated (#1368) |
| TG-MER-08 | Delete the per-packet encrypt/decrypt TRACE rows | W1 | high | Integrated (#1437) |
| TG-MER-09 | `mercury.packet` to the firehose plus a counter | W1 | high | Integrated (#1471) |
| TG-NPC-01 | Sample unwitnessed Patrol/Wander tick rows | W1 | high | Integrated (#1469) |
| TG-NPC-02 | Gate `movement.npc` step rows on witnesses | W1 | high | Integrated (#1464) |
| TG-NPC-03 | Drop `no_candidates` rows with zero witnesses | W1 | high | Integrated (#1472) |
| TG-NPC-09 | Sample unwitnessed leg and path-request rows | W1 | med | Integrated (#1494) |
| TG-MER-10 | Throttle WSAECONNRESET | W1 | low | Integrated (#1427) |
| TG-SOC-11 | Discord config watcher self-trigger | W1 | med | Integrated (#1467) |
| TG-AOI-10 | Demote single-packet AoI bundle rows | W1 | med | Integrated (#1455) |
| TG-MOV-11 | Drop the per-packet `EntityMove` TRACE | W1 | low | Integrated (#1492) |
| TG-MOV-03 | Speed warn: separate bunched packets | W1 | med | Integrated (#1463) |
| TG-MOV-10 | GM navmesh bypass: INFO, throttled | W1 | low | Integrated (#1493) |
| TG-MOV-14 | Dial-hub GM grant WARN to INFO | W1 | low | Integrated (#1419) |
| TG-CMB-04 | Quiet the boot `effect_script_unregistered` WARN | W1 | med | Integrated (#1462) |
| TG-CMB-09 | Demote NPC-only launch and kill INFO rows | W1 | low | Integrated (#1470) |
| TG-SOC-13 | Mail sweep: no row when nothing was scanned | W1 | low | Integrated (#1468) |
| TG-MIS-12 | Demote no-listener cover rows; collapse `add_dialog_set` | W1 | low | Integrated (#1461) |
| TG-MER-14 | Cooked-sync logout race to DEBUG | W1 | low | Integrated (#1465) |
| TG-DB-11 | Demote the `sqlx::query` statement rows | W1 | low | Integrated (#1629) |
| TG-NET-01 | `session.end` on logOff to character select | W2 | high | Integrated (#1484) |
| TG-NET-02 | End-cause hints on `session.end` | W2 | high | Integrated (#1549) |
| TG-MER-07 | Channel health on `session.end` | W2 | med | Integrated (#1594) |
| TG-NET-03 | Duplicate-login liveness | W2 | high | Integrated (#1486) |
| TG-NET-10 | `session.end server_shutdown` at stop | W2 | med | Integrated (#1548, #1564) |
| TG-NET-14 | Inactivity-timeout row context | W2 | med | Integrated (#1608) |
| TG-NET-13 | Dev-session refusals no longer dropped | W2 | med | Integrated (#1487) |
| TG-NET-15 | `session.start` says login or travel | W2 | low | Integrated (#1481) |
| TG-NET-16 | Test-account flag on `account` | W2 | med | Integrated (#1485) |
| TG-NET-17 | `cimmeria.session_kind` on server session rows | W2 | high | Integrated (#1578) |
| TG-NET-18 | World entry says whether client telemetry exists | W2 | med | UATPending (#1628; server half live, the launcher's `mid` claim ships with the next launcher release; D-TG19) |
| TG-PIPE-17 | labd ships its log to SigNoz | W2 | high | Integrated (#1490) |
| TG-PIPE-05 | labd launch, stop and pid-overwrite rows | W2 | med | Integrated (#1489) |
| TG-PIPE-06 | labd per-tool-call row | W2 | med | Integrated (#1480) |
| TG-PIPE-11 | lab-mcp audit: error and duration | W2 | low | Integrated (#1479) |
| TG-PIPE-07 | `lab_disconnect` flow | W2 | med | Integrated (#1488) |
| TG-PIPE-08 | S1: runner ends every run with logout and disconnect | W2 | med | Integrated (#1618) |
| TG-AOI-01 | Cap drop names the entity and kind | W3 | high | Integrated (#1514) |
| TG-AOI-02 | Appearance rebroadcast success row | W3 | med | Integrated (#1528) |
| TG-AOI-04 | Flush-source `send_kind` | W3 | med | Integrated (#1579) |
| TG-AOI-09 | Per-message manifest on flush bundles | W3 | high | Integrated (#1633) |
| TG-AOI-05 | Classify `not_in_witness_aoi` | W3 | med | Integrated (#1529) |
| TG-AOI-06 | Close the cinematic-hold lifecycle | W3 | low | Integrated (#1613) |
| TG-AOI-07 | AoI counters and witness-set gauge | W3 | med | Integrated (#1552) |
| TG-AOI-08 | Leave and invisible send-failure row | W3 | low | Integrated (#1543) |
| TG-MER-01 | Numeric identity on `mercury.reliable_send` | W3 | high | Integrated (#1499) |
| TG-MER-02 | Cause fields on `mercury.retransmit` | W3 | high | Integrated (#1614) |
| TG-MER-03 | Channel log identity | W3 | high | Merge pending (#1645) |
| TG-MER-04 | Identity on `tx_hole` and `rx_order` | W3 | med | Merge pending (#1645) |
| TG-MER-05 | Base→cell send seam WARNs | W3 | high | Integrated (#1512) |
| TG-MER-06 | Inbound bundle-walk drops out of TRACE | W3 | med | Integrated (#1615) |
| TG-MER-11 | Identity on the catch-all handler rows | W3 | low | Integrated (#1587) |
| TG-MER-13 | Client version and cooked-reply fields | W3 | low | Integrated (#1632) |
| TG-CMB-01 | Target-scan blind spot; pin `player.respawn` | W3 | high | Integrated (#1498) |
| TG-CMB-02 | Stale combat-state detector | W3 | high | Integrated (#1508) |
| TG-CMB-03 | `effect_inert` WARN | W3 | med | Integrated (#1523) |
| TG-CMB-05 | `bandolier` rows: Rule 5 and numeric ids | W3 | med | Integrated (#1596) |
| TG-CMB-06 | Death and revive rows: Rule 5/6 keys | W3 | med | Integrated (#1589) |
| TG-CMB-07 | No-witness WARNs say why a player counted | W3 | med | Integrated (#1530) |
| TG-CMB-08 | Proximity aggro: combat flag not announced | W3 | low | Integrated (#1590) |
| TG-CMB-10 | Log a failed `onEndAidWait` send | W3 | low | Integrated (#1541) |
| TG-ITM-01 | Appearance refresh outcome rows | W3 | high | Integrated (#1509) |
| TG-ITM-14 | Keep the last good appearance on a failed load (D-TG11) | W3 | high | Integrated (#1595) |
| TG-ITM-02 | Loot roll outcome row | W3 | high | Integrated (#1497) |
| TG-ITM-04 | Inline move refusals get event and reason | W3 | med | Integrated (#1531) |
| TG-ITM-05 | After-commit: no silent drops | W3 | med | Integrated (#1522) |
| TG-ITM-07 | Free vendor repair and recharge via `VendorLog` | W3 | med | Superseded (#1381: the free path no longer exists, and `paid_repair`/`paid_recharge` already log `VendorLog` rows; D-TG12) |
| TG-ITM-08 | Numeric ids and identity on `item_sequence` | W3 | med | Integrated (#1540) |
| TG-ITM-10 | Remove rows: origin and stack counts | W3 | med | Integrated (#1544) |
| TG-ITM-12 | Grant refusal levels | W3 | low | Integrated (#1546) |
| TG-ITM-13 | Double-click `useItem` level; entry identity | W3 | low | Integrated (#1547) |
| TG-MG-01 | Base minigame seam: lost result, disabled server | W3 | high | Integrated (#1501) |
| TG-MG-02 | MinigamePlayer stubs log every call per the .def | W3 | high | Integrated (#1598) |
| TG-MG-03 | Session end reason, outcome and duration | W3 | med | Integrated (#1516) |
| TG-MG-04 | Duplicate-session row says why | W3 | med | Integrated (#1582) |
| TG-MG-05 | Livewire rejection rows name the player | W3 | med | Integrated (#1545) |
| TG-MG-06 | `minigame.connection` span | W3 | med | Integrated (#1588) |
| TG-MG-07 | Victory rows carry `validation` | W3 | low | Integrated (#1623) |
| TG-MG-08 | Login row: peer, player, ticket age | W3 | low | Integrated (#1537) |
| TG-MG-09 | Cell result for an unknown entity | W3 | low | Integrated (#1542) |
| TG-MG-10 | `pending_sessions` on the D-TG1 refusal | W3 | low | Integrated (#1624) |
| TG-MIS-01 | Identity and `event=` on mission lifecycle rows | W3 | high | Integrated (#1513) |
| TG-MIS-02 | Identity and context on friction rows | W3 | high | Integrated (#1593) |
| TG-MIS-03 | `mission.step_anchor` row at step activation | W3 | high | Integrated (#1602) |
| TG-MIS-05 | `missions_restored` login snapshot | W3 | med | Integrated (#1642) |
| TG-MIS-06 | Executor mission rows carry `player_id` | W3 | med | Integrated (#1563) |
| TG-MIS-08 | WARN on dropped mission client frames | W3 | med | Integrated (#1586) |
| TG-MIS-09 | Numeric ids on `content.resolve` | W3 | med | Integrated (#1559) |
| TG-MIS-10 | Malformed-args and missing-player rows | W3 | low | Integrated (#1562) |
| TG-MIS-11 | Base mission persist rows | W3 | low | Integrated (#1642) |
| TG-MIS-13 | OTEL row for content-engine conditions | W3 | low | Integrated (#1609) |
| TG-MOV-01 | Truthful, joinable forced-position row | W3 | high | Integrated (#1510) |
| TG-MOV-02 | `movement.teleport` row for authorized moves | W3 | high | Integrated (#1631) |
| TG-MOV-04 | Ring passenger release row | W3 | med | Integrated (#1526) |
| TG-MOV-05 | Ring FSM transition rows | W3 | med | Integrated (#1565) |
| TG-MOV-06 | Split `gm/travel.rs` | W3 | low | Integrated (#1570) |
| TG-MOV-07 | Rule 5 on GM travel rows | W3 | med | Integrated (#1592) |
| TG-MOV-08 | Content teleport identity | W3 | med | Integrated (#1558) |
| TG-MOV-09 | Method 116: RE the drop, then D-TG17 | W3 | med | Integrated (RE #1611; D-TG17 code #1642) |
| TG-MOV-12 | `navmesh_missing` counter by world | W3 | low | Integrated (#1554) |
| TG-NPC-04 | A death that will not respawn says so | W3 | med | Integrated (#1584) |
| TG-NPC-05 | NPC attack-not-fired: event, throttle | W3 | med | Integrated (#1519) |
| TG-NPC-06 | Instance population summary | W3 | med | Integrated (#1553) |
| TG-NPC-07 | `player_id` holding an entity id | W3 | med | Integrated (#1585) |
| TG-NPC-08 | Target player identity on leash and stuck | W3 | med | Integrated (#1566) |
| TG-NPC-10 | Cover decisions and `path_fail` attributable | W3 | low | Integrated (#1557) |
| TG-NPC-11 | Zone on tick rows; aggro outcome | W3 | low | Integrated (#1580) |
| TG-NPC-12 | Respawn tick negative gaps | W3 | low | Integrated (#1561) |
| TG-NPC-13 | DoT kill-credit miss WARN | W3 | low | Integrated (#1568) |
| TG-NPC-15 | `spawner.npc_behaviour` field hygiene | W3 | low | Integrated (#1583) |
| TG-SOC-04 | Presence skips Ignore-list watchers | W3 | high | Integrated (#1616; presence half of D-TG10 only, the trade half is a [leftover](#leftovers)) |
| TG-SOC-05 | Presence fan-out counts | W3 | med | Integrated (#1581) |
| TG-SOC-06 | Cell trade events and cancel reason | W3 | med | Integrated (#1518) |
| TG-SOC-07 | Spatial chat send failures and identity | W3 | med | Integrated (#1556) |
| TG-SOC-08 | Base chat malformed and relay refusals | W3 | med | Integrated (#1567) |
| TG-SOC-09 | Cell contact-list short-args rows | W3 | med | Integrated (#1572) |
| TG-SOC-10 | Contact header ops events and reasons | W3 | med | Integrated (#1576) |
| TG-SOC-12 | Discord drop accounting | W3 | med | Integrated (#1560) |
| TG-SOC-14 | User-channel teardown leaves | W3 | low | Integrated (#1610) |
| TG-SOC-15 | Death presence without a character name | W3 | low | Integrated (#1617) |
| TG-NET-04 | Phase 1/2 refusals log a reason | W3 | med | Integrated (#1574) |
| TG-NET-05 | Reaper logs each unconsumed ticket and SID | W3 | med | Integrated (#1573) |
| TG-NET-06 | Phase 3 refusal context; account-arm identity | W3 | med | Integrated (#1630) |
| TG-NET-09 | GM gate and lab console identity and reason | W3 | med | Integrated (#1569) |
| TG-NET-12 | Rate-limited unknown-peer datagram row | W3 | low | Integrated (#1622) |
| TG-DB-05 | Slow cell-to-base dispatch WARN | W3 | med | Integrated (#1621) |
| TG-DB-06 | Pool configuration, saturation and slow statements | W3 | med | Integrated (#1577) |
| TG-DB-08 | Character-create refusals carry event and reason | W3 | med | Integrated (#1575) |
| TG-DB-09 | Startup cache-load failures and summary | W3 | low | Integrated (#1642) |
| TG-PIPE-10 | Busy refusal names its pool | W3 | med | Integrated (#1591) |
| TG-PIPE-15 | Bundle lines carry a parsed level | W3 | low | Integrated (#1642) |
| TG-PIPE-16 | `entity_labels_unavailable` carries `session_id` | W3 | low | Integrated (#1571) |
| TG-PIPE-02 | DLL sends its pid with every chunk | W4 | high | UATPending (#1597) |
| TG-PIPE-04 | Ingest stamps `client_pid` (server) | W4 | high | Integrated (#1605) |
| TG-PIPE-03 | DLL final flush at process exit | W4 | high | Merge pending (#1645), then UATPending |
| TG-CLI-04 | `client.session.end` from the DLL | W4 | high | Merge pending (#1645), then UATPending |
| TG-PIPE-12 | Launcher `session_meta kind=game_exited` | W4 | low | UATPending (#1601; the egui launcher sends `kind=game_exited`, not the spec's `ended`) |
| TG-PIPE-13 | DLL upload-failure accounting | W4 | low | Merge pending (#1645), then UATPending |
| TG-PIPE-14 | DLL boot failures as events | W4 | low | Merge pending (#1645), then UATPending |
| TG-MER-12 | Client join keys on Mercury faults | W4 | med | UATPending (#1607) |
| TG-CLI-01 | `client.viewport.unknown_entity` per entity | W4 | high | UATPending (#1603) |
| TG-CLI-02 | Tag BigWorld errors after an unpack fault | W4 | med | Merge pending (#1645), then UATPending |
| TG-CLI-03 | Cooked pak cache write result and tier | W4 | med | UATPending (#1604) |
| TG-CLI-05 | Main-thread phase on hitch rows | W4 | med | UATPending (#1600) |
| TG-CLI-06 | Audio stop names its cue | W4 | med | UATPending (#1606) |
| TG-CLI-07 | IAT slot mismatch names its owner | W4 | med | UATPending (#1599) |

## Leftovers

Follow-ups the campaign did not close. None blocks the close-out. Each entry has enough context to become its own issue; the issue column is filled in when one exists.

### Behaviour and decided work

| # | Item | Context | Owner / agent | Issue |
|---|---|---|---|---|
| L1 | Trade honours the Ignore list (trade half of D-TG10) | TG-SOC-04 (#1616) shipped only the presence half. D-TG10 also says a trade request from a player on the target's Ignore list is refused, with visible feedback to the requester on the first press. Today trade never consults the Ignore list (the social review's "Not packeted" row). Needs its own packet with a live-DB guard and a feedback line. | social-systems-engineer | -- |
| L2 | OTLP exporter dropped-records counter (D-TG16) | Decided 2026-10-10 (yes), never packeted. A periodic counter row of OTLP records the exporter dropped, written locally and sent to SigNoz once the exporter recovers. Accepts an exporter-side change. | rust-gameserver-dev | -- |
| L3 | Merge #1645 | TG-MER-03/04, TG-PIPE-03, TG-CLI-04, TG-PIPE-13/14 and TG-CLI-02 were in #1645, still open at close-out. When it merges, flip those rows to Integrated (the MER pair) or UATPending (the five DLL packets). | coordinator | #1645 |

### Code and test hygiene from packet reviews

| # | Item | Context | Owner / agent | Issue |
|---|---|---|---|---|
| L4 | `content.resolve` names its source entity (TG-MIS-09 follow-up) | #1559 made `entity_id` numeric but left it `// nt:id-only`. Add the source entity's name to `ExecutionContext`, set it in `populate_world_context`, emit `entity_name` on the `content.resolve` rows and drop the marker. Also add a test for the `filtered_out` row and hoist the repeated `world` lookup. | packet-coder | -- |
| L5 | Discord drop accounting gaps (TG-SOC-12 follow-up) | #1560: assert the counter delta on the queue-full path. `SenderStats.dropped_closed` and `no_webhook` disagree with the `discord_events_total{outcome}` counter; document the difference or make them agree. | packet-coder | -- |
| L6 | Spatial chat broadcast row test (TG-SOC-07 follow-up) | #1556: assert the broadcast row's `ignored` count and identity fields, not only the send-failure WARN. | packet-coder | -- |
| L7 | `load_failed` arm of the navmesh counter (TG-MOV-12 follow-up) | #1554 tests only `reason=missing`; add a test for `movement_navmesh_missing_total{reason=load_failed}`. | packet-coder | -- |
| L8 | NPC respawn target-clear WARN interpolates the error | `crates/cell/src/cell/service/ticks/npc_respawn/mod.rs`: "onTargetUpdate(0) could not be enqueued ({e})" should log `error = %e` as a field, per the negative-logging convention. Found reviewing TG-NPC-12. | packet-coder | -- |
| L9 | Unauthenticated auth refusals need a throttle (TG-NET-04 follow-up) | The INFO refusal rows for `unknown_sku`, `plaintext_length`, `sid_missing` and `sid_unknown`, and the `no_shards` WARN, come from unauthenticated peers, so one peer can flood them. Add a Pattern D throttle keyed on `peer_ip`, and set the span's `result` field on refusals. | network-security-auth | -- |
| L10 | Contact-list header ops: remaining `{e}` rows and test layout (TG-SOC-10 follow-up) | `crates/base-session/src/base/contact_list/handlers/header_ops.rs` still interpolates `{e}` in five DB-error rows (delete, rename, rename reload, flags update, flags reload); make each `error = %e`. Its tests live in `header_ops/tests.rs` beside a non-`mod.rs` `header_ops.rs`, which breaks the repo's `foo/mod.rs` style: move it to `header_ops/mod.rs`. | packet-coder | -- |
| L11 | `loot_table_id` on `spawner.npc_behaviour` (TG-NPC-15 follow-up) | `crates/cell-world/src/cell/space_manager/npc_population.rs` still logs `loot_table_id = ?e.loot_table_id` (a `Some(n)` string), marked `nt:id-only` because the NameBook has no loot-table lookup. Log the bare `Option`, and pair a name once loot tables are named. | packet-coder | -- |
| L12 | `cell_to_base_closed` versus `base_channel_closed` | The campaign's contract spells the reason `base_channel_closed`. `cell_to_base_closed` remains in `consumable_use.rs`, `ability_grant.rs`, `ability_granter.rs`, `tutorial.rs` and `ring_transport/dispatch.rs` (cell-content), and in 34 other files under `crates/` (39 in all on 2026-10-11, most of them older than the campaign). Decide the scope first: a rename is a key change and goes in the negative-logging convention's key-change table. | packet-coder | -- |
| L13 | Draft spec puts the receive queue before the filter | `docs/drafts/spec/mercury-wire-format.md` and its Figure 33 show `recvfrom`, then the queue, then the cipher filter. TG-MER-12's RE ([client-mercury-receive-path.md](../../reverse-engineering/findings/client-mercury-receive-path.md)) found the network thread runs `recvfrom`, then the filter, then the queue. Fixing it changes a figure, so the figure-source and figure-style checks gate it. | documentation-writer with bigworld-engine-advisor | -- |
| L14 | `lab-bridge` clippy on the native target | `cargo clippy -p cimmeria-client-telemetry --features lab-bridge` on the native (x64) target fails with 5 errors in the bridge files. It predates the campaign, and that build never ships (the lab build is i686), so CI never sees it. Fix or gate the bridge for non-i686 targets. | rust-gameserver-dev | -- |

### Tooling

| # | Item | Context | Owner / agent | Issue |
|---|---|---|---|---|
| L15 | An empty lane slot makes every worktree look busy | In `rm-worktree.ps1`, `Test-Building` treats a lane slot directory with no `what` file as a build in progress in every worktree. One slot left empty during the campaign made `--merged` retire 0 of 143 worktrees, and the Dev Drive dropped below the lane's free-space floor. Fix: ignore or reap a slot with no `what` or `pid` older than a few minutes, and check whether `lane-slots.ps1` reaps it too. | coordinator | #1646 |

### Carried from "Not packeted"

These were triaged out at collation ([work-packets.md § Not packeted](work-packets.md#not-packeted)) and are still open. Each needs its own issue.

| # | Item | Disposition |
|---|---|---|
| L16 | The outbox drainer can pick rows `try_dispatch_now` is still sending | Issue for database-persistence |
| L17 | Non-vault inventory move refusals don't snap the item back in the client | Issue (behaviour) for items-systems-advisor |
| L18 | A same-world respawn write zeroes the player's facing | Issue for movement-teleport-advisor |
| L19 | Claude Code reports MCP tools as `mcp_tool` in its telemetry | Ops change: check the installed version's tool-detail setting |
| L20 | Why a no-argument `endCurrentMinigame` carries 4 bytes | Question for bigworld-engine-advisor |
| L21 | Deferred packets TG-MIS-04 (`anchor_unused` heuristic), TG-NPC-14 (threat on a non-preemptable NPC), TG-SOC-16 (org roster on a poisoned lock) | Revisit TG-MIS-04 once `mission.step_anchor` rows (TG-MIS-03) have been read in a real session; the other two only on evidence |
