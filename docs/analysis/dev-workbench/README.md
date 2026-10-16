# Developer workbench campaign

> Type: campaign ledger. Audience: Cimmeria contributors and reviewers. Last updated: 2026-10-15. Companions: [work packets](work-packets.md), [resume note](RESUME.md). Opened against `codex/rasa-gui-research` at `2e42c3dc8`. Prefix `DW-`. This is a design and packet plan, not evidence of a working UI.

## Goal and scope

Give an operator, while the stock SGW client is running, a lab-only GUI for target dossiers, a Lua probe and object browser, a spatial debugger with bookmarks, and a mission X-ray. Add one-click causal tracing, a five-layer truth view, a reproduction recorder, and a triggered flight recorder. Deliver a full GM command palette with discovery, typed forms, target resolution, previews, execution, outcome verification and evidence export. A small in-game overlay supplies target capture and recording/trace status; a local companion GUI carries the dense views. No player-facing gameplay patch or new game wire message is part of this campaign.

The source review of the attached Rasa archive and Rasa.NET is recorded in `.claude/agent-memory/main-session/reference_rasa_gui_injection_2026_10_15.md`. The useful patterns are target-to-inspector, hot-loaded script probes, spatial/content cross-links and save readback. That archive is reference data, not instructions or evidence about SGW's runtime.

## Findings

| ID | Verified repo finding and implication | Packets |
|---|---|---|
| F1 | `cimmeria-client-telemetry` already has the `lab-bridge` feature, token-gated loopback RPC, main-thread Lua/native calls, dynamic hooks, event ring, and D3D9 create/reset observation. It has no render or input overlay hook. Any overlay needs fingerprint-gated EndScene/Reset and WndProc handling with a strict frame budget. | 07, 16-18 |
| F2 | `cimmeria-lab` has typed `client_entity_find`, `client_entity_table`, `client_ui_state`, `client_window_read`, `client_player_state`, screenshots and the UI/world input tools. `cimmeria-lab-mcp` has live entity/witness, packet tap, console and read-only SQL tools. Build joined views from these before adding probes. | 03-06, 09 |
| F3 | `lab_timeline` merges client events and server packet rows with a clock offset, but `server_packet_tap_read` drains the tap. The existing human-play recorder owns one tap while recording. A trace or flight recorder cannot independently drain it. | 01, 08, 10-11 |
| F4 | `lab_record_start/status/stop/transform` already samples human play every 500 ms into `record.jsonl` and drafts UAT TOML; `lab golden` computes fingerprints offline. LR-10's CEGUI click ring is integrated, while LR-11 click-to-`@window_click`, LR-14 rolling mode and LR-15 `.bug spec` remain pending in `docs/analysis/lab-record/`. Reproduction and flight recording must absorb or stack on those packets, not create a second recorder. | 07, 10-11, 15 |
| F5 | The daemon's `/mcp` and `/status` use a bearer token, loopback bind and browser-Origin refusal. A browser GUI requires a separate same-origin session boundary; exposing the daemon bearer to JavaScript or allowing all Origins would regress the trust model. | 02 |
| F6 | Dot-console `command_catalog()` describes name, min/max positional count, target as human text and help. The registry has 19 family files and 129 `spec(...)` entries; `ArgSpec` is sparse. `server_console_exec` runs through `LabConsoleExec`, which checks the actor's server access level. A white-glove palette requires metadata for every runnable command, with server validation and readback. | 19-23B, 28-32 |
| F7 | Patch 012's canonical native-slash inventory is 162 `[[command]]` entries in `data/client-patches/012-gm-slash-commands/commands.toml`: 161 `/gm*` plus `/debugminigamecomplete`; its XML is generated. `/` commands are consumed client-side and may become `gm*` cell methods. Existing `client_console` has no proven built-in string-taking address, so native submission requires proof. Never add a server slash-chat parser or execute slash as dot. | 24-27B, 28-32 |
| F7a | At the planning base, older prose in `docs/commands.md` and `docs/architecture/gm-cell-method-gating.md` said 167 `/gm*`, while the patch table has 161; `data/client-patches/README.md` describes 164 QA XML entries and 162 distinct words. DW-24A found that stock SGW `SlashCommands.xml`/`FinalSlashCommands.xml` are absent from this checkout, so [its stock audit](stock-slash-audit.md) is explicitly unknown. DW-27C owns descriptors for any verified extras once those files can be inspected; DW-32 cannot claim complete coverage yet. | 24A, 27C, 32 |
| F8 | Seeds remain the source of truth. Live authoring commands can emit SQL and update memory/DB, but a palette must show that the emitted seed change needs committing and cannot imply a durable save. | 20B, 30-31 |
| F9 | No Tabula Rasa client or SGW client was run in the source review. Lab-live verification remains unperformed and needs the user's explicit go-ahead before any lease, start/restart or server change. | 35 |
| F10 | DW-03 leaves live server fields unbound. DW-03S-E traced a specific producer gap: Mercury `Transport::send_to` has only bytes/peer, retransmit collection returns `Vec<Bytes>`, and a BaseService peer lookup after await could label delayed A traffic as replacement B. Cell origin teardown can also precede Base's GateTravel handling while the ordinary roster remains unchanged. A queued old A `GateTravel` can borrow B's current session before reservation, and an old A `CreateEntity` can overwrite B's reused Cell entity. Source ownership, Base terminal reject before Cell rejection activates, accepted ordered Cell claim, in-flight selected-player takeover invalidation and exact DB conditional arrival are mandatory. An immutable send ticket must cross the actual await and a Cell travel epoch must gate the join. No live hook event or endpoint is proven. | 03S-E, 03S-P/G/H/D/F/S, 03S-T-C/T-O-M/T-O-C/T-O-S0/T-O-S1/T-O-I/T-O-V/T-O-F/T-O-Q/T-DB0/T-DB2/T-K/T-DB1/T-P1/T-P2/T-A/T-R, 03S-X/I/B |
| F11 | The `recvfrom` IAT slot is process-wide. A 1472-byte request with zero flags is a candidate shape, not Mercury socket identity; the Phase-3 login reply is plaintext before AES. The receive-path notes identify `FUN_0158a200` and `PacketEncrypter::recv` but do not supply the exact original-SGW.exe x86 hook contract for a new seam. DW-03S-C candidate rows remain ineligible until R-E verifies that contract, R-P proves Mercury provenance and R-C proves post-handshake decrypt/filter disposition for the same datagram. | 03S-C, 03S-R-E/R-P/R-C, 03S-B |

