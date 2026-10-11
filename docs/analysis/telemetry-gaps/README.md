# Telemetry Gaps Campaign

> Type: how-to (campaign ledger). Audience: the coordinator session and packet workers.
> Updated: 2026-10-10 (reviews collated into 160 packets). Companions: [work-packets.md](work-packets.md) (rules, contract, W0-W2), [work-packets-w3-w4.md](work-packets-w3-w4.md) (W3-W4), [reviews/](reviews/), [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [negative-logging-convention.md](../../architecture/negative-logging-convention.md), [observability.md](../../architecture/observability.md), [client-telemetry.md](../../architecture/client-telemetry.md).

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
| T8 | A player session with server logs and zero client telemetry (2026-10-09) | At world entry, log whether the session has a client telemetry stream | Server | TG-NET-18 (needs D-TG14), TG-NET-13 |
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
| D-TG14 | TG-NET-18: which join key ties a game session to its client telemetry stream (account on the dev-session mint, or something else)? | Decided 2026-10-10: account name in the launcher token | The launcher adds the account name to the telemetry session token it mints, and ingest joins on it. Needs a launcher release; the account name enters the telemetry claims |
| D-TG15 | TG-DB-02: how long may shutdown wait to drain position and ammo saves inside the container stop grace, and in what stop order? | Decided 2026-10-10: up to 10 s | Save every in-world player in parallel. After 10 s, shut down anyway and WARN once per player not saved, with player_id and name |
| D-TG16 | Add a periodic counter of OTLP records the exporter dropped, accepting an exporter-side change? | Decided 2026-10-10: yes | A periodic dropped-records counter row, written locally and to SigNoz once the exporter recovers |
| D-TG17 | TG-MOV-09 (after the RE): keep sending method 116 to `SGWGmPlayer` clients, skip it, or route the streaming hint another way? | Decided 2026-10-10: decide after TG-MOV-09 | Run the Ghidra packet first, then choose |
| D-TG18 | When the player load degrades (TG-DB-07), refuse world entry or suppress mission persistence for that session? | Decided 2026-10-10: enter with missions locked | Retry the saved-missions read a few times. If it still fails, enter the world with those missions marked unknown, refuse accepting or advancing them with visible feedback and a WARN, and keep retrying in the background. Unlock once the read succeeds. Nothing overwrites completed progress |

## Packet status

States: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done. BlockedDependency includes waiting on a file-collision predecessor ([table](work-packets.md#file-collision-table)). Specs: W0-W2 in [work-packets.md](work-packets.md), W3-W4 in [work-packets-w3-w4.md](work-packets-w3-w4.md).

| ID | Title | Wave | Sev | State |
|---|---|---|---|---|
| TG-PIPE-01 | Bundle replay allowlists log files | W0 | high | Writing (branch `fix/bundle-replay-allowlist`) |
| TG-SOC-01 | Strip the webhook URL from Discord send errors | W0 | high | Review (PR #1349) |
| TG-NET-08 | `admin.request` row for every admin API request | W0 | high | Ready |
| TG-DB-01 | Shutdown logs each in-world player's unsaved state | W0 | high | Ready |
| TG-DB-02 | Flush positions and ammo on shutdown | W0 | high | Ready (D-TG15 decided) |
| TG-DB-03 | Cell position-save skips and queues are logged | W0 | high | Ready |
| TG-DB-04 | `PersistPosition` rows: identity, Pattern B, reason | W0 | med | Ready |
| TG-DB-07 | Player-load fallbacks say they degraded | W0 | med | Ready |
| TG-DB-12 | Lock unreadable missions instead of overwriting them (D-TG18) | W0 | high | BlockedDependency (TG-DB-07) |
| TG-ITM-06 | Stale bandolier ammo writeback to WARN with counts | W0 | med | Ready |
| TG-ITM-11 | Bandolier flush summary and dropped-slot trace | W0 | med | Ready |
| TG-ITM-09 | Loot silent drops and lost cash | W0 | med | Ready |
| TG-ITM-03 | Outbox replay row and identity on outbox WARNs | W0 | med | Ready |
| TG-SOC-02 | `character.delete_cascade` row | W0 | high | Ready |
| TG-SOC-03 | Character delete: refund or destroy | W0 | high | Dropped (D-TG9: accept destruction) |
| TG-MIS-07 | Log dropped deferred content actions | W0 | med | Ready |
| TG-MER-08 | Delete the per-packet encrypt/decrypt TRACE rows | W1 | high | Ready |
| TG-MER-09 | `mercury.packet` to the firehose plus a counter | W1 | high | Ready (D-TG7 decided) |
| TG-NPC-01 | Sample unwitnessed Patrol/Wander tick rows | W1 | high | Ready |
| TG-NPC-02 | Gate `movement.npc` step rows on witnesses | W1 | high | Ready |
| TG-NPC-03 | Drop `no_candidates` rows with zero witnesses | W1 | high | Ready |
| TG-NPC-09 | Sample unwitnessed leg and path-request rows | W1 | med | Ready |
| TG-MER-10 | Throttle WSAECONNRESET | W1 | low | Ready |
| TG-SOC-11 | Discord config watcher self-trigger | W1 | med | Ready |
| TG-AOI-10 | Demote single-packet AoI bundle rows | W1 | med | Ready |
| TG-MOV-11 | Drop the per-packet `EntityMove` TRACE | W1 | low | Ready |
| TG-MOV-03 | Speed warn: separate bunched packets | W1 | med | Ready |
| TG-MOV-10 | GM navmesh bypass: INFO, throttled | W1 | low | Ready (after TG-MOV-03) |
| TG-MOV-14 | Dial-hub GM grant WARN to INFO | W1 | low | Ready |
| TG-CMB-04 | Quiet the boot `effect_script_unregistered` WARN | W1 | med | Ready |
| TG-CMB-09 | Demote NPC-only launch and kill INFO rows | W1 | low | Ready |
| TG-SOC-13 | Mail sweep: no row when nothing was scanned | W1 | low | Ready |
| TG-MIS-12 | Demote no-listener cover rows; collapse `add_dialog_set` | W1 | low | Ready |
| TG-MER-14 | Cooked-sync logout race to DEBUG | W1 | low | Ready |
| TG-DB-11 | Demote the `sqlx::query` statement rows | W1 | low | BlockedDependency (TG-DB-06; D-TG8 decided) |
| TG-NET-01 | `session.end` on logOff to character select | W2 | high | Ready |
| TG-NET-02 | End-cause hints on `session.end` | W2 | high | BlockedDependency (TG-NET-01) |
| TG-MER-07 | Channel health on `session.end` | W2 | med | BlockedDependency (TG-NET-02) |
| TG-NET-03 | Duplicate-login liveness | W2 | high | Ready |
| TG-NET-10 | `session.end server_shutdown` at stop | W2 | med | BlockedDependency (TG-NET-01) |
| TG-NET-14 | Inactivity-timeout row context | W2 | med | BlockedDependency (TG-MER-07) |
| TG-NET-13 | Dev-session refusals no longer dropped | W2 | med | Ready |
| TG-NET-15 | `session.start` says login or travel | W2 | low | Ready (domain agent) |
| TG-NET-16 | Test-account flag on `account` | W2 | med | Ready (domain agent) |
| TG-NET-17 | `cimmeria.session_kind` on server session rows | W2 | high | Ready (D-TG13 decided) |
| TG-NET-18 | World entry says whether client telemetry exists | W2 | med | Ready (D-TG14 decided; launcher release) |
| TG-PIPE-17 | labd ships its log to SigNoz | W2 | high | Ready (domain agent) |
| TG-PIPE-05 | labd launch, stop and pid-overwrite rows | W2 | med | Ready |
| TG-PIPE-06 | labd per-tool-call row | W2 | med | Ready |
| TG-PIPE-11 | lab-mcp audit: error and duration | W2 | low | Ready |
| TG-PIPE-07 | `lab_disconnect` flow | W2 | med | Ready |
| TG-PIPE-08 | S1: runner ends every run with logout and disconnect | W2 | med | BlockedDependency (TG-PIPE-07, TG-NET-01) |
| TG-AOI-01 | Cap drop names the entity and kind | W3 | high | Ready |
| TG-AOI-02 | Appearance rebroadcast success row | W3 | med | Ready |
| TG-AOI-04 | Flush-source `send_kind` | W3 | med | Ready |
| TG-AOI-09 | Per-message manifest on flush bundles | W3 | high | BlockedDependency (TG-AOI-04, TG-AOI-10) |
| TG-AOI-05 | Classify `not_in_witness_aoi` | W3 | med | Ready |
| TG-AOI-06 | Close the cinematic-hold lifecycle | W3 | low | BlockedDependency (TG-AOI-01) |
| TG-AOI-07 | AoI counters and witness-set gauge | W3 | med | Ready |
| TG-AOI-08 | Leave and invisible send-failure row | W3 | low | Ready |
| TG-MER-01 | Numeric identity on `mercury.reliable_send` | W3 | high | Ready |
| TG-MER-02 | Cause fields on `mercury.retransmit` | W3 | high | BlockedDependency (TG-MER-07) |
| TG-MER-03 | Channel log identity | W3 | high | BlockedDependency (TG-MER-02) |
| TG-MER-04 | Identity on `tx_hole` and `rx_order` | W3 | med | BlockedDependency (TG-MER-03) |
| TG-MER-05 | Base→cell send seam WARNs | W3 | high | Ready |
| TG-MER-06 | Inbound bundle-walk drops out of TRACE | W3 | med | BlockedDependency (TG-MER-05) |
| TG-MER-11 | Identity on the catch-all handler rows | W3 | low | BlockedDependency (TG-MER-10) |
| TG-MER-13 | Client version and cooked-reply fields | W3 | low | BlockedDependency (TG-MER-06) |
| TG-CMB-01 | Target-scan blind spot; pin `player.respawn` | W3 | high | Ready |
| TG-CMB-02 | Stale combat-state detector | W3 | high | Ready |
| TG-CMB-03 | `effect_inert` WARN | W3 | med | Ready |
| TG-CMB-05 | `bandolier` rows: Rule 5 and numeric ids | W3 | med | BlockedDependency (TG-ITM-11, TG-NET-17) |
| TG-CMB-06 | Death and revive rows: Rule 5/6 keys | W3 | med | BlockedDependency (TG-CMB-09) |
| TG-CMB-07 | No-witness WARNs say why a player counted | W3 | med | Ready |
| TG-CMB-08 | Proximity aggro: combat flag not announced | W3 | low | BlockedDependency (TG-NPC-11) |
| TG-CMB-10 | Log a failed `onEndAidWait` send | W3 | low | Ready |
| TG-ITM-01 | Appearance refresh outcome rows | W3 | high | Ready |
| TG-ITM-14 | Abort an appearance refresh on a failed load | W3 | high | Ready (D-TG11 decided) |
| TG-ITM-02 | Loot roll outcome row | W3 | high | Ready |
| TG-ITM-04 | Inline move refusals get event and reason | W3 | med | Ready |
| TG-ITM-05 | After-commit: no silent drops | W3 | med | Ready |
| TG-ITM-07 | Free vendor repair and recharge via `VendorLog` | W3 | med | Superseded (vendor fix branch) |
| TG-ITM-08 | Numeric ids and identity on `item_sequence` | W3 | med | Ready |
| TG-ITM-10 | Remove rows: origin and stack counts | W3 | med | Ready |
| TG-ITM-12 | Grant refusal levels | W3 | low | Ready |
| TG-ITM-13 | Double-click `useItem` level; entry identity | W3 | low | Ready |
| TG-MG-01 | Base minigame seam: lost result, disabled server | W3 | high | Ready |
| TG-MG-02 | MinigamePlayer stubs log every call per the .def | W3 | high | Ready (coordinate with #1303) |
| TG-MG-03 | Session end reason, outcome and duration | W3 | med | Ready |
| TG-MG-04 | Duplicate-session row says why | W3 | med | BlockedDependency (TG-MG-01) |
| TG-MG-05 | Livewire rejection rows name the player | W3 | med | Ready |
| TG-MG-06 | `minigame.connection` span | W3 | med | BlockedDependency (TG-MG-03) |
| TG-MG-07 | Victory rows carry `validation` | W3 | low | BlockedDependency (TG-MG-06) |
| TG-MG-08 | Login row: peer, player, ticket age | W3 | low | Ready |
| TG-MG-09 | Cell result for an unknown entity | W3 | low | Ready |
| TG-MG-10 | `pending_sessions` on the D-TG1 refusal | W3 | low | BlockedDependency (PR #1348) |
| TG-MIS-01 | Identity and `event=` on mission lifecycle rows | W3 | high | Ready |
| TG-MIS-02 | Identity and context on friction rows | W3 | high | BlockedDependency (TG-NET-17) |
| TG-MIS-03 | `mission.step_anchor` row at step activation | W3 | high | BlockedDependency (TG-MIS-09) (domain agent) |
| TG-MIS-05 | `missions_restored` login snapshot | W3 | med | BlockedDependency (TG-CMB-05) |
| TG-MIS-06 | Executor mission rows carry `player_id` | W3 | med | Ready |
| TG-MIS-08 | WARN on dropped mission client frames | W3 | med | BlockedDependency (TG-MIS-01) |
| TG-MIS-09 | Numeric ids on `content.resolve` | W3 | med | Ready |
| TG-MIS-10 | Malformed-args and missing-player rows | W3 | low | Ready |
| TG-MIS-11 | Base mission persist rows | W3 | low | BlockedDependency (TG-DB-07) |
| TG-MIS-13 | OTEL row for content-engine conditions | W3 | low | Ready |
| TG-MOV-01 | Truthful, joinable forced-position row | W3 | high | Ready |
| TG-MOV-02 | `movement.teleport` row for authorized moves | W3 | high | BlockedDependency (TG-MOV-10) |
| TG-MOV-04 | Ring passenger release row | W3 | med | Ready |
| TG-MOV-05 | Ring FSM transition rows | W3 | med | Ready |
| TG-MOV-06 | Split `gm/travel.rs` | W3 | low | Ready |
| TG-MOV-07 | Rule 5 on GM travel rows | W3 | med | BlockedDependency (TG-MOV-06) |
| TG-MOV-08 | Content teleport identity | W3 | med | Ready |
| TG-MOV-09 | Method 116 to `SGWGmPlayer`: RE the drop | W3 | med | Ready (RE, domain agent); then D-TG17 |
| TG-MOV-12 | `navmesh_missing` counter by world | W3 | low | Ready |
| TG-NPC-04 | A death that will not respawn says so | W3 | med | BlockedDependency (TG-CMB-09) |
| TG-NPC-05 | NPC attack-not-fired: event, throttle | W3 | med | Ready |
| TG-NPC-06 | Instance population summary | W3 | med | Ready |
| TG-NPC-07 | `player_id` holding an entity id | W3 | med | BlockedDependency (TG-NPC-03) |
| TG-NPC-08 | Target player identity on leash and stuck | W3 | med | Ready |
| TG-NPC-10 | Cover decisions and `path_fail` attributable | W3 | low | Ready |
| TG-NPC-11 | Zone on tick rows; aggro outcome | W3 | low | BlockedDependency (TG-NPC-01) |
| TG-NPC-12 | Respawn tick negative gaps | W3 | low | Ready |
| TG-NPC-13 | DoT kill-credit miss WARN | W3 | low | Ready |
| TG-NPC-15 | `spawner.npc_behaviour` field hygiene | W3 | low | BlockedDependency (TG-NPC-06) |
| TG-SOC-04 | Presence skips Ignore-list watchers | W3 | high | Ready (D-TG10 decided) |
| TG-SOC-05 | Presence fan-out counts | W3 | med | Ready |
| TG-SOC-06 | Cell trade events and cancel reason | W3 | med | Ready |
| TG-SOC-07 | Spatial chat send failures and identity | W3 | med | Ready |
| TG-SOC-08 | Base chat malformed and relay refusals | W3 | med | Ready |
| TG-SOC-09 | Cell contact-list short-args rows | W3 | med | Ready |
| TG-SOC-10 | Contact header ops events and reasons | W3 | med | Ready |
| TG-SOC-12 | Discord drop accounting | W3 | med | Ready |
| TG-SOC-14 | User-channel teardown leaves | W3 | low | BlockedDependency (TG-NET-17) |
| TG-SOC-15 | Death presence without a character name | W3 | low | BlockedDependency (TG-CMB-06) |
| TG-NET-04 | Phase 1/2 refusals log a reason | W3 | med | Ready |
| TG-NET-05 | Reaper logs each unconsumed ticket and SID | W3 | med | Ready |
| TG-NET-06 | Phase 3 refusal context; account-arm identity | W3 | med | BlockedDependency (TG-MER-06) |
| TG-NET-09 | GM gate and lab console identity and reason | W3 | med | Ready |
| TG-NET-12 | Rate-limited unknown-peer datagram row | W3 | low | BlockedDependency (TG-MER-11) |
| TG-DB-05 | Slow cell-to-base dispatch WARN | W3 | med | BlockedDependency (TG-NET-10) |
| TG-DB-06 | Pool configuration, saturation and slow statements | W3 | med | Ready |
| TG-DB-08 | Character-create refusals carry event and reason | W3 | med | Ready |
| TG-DB-09 | Startup cache-load failures and summary | W3 | low | BlockedDependency (TG-CMB-04, TG-DB-05) |
| TG-PIPE-10 | Busy refusal names its pool | W3 | med | BlockedDependency (TG-NET-17) |
| TG-PIPE-15 | Bundle lines carry a parsed level | W3 | low | BlockedDependency (TG-PIPE-01) |
| TG-PIPE-16 | `entity_labels_unavailable` carries `session_id` | W3 | low | Ready |
| TG-PIPE-02 | DLL sends its pid with every chunk | W4 | high | Ready |
| TG-PIPE-04 | Ingest stamps `client_pid` (server) | W4 | high | Ready |
| TG-PIPE-03 | DLL final flush at process exit | W4 | high | BlockedDependency (TG-PIPE-02) (domain agent) |
| TG-CLI-04 | `client.session.end` from the DLL | W4 | high | BlockedDependency (TG-PIPE-03) |
| TG-PIPE-12 | Launcher `session_meta kind=ended` | W4 | low | Ready |
| TG-PIPE-13 | DLL upload-failure accounting | W4 | low | BlockedDependency (TG-PIPE-03) |
| TG-PIPE-14 | DLL boot failures as events | W4 | low | BlockedDependency (TG-PIPE-13) |
| TG-MER-12 | Client join keys on Mercury faults | W4 | med | Ready (domain agent) |
| TG-CLI-01 | `client.viewport.unknown_entity` per entity | W4 | high | Ready |
| TG-CLI-02 | Tag BigWorld errors after an unpack fault | W4 | med | BlockedDependency (TG-MER-12, TG-CLI-01) |
| TG-CLI-03 | Cooked pak cache write result and tier | W4 | med | Ready (domain agent) |
| TG-CLI-05 | Main-thread phase on hitch rows | W4 | med | Ready |
| TG-CLI-06 | Audio stop names its cue | W4 | med | Ready |
| TG-CLI-07 | IAT slot mismatch names its owner | W4 | med | Ready |
