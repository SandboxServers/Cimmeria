server/client-patch: /gmmissionclear still reaches the server with an empty DesignID (starter-praxis UAT SP-DTO4-p2)

## Problem

A client-typed `/gmmissionclear 568` sends cell method 110 with an empty WSTRING DesignID, so `parse_mission_id` rejects it and the abandon never runs. This is a second sighting of the defect in SP-DHA1-p1-1 (check whether that issue is already filed and attach this evidence to it instead of opening a duplicate).

## Evidence

- Server (local dr-lab server.log, character Fxdcatwo, player 105): INFO `GM-gated cell method authorized` then WARN `GM mission cmd: DesignID is not a positive numeric mission id` with `mission_arg=""` at 2026-11-09 08:06:07Z and 08:07:03Z. No "malformed DesignID WSTRING" WARN, so the WSTRING decodes and is empty.
- Driver quote: chat reply `gmMissionClear: DesignID must be a positive numeric id` (seq 4753531 and 4754957). The text sent was `/gmmissionclear 568` via the chat box accept handler.
- Code: `crates/cell-console/src/cell/console/gm/missions.rs` `parse_mission_id` is correct for a non-empty numeric string; the empty value arrives from the client side.
- Docs: `docs/commands.md:371` lists `/gmmissionclear <missionId>` as working; `docs/analysis/dev-workbench/evidence/DW-25A-E.md` says native submission proof for it is absent. Multi-parameter commands (`/gmmissionassign 686 1`) deliver their arguments.

## Acceptance criteria

A GM in the real client types `/gmmissionclear 568` on an active mission 568: the server logs `gmMissionClear: abandoning mission` with mission_id 568, the chat reply is `gmMissionClear: abandoned mission 568`, and the mission leaves the journal and the saved row.

## Test type

Live lab UAT with a packet tap on cell method 110 (compare the frame with a working `/gmmissionassign` frame), plus a wire-level unit test of the single-String slash command argument if the cause is on the server decode side.

## Docs to update

`docs/commands.md:371`, `docs/analysis/dev-workbench/evidence/DW-25A-E.md`, and the starter UAT briefs that use `/gmmissionclear` as the abandon route.

## Client impact

Unknown until the method frame is captured; free if the server can accept the frame as sent, otherwise needs a client patch (`data/client-patches/012-gm-slash-commands`).

## Domain advisor

mission-systems-advisor

## Needs a human for

Nothing; a lab packet-tap capture decides it.


## Update (runner-DCA-q, 2026-11-09)
See SP-DHA1-p1-1.md abandon probe: `/gmmissionabandon 568` works from the same chat path (ParamName MissionDesignId); `/gmmissionclear` (ParamName DesignID) is the only failing one. Attach to SP-DHA1-p1-1 rather than filing a duplicate.