## Decisions

| ID | Status | Decision and reason |
|---|---|---|
| D-DW1 | Decided | Reuse the existing lab daemon, bridge, server lab endpoint, UAT recorder, packet tap and golden bundle. Keep all new tools lab-only. The DLL remains feature-gated and session-gated; no server-to-client command relay. |
| D-DW2 | Decided (user, 2026-10-15) | A local companion window serves dense inspectors and the **full GM searchable forms/results**. The daemon creates a short-lived, same-origin workbench session via a CLI opener, uses an HttpOnly SameSite cookie plus CSRF token for mutations, rejects foreign Origin/Host, and never sends the MCP bearer or bridge token to page JavaScript. The existing `/mcp` and `/status` auth behavior stays intact. |
| D-DW3 | Decided (user, 2026-10-15) | A minimal lab-only D3D9 overlay captures the current client target and shows trace/recording status plus an opener for the local companion window; the companion owns forms and diagnostics. Overlay hooks must be fingerprint-gated, main-thread only, inert without a lab session, and restore device/input state. This is authorized by the user's lab-DLL extension request. |
| D-DW4 | Decided | All joins use an explicit `CaptureContext` (instance, client PID/fingerprint, player entity, selected entity, server session, wall/monotonic anchors, source and completeness flags). Missing layers display `unknown` with a reason; they never become inferred equality or success. |
| D-DW5 | Decided | Tap ownership is exclusive. Trace and flight recording consume events through a bounded supervisor broker with named cursors; one server tap is read by one owner and fanned out to subscribers. A UAT/human recorder may refuse competing capture rather than steal its tap. |
| D-DW6 | Decided | Command metadata is generated from the dot-console registry and completed in family slices. A temporary manual form is allowed during rollout; final acceptance requires an explicit descriptor or justified hidden/retired disposition for every runnable registry command. The cell-side GM gate remains authoritative. |
| D-DW7 | Decided | Preview is an impact plan and precondition check, not a claim that arbitrary commands can be dry-run. Every mutation reports the planned effect, risk and readback recipe; an undo button appears only for a proven inverse, otherwise a labelled compensating action or irreversible state. |
| D-DW8 | Decided | Dot and native slash commands share one palette but have separate authoritative inventories and execution paths. All 162 patch-table slash entries, plus any stock GM-gated extras DW-24A verifies, receive typed forms, target/impact/readback metadata and availability; an unsupported handler or unproven native path is disabled per command with an explanation. A submitted slash string is not proof that the server accepted it. |
| D-DW9 | BlockedDecision | Live lab acceptance requires the owner to approve taking the shared lab lease and, if necessary, starting or restarting the client. Code and offline tests can proceed. Ask immediately before DW-35; do not bundle approval into a code packet. |
| D-DW10 | OutOfScope | Distributing the overlay or GUI to ordinary players would be a distinct product decision. No packet depends on it; this campaign does not ship that surface. |
| D-DW11 | Decided | A trusted live join requires two distinct encrypted datagrams observed by the process-bound DLL and attributed to one current server connection generation, plus fresh, unique generation-bearing roster confirmation of that generation, player entity and world before and after server reads. The server mints a CSPRNG boot ID on each service start and carries an immutable origin generation with each queued/awaited send, including a send that completes after same-peer relog. The supervisor stamps PID, process creation ticks, bridge epoch and capture time when each client event enters the store, never when it is later read. Client and server wall clocks may differ; check source-local freshness and pair-to-pair offset consistency. Server and client UDP endpoints are diagnostics, not equality keys: NAT can rewrite the source. FNV wire fingerprints are correlation evidence, not credentials or authority for a GM action. Ambiguity, missed observations, incomplete send-path coverage, stale generations, PID reuse, relog and Cell-authoritative travel epoch/state changes clear the binding. A Base roster that looks unchanged during Cell teardown is insufficient; X/B require matching stable Cell/Base epochs. |
| D-DW12 | BlockedDecision | Exact live Mercury peer and fingerprint behavior, including the issue #1703 missing-hook case, requires a separately approved lab lease/client run. Offline proof and code packets may proceed; a binding cannot be called live verified until the owner approves that probe and its evidence passes. |
| D-DW13 | Decided (static evidence, 2026-10-16) | The send proof is opt-in at Mercury's transport trait and immutable at the packet's originating session, including retransmit collection. The Base observer records only a full successful send whose destination equals the immutable origin peer, exempts only the two verifiable pre-Phase-3 handshake sends and treats mismatched peers or other untagged/uncensused paths as incomplete. Cell `SpaceManager::destroy_entity` supplies an authoritative per-player travel epoch at origin teardown; the exact epoch is returned with destination creation and carried through session pending stages. Because `onClientReady` is argument-free, a second trip in one generation makes readiness proof ambiguous and blocks Cell settlement until a future trip-specific client signal is verified; X/B require a matching stable Cell/Base epoch and current occupant incarnation, including for same-world travel. No address lookup after send, latest-epoch adoption or unchanged-roster shortcut is permitted. |
| D-DW14 | Decided (protocol/dispatch review, 2026-10-16) | `SGWPlayer.onClientReady` has no trip argument, so a pending token or ingress time cannot prove which trip emitted a late ready. First ready in a connection generation may settle only while unambiguous. Starting a second trip in that generation latches `ready_ambiguous` until a new generation; gameplay can proceed, but no Cell proof settlement or joined dossier is eligible. A later-trip rebind needs a separately evidenced trip-specific client signal and contract revision. |
| D-DW15 | Decided (static source, 2026-10-16) | `Orchestrator::new` constructs Base, Cell and one in-process Tokio channel, and Cell consumes messages serially, but cloned senders and a retained Cell across Base starts have no source-proven owner order. Every player create needs an opaque incarnation and monotonic process-lifetime issue order reserved together before await, then finalized with the entity ID; Cell must acknowledge B's accepted claim and reject lower-order A before insertion. Any missing/shared-order proof, B ack, current occupant incarnation or unsupported process epoch is `owner_order_unknown` and blocks joined fields. T-O-S0/S1/I/V/F/Q and T-DB0/DB2/DB1 own the prerequisite proof and tests; until merged and passed, P1/P2/S/X/B remain `BlockedDependency`. |
| D-DW16 | Decided (static source, 2026-10-16) | `CellToBaseMsg::GateTravel` omits source incarnation/epoch, while Base selects a current peer before minting and `persist_arrival` updates by account/player only after an awaited stargate query. A Cell-origin token must be compared before reservation; rejected/missing Cell create ack is terminal. T-K terminal Base handling must merge before O-F emits a live Cell rejection. DB0 records a selected-player claim before the first play-character await, while `active_player_id` may be `None`; a DB row owner claim and phase-3 takeover invalidation of active or in-flight selected claims (revoking old A before minting the invalidation order) with monotonic process order, plus exact-owner conditional arrival UPDATE must pass a takeover race before persistence is safe. Cross-process epoch rollover is unproven and remains `owner_db_unknown`/`BlockedDependency`. |
| D-DW17 | BlockedDependency (static source, 2026-10-16) | `lifecycle.rs::persist_last_position` queues `CellToBaseMsg::PersistPosition` without an owner token, and `cell_dispatch/position.rs::persist_position` updates `sgw_player.world_location/pos_*` by player ID alone. A queued old A logout write could overwrite B after phase-3 takeover despite the guarded GateTravel arrival. A separately scoped owner-carrying conditional write plus old-A-after-B live-DB regression is required before S/X/B may expose complete joined fields. No packet here claims that guard; other tracks may proceed with joined fields unknown. |
| D-DW18 | Decided | DW-05 ships spatial observations and local bookmark CRUD. Bookmark click and teleport return `bookmark_action_unavailable` with `session_identity_unbound` before client input. Same-world relog cannot be proven from world/PID/player equality, so `current` stays false until a trusted login generation exists. DW-05G establishes an execution-time input guard after DW-03S-B; DW-05A uses it for bookmark actions. The GUI keeps those controls disabled until DW-05A passes. |
| D-DW19 | BlockedDependency (static receive-path review, 2026-10-16) | A process-wide `recvfrom` IAT slot plus 1472-byte/zero-flags shape does not establish Mercury socket identity or encryption phase. DW-03S-C emits diagnostic candidate rows only, with `eligible:false` and `mercury_socket_unverified`; the Phase-3 reply is plaintext before AES. DW-03S-R-E must first verify exact x86 ABI, prologue, call bytes, object lifetime and original-build fingerprint against the original SGW.exe, which is absent from this checkout. R-P/R-C must add every new target to the global boot `CODE_SITES` gate and hook inventory; a byte mismatch blocks installation before any new detour runs. R-P may prove Mercury receive provenance; R-C must pair that exact datagram with successful post-handshake `PacketEncrypter::recv` and downstream filter disposition before any row becomes eligible. DW-03S-B requires all three reviewed and merged. No peer, fingerprint, packet shape or successful bridge read bypasses the gate. |

