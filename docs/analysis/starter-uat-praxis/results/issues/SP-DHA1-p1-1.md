server/client-patch: /gmmissionclear never delivers its DesignID argument from the real client (starter UAT SP-DHA1-p1)

## Problem
Typing `/gmmissionclear <missionId>` in the real SGW client reaches the cell handler `gmMissionClear` (method 110) with an EMPTY WSTRING, so the server rejects it with "GM mission cmd: DesignID is not a positive numeric mission id" (`mission_arg=""`) and the client shows "gmMissionClear: DesignID must be a positive numeric id". The command is documented as working (`docs/commands.md:371`, `/gmmissionclear 1001`) and is the only documented slash route to abandon a mission (`gmMissionAbandon`, index 120, has no slash binding; `gmMissionClearActive` is not implemented). Any UAT or GM workflow that needs to abandon a mission from a real client is blocked. @Cadacious

## Evidence
- Server (SigNoz cimmeria-server, lab_run dr-lab, character `Uat Fxdhajaffa`, player 110; entity ids 99, 100, 101 across respawns): WARN scope `cimmeria_cell_console::cell::console::gm::missions`, `cmd=gmMissionClear`, `mission_arg=""` at 2026-11-03 14:33:00.975, 14:36:23.400, 14:36:28.765, 14:38:28.859 (attempts 1, 2, 3). The text typed each time was `/gmmissionclear 1324`.
- The WSTRING is well formed (zero characters), not malformed: the "malformed DesignID WSTRING" branch did not fire, so the argument bytes were absent or empty on the wire.
- Other typed GM commands from the same driver path delivered their arguments: `/gmdhd 3` (INT8) in packets SP-DL2 (results\SP-DL2.md, gmDHD gate_addr 3), `/gmgivecash 5` (SP-DCA6-p1), and `/gmmissionassign 686 1` (String + Boolean, "gmMissionAssign: assigning mission" mission_id 686 at 09:05, 09:10, 09:52 UTC) and `/gmgiveitem` (String + Integer). The failing shape is a command with a single String parameter and no further parameters (`/gmgoto`, `/gmsummon` share the shape and are unverified).
- Server side is correct: `parse_mission_id` (crates/cell-console/src/cell/console/gm/missions.rs:38-71) parses a proper WSTRING; tests `gm_mission_clear_saves_the_abandon` and `gm_mission_clear_rejects_non_numeric_design_id` pass with a well-formed argument.
- Client patch: data/client-patches/012-gm-slash-commands/commands.toml:28-30 and InternalSlashCommands.xml:8-10 declare `/gmmissionclear` with one `String DesignID`. docs/analysis/dev-workbench/evidence/DW-25A-E.md says native submission/quoting proof for these slash commands is absent.
- Screenshots: none relevant (the reply was read from chat channel 9: "gmMissionClear: DesignID must be a positive numeric id").
- Packet: SP-DHA1-p1 (results\SP-DHA1-p1.md), row H20a leg 2 BLOCKED by this.

## Acceptance criteria
- With a GM character in the real client, `/gmmissionclear 1324` on an active mission 1324 logs "gmMissionClear: abandoning mission" with mission_id 1324, fires `mission_abandoned`, and the mission leaves the journal and the saved row.
- `docs/commands.md` states the working argument form and, if the cause is a client quirk, the workaround (for example a quoted form, or a second dummy argument).

## Test type
Packet-tap capture on the lab (server_packet_tap_start) while typing the command, to see the bytes of method 110, then a regression test at the layer that is at fault: a cell-console test if the server must accept an alternative encoding, otherwise a client-patch or lab-typing fix verified in the live lab. Real-client check required: yes (this is exactly what the unit tests cannot see).

## Docs to update
`docs/commands.md:371` (usage and any workaround); `docs/analysis/castle-cellblock-rebuild/uat-guide.md:90` if the form changes; `docs/analysis/dev-workbench/evidence/DW-25A-E.md` (native submission note for `/gmmissionclear`); the starter UAT brief for SP-DHA1 (abandon route).

## Client impact
Possibly a client-patch change (slash command definition or parameter type) if the bug is in `012-gm-slash-commands`; a new opcode is not needed. A server-only fix is possible only if the packet tap shows the argument arrives in a different position or encoding.

## Domain advisor
mission-systems-advisor (abandon flow), game-archaeology-specialist (client slash-command parser, `SGWTextCommandMgr`).

## Needs a human for
Maintainer decision if the fix needs a client patch change; otherwise none.

## Implementation plan
1. Run server_packet_tap_start for the lab session, type `/gmmissionclear 1324` and `/gmmissionassign 1324 1`, and read both method frames (index 110 vs 109).
2. If method 110 arrives with a zero-length string, compare the client's argument packing for single-String commands against two-parameter ones (re-lookup skill; docs/protocol/cell-method-dispatch-table.md:540 first, then Ghidra on the slash dispatcher).
3. Fix at the faulty layer (patch XML/TOML, a server-tolerant fallback if a second string arrives, or a documented quoting form), add the test, and update the docs above.
4. Until then, the starter UAT abandon uses a journal/tracker UI route or a console route (see the packet resume hint).

## Abandon probe (runner-DCA-q, 2026-11-09, local lab, character Fxdcatwo, player 105)

Typed `/gmmissionabandon 568` through the chat box accept handler on an active mission 568 (fixture step 1965). It WORKS: at 08:16:08Z the server logged INFO `GM-gated cell method authorized` (method 120, `gmMissionAbandon`), then `gmMissionClear: abandoning mission` with `mission_id` 568, then `Mission abandoned` (`mission_abandoned`, `mission.abandon` span) and `fire_mission_abandoned: matched` actions 3. The chat reply was `gmMissionClear: abandoned mission 568` (channel 9). The argument arrived non-empty.

Diagnosis supported: the commands that deliver their argument (`/gmmissionabandon`, `/gmmissionassign`, `/gmmissiondetails`) declare `ParamName="MissionDesignId"`, while the failing `/gmmissionclear` declares `ParamName="DesignID"` (`data/client-patches/012-gm-slash-commands/InternalSlashCommands.xml:9`). The client appears to bind the typed argument by the parameter name it knows (`MissionDesignId`). Suggested fix: rename the `/gmmissionclear` ParamName to `MissionDesignId` in the XML and `commands.toml`, then retest; until then the starter UAT abandon route is `/gmmissionabandon <id>`.