## Packet ledger

`Ready` means design-ready, not dispatched; `Writing` means a packet owns a branch and worktree; `Review` means its implementation is committed and under adversarial review. Each packet estimates **25-45k working tokens total** including implementation, tests, docs and review overhead; none is intended to exceed 50k. Evidence packets (`-E`) produce a sourced per-command contract before their metadata coder packet may run; the coder cannot invent behavior. Waves are dependency phases; within a wave, review and integration proceed in batches of about six packets. Packet specs carry contract and rules independently for fresh workers. Wave 8 is a live gate.

| Wave | ID | Result | Depends | Budget | Status |
|---|---|---|---|---|---|
| 1 | DW-01 | Capture contract and broker | — | 40k | Integrated |
| 1 | DW-02A | GUI session security | — | 40k | Integrated |
| 1 | DW-02B | GUI opener/static shell/API | 02A | 40k | Integrated |
| 1 | DW-03 | Target dossier model | 01, 02B | 35k | Integrated |
| 1B-a | DW-03S-E | Static binding evidence and fixture contract | 03 | 30k | Integrated (#1750) |
| 1B-a | DW-03S-C | Process-bound DLL Mercury candidate observer | 03S-E | 40k | Integrated (#1763) |
| 1B-a-receive | DW-03S-R-E | Original-client receive/cipher hook evidence | 03S-C, original SGW.exe | 30k | BlockedDependency |
| 1B-a-receive | DW-03S-R-P | Mercury socket and exact datagram provenance | 03S-R-E | 45k | BlockedDependency |
| 1B-a-receive | DW-03S-R-C | Post-handshake decrypt/filter classifier | 03S-R-P | 45k | BlockedDependency |
| 1B-a | DW-03S-P | Opt-in Mercury send-origin seam | 03S-E | 35k | Integrated (#1761) |
| 1B-a | DW-03S-G | Base boot/generation and bounded proof observer | 03S-P | 45k | Review |
| 1B-a | DW-03S-H | Base-session reliable/unreliable/tick/retransmit origin | 03S-G | 45k | BlockedDependency |
| 1B-a | DW-03S-D | World-entry and direct Base send origin | 03S-H | 45k | BlockedDependency |
| 1B-b | DW-03S-F | Cell fan-out and complete send-path census | 03S-D | 40k | BlockedDependency |
| 1B-b | DW-03S-T-C | Cell-authoritative travel epoch | 03S-E | 45k | Ready |
| 1B-b | DW-03S-T-O-M | Typed internal create owner and rejection reply | 03S-E | 45k | Ready |
| 1B-b | DW-03S-T-O-C | Cell owner fence and occupant sidecar | 03S-T-O-M | 40k | BlockedDependency |
| 1B-b | DW-03S-T-O-S0 | Typed GateTravel source and fixtures | 03S-T-O-M | 35k | BlockedDependency |
| 1B-b | DW-03S-T-O-I | Ordered process-lifetime create issuer | 03S-T-O-M, 03S-G | 40k | BlockedDependency |
| 1B-c | DW-03S-T-O-S1 | Stamp actual Cell travel source | 03S-T-C, 03S-T-O-C, 03S-T-O-S0 | 45k | BlockedDependency |
| 1B-c | DW-03S-T-O-V | Validate source before destination reservation | 03S-T-O-S1, 03S-T-O-I | 45k | BlockedDependency |
| 1B-c | DW-03S-T-K | Terminal rejected Cell create | 03S-T-O-M, 03S-T-O-V | 35k | BlockedDependency |
| 1B-c | DW-03S-T-O-F | Reject stale Cell creates before insertion | 03S-T-O-V, 03S-T-K, 03S-T-O-C, 03S-T-C | 45k | BlockedDependency |
| 1B-c | DW-03S-T-O-Q | Query exact Cell occupant incarnation | 03S-T-O-F, 03S-T-C | 40k | BlockedDependency |
| 1B-c | DW-03S-T-DB0 | DB player travel owner claim | 03S-T-O-I | 45k | BlockedDependency |
| 1B-d | DW-03S-T-DB2 | Invalidate active and in-flight DB owner at takeover | 03S-T-DB0, 03S-G | 45k | BlockedDependency |
| 1B-d | DW-03S-T-DB1 | Conditional arrival write for exact trip | 03S-T-DB0, 03S-T-DB2, 03S-T-K, 03S-T-O-V | 45k | BlockedDependency |
| 1B-d | DW-03S-T-P1 | Atomic Cell destination-creation receipt | 03S-T-C, 03S-T-O-Q, 03S-T-DB1 | 45k | BlockedDependency |
| 1B-d | DW-03S-T-P2 | Carry exact trip token through pending stages | 03S-T-P1, 03S-T-DB1, 03S-G | 45k | BlockedDependency |
| 1B-d | DW-03S-T-A | Argument-free ready ambiguity gate | 03S-T-P2 | 45k | BlockedDependency |
| 1B-d | DW-03S-T-R | Destination readiness and guarded epoch settlement | 03S-T-A, 03S-G | 45k | BlockedDependency |
| 1B-e | DW-03S-S | Server proof snapshot/eligibility gate | 03S-F, 03S-T-R, queued PersistPosition guard (unscoped) | 35k | BlockedDependency |
| 1B-e | DW-03S-X | Lab-only generation/transition roster | 03S-S, 03S-T-R | 35k | BlockedDependency |
| 1B-e | DW-03S-I | Ingestion-time client event identity | 03S-C | 30k | BlockedDependency |
| 1B-e | DW-03S-B | Supervisor binder and dossier integration | 03S-R-E/R-P/R-C, 03S-X, 03S-I, 03 | 45k | BlockedDependency |
| 1 | DW-04 | Lua probe/object model | 01, 02B | 40k | Integrated (#1749) |
| 1 | DW-05 | Spatial observations and bookmark CRUD | 01, 02B | 40k | Integrated (#1758) |
| 2 | DW-05G | Execution-time client input guard | 05, 03S-B | 45k | BlockedDependency |
| 2 | DW-05A | Guarded bookmark actions | 05G | 35k | BlockedDependency |
| 2 | DW-06 | Mission X-ray definition/UI model; server slot fail closed | 01, 02B | 40k | Integrated (#1762) |
| 2 | DW-06A | Live mission state from exact joined server session | 03S-B, 06 | 40k | BlockedDependency |
| 2 | DW-07 | Client click/target correlation | 01 | 35k | Integrated |
| 2 | DW-08 | Trace capture anchor | 01, 02B, 03S-B | 40k | BlockedDependency |
| 2 | DW-09 | Five-layer truth assembler | 03S-B, 06A, 08 | 40k | BlockedDependency |
| 2 | DW-10 | Reproduction recorder enrichment | 07-08, 03S-B, LR-11 | 45k | BlockedDependency |
| 2 | DW-11 | Triggered flight recorder | 08, 03S-B, LR-14/15 | 45k | BlockedDependency |
| 3 | DW-12A | GUI dossier view | 02B-03 | 30k | Integrated (#1760) |
| 3 | DW-12B | GUI Lua probe/object view | 02B, 04 | 30k | Integrated (#1764) |
| 3 | DW-12C | GUI spatial/bookmark view and actions | 02B, 05, 05A | 30k | BlockedDependency |
| 3 | DW-12D | GUI mission X-ray view | 02B, 06 | 30k | Ready |
| 3 | DW-13 | One-click causal trace | 07-08 | 40k | Ready |
| 3 | DW-14 | Trace and truth GUI | 09, 12A, 13 | 35k | Ready |
| 3 | DW-15 | Both recorder GUI flows | 10-11, 12A | 40k | BlockedDependency |
| 4 | DW-16 | Overlay D3D9 render/device | 01 | 45k | Ready |
| 4 | DW-17 | Overlay input/fingerprint | 16 | 45k | Ready |
| 4 | DW-18 | Overlay target/status controls | 13, 15, 17 | 40k | BlockedDependency |
| 4 | DW-19 | GM descriptor/authority contract | 01 | 45k | Integrated |
| 4 | DW-20A-E | Dot travel/spawn evidence, 16 specs | 19 | 35k | Integrated |
| 4 | DW-20A | Dot travel/spawn metadata | 20A-E | 40k | Integrated (#1751) |
| 4 | DW-20B-E | Dot patrol/authoring evidence, 28 specs | 19 | 45k | Integrated (#1765) |
| 4 | DW-20B | Dot patrol/authoring metadata | 20B-E | 45k | Ready |
| 5 | DW-21-E | Dot progression/ability/stat/ammo/pet evidence, 27 specs | 19 | 45k | Ready |
| 5 | DW-21 | Dot progression/ability/stat/ammo/pet metadata | 21-E | 45k | Ready |
| 5 | DW-22-E | Dot social/org/squad/bank/black-market evidence, 25 specs | 19 | 45k | Ready |
| 5 | DW-22 | Dot social/org/squad/bank/black-market metadata | 22-E | 45k | Ready |
| 5 | DW-23A-E | Dot query/maintenance/meta evidence, 19 specs | 19 | 35k | Ready |
| 5 | DW-23A | Dot query/maintenance/meta metadata | 23A-E | 40k | Ready |
| 5 | DW-23B-E | Dot net-debug evidence, 14 specs | 19 | 35k | Ready |
| 5 | DW-23B | Dot net-debug metadata | 23B-E | 35k | Ready |
| 5 | DW-24A | Slash inventory/parser/stock audit | 19 | 40k | Integrated (#1752) |
| 5 | DW-24B | Native submission/quoting/receipt proof | 24A | 45k | Ready |
| 6 | DW-25A-E | Slash evidence 1-27 | 24A | 40k | Ready |
| 6 | DW-25A | Slash descriptors 1-27 | 25A-E | 40k | Ready |
| 6 | DW-25B-E | Slash evidence 28-54 | 24A | 40k | Ready |
| 6 | DW-25B | Slash descriptors 28-54 | 25B-E | 40k | Ready |
| 6 | DW-26A-E | Slash evidence 55-81 | 24A | 40k | Ready |
| 6 | DW-26A | Slash descriptors 55-81 | 26A-E | 40k | Ready |
| 6 | DW-26B-E | Slash evidence 82-108 | 24A | 40k | Ready |
| 6 | DW-26B | Slash descriptors 82-108 | 26B-E | 40k | Ready |
| 6 | DW-27A-E | Slash evidence 109-135 | 24A | 40k | Ready |
| 6 | DW-27A | Slash descriptors 109-135 | 27A-E | 40k | Ready |
| 6 | DW-27B-E | Slash evidence 136-162 | 24A | 40k | Ready |
| 6 | DW-27B | Slash descriptors 136-162 and merge | 25A-27A, 27B-E | 45k | Ready |
| 6 | DW-27C | Verified stock GM slash addendum | 24A, 24B, 27B | 30k | BlockedDependency |
| 7 | DW-28 | Palette search/preferences | 02B, 19-27C | 35k | Ready |
| 7 | DW-29 | Typed forms/target picker | 03S-B, 19-28 | 45k | BlockedDependency |
| 7 | DW-30 | Impact preview/execute/audit | 19-29, 24B, 03S-B | 45k | BlockedDependency |
| 7 | DW-31 | Readback/compensation/evidence | 09, 30 | 40k | Ready |
| 7 | DW-32 | Dot and slash catalog coverage | 20A-31, 24B, 27C | 35k | Ready |
| 7 | DW-33 | Frontend logic/offline UAT | 12A-15, 28-32 | 35k | Ready |
| 8 | DW-34 | Documentation/offline close-out | 01-33 | 30k | BlockedDependency |
| 8 | DW-35 | Live lab acceptance | 34, D-DW9 | 30k | BlockedDecision |

## Campaign acceptance

Every view identifies its data source and freshness; an absent source is shown as unknown. Joined server fields require DW-03S-B's unique, current process-to-server generation proof; offline fixtures cannot establish live hook behavior, which stays under D-DW12/DW-35. The one-click trace identifies the first missing observed stage without mistaking a missing observation for proof that code did not run. Both recorders retain bounded evidence. The local companion palette covers every runnable dot and native-slash command with keyboard search, contextual help/examples, name/id resource lookup, typed controls, target/availability, impact preview, channel-specific execution, readback and evidence; each unsupported entry has a justified disabled status. Slash completeness includes the patch table and a stock-client command inventory audit. A raw form is not final coverage. A reviewer proves each regression guard fails when its fix is removed. Frontend packets include JS REPL-style state and persistence UAT plus visual review; DW-35 is real-client UAT after approval. Campaign status docs change only in DW-34.
