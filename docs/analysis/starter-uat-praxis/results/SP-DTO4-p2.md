# SP-DTO4-p2 result: Tollana mission 568 abandon rows U5a and U5b

Verifier: server evidence only (local dr-lab `logs/server.log`, UTC, character Fxdcatwo, player 105; the packet ran on instance p2 with this reused character, per the run note). No screenshots were viewed and no looker was run: the abandon never happened, so the after-abandon marks have nothing to grade. No DB read was taken; the log lines below carry the state changes.

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| U5a | BLOCKED | G (`/gmmissionclear` via chat accept handler) | Fixture: INFO `lab fixture applied` 08:04:38Z (missions 1, moved true); in-progress bind `adding dialog set` 4959 at 08:04:48Z. Bookmarks `U5a start` 08:06:01Z, `U5a abandon` 08:06:05Z. At 08:06:07Z: INFO `GM-gated cell method authorized`, then WARN `GM mission cmd: DesignID is not a positive numeric mission id` with `mission_arg=""` (empty). No `gmMissionClear: abandoning mission`, no `removing dialog set` 4959, no `adding dialog set` 2656 in the window. No `Content: accepting mission` / `Mission accepted` at `U5a reaccepted` 08:06:35Z (the DIALOG step closed the in-progress dialog, not 2153). Mission 568 did not change state. | not viewed | results/issues/SP-DTO4-p2-1.md (recurrence of SP-DHA1-p1-1) |
| U5b | BLOCKED | N1 (Core click) then G | Core click did advance 1965 to 1966: `Content: advancing step` mission 568 step 1966 at 08:06:55Z, `removing dialog set` 4959 and `adding dialog set` 6393 right after. `U5b abandon` bookmark 08:07:01Z; at 08:07:03Z the same WARN with `mission_arg=""`. No abandon row, 6393 not removed by an abandon, no 2656 repaint. At 08:07:39Z (`U5b reaccepted` bookmark) the driver's click on Copplemann plus DIALOG closed the turn-in dialog: `removing dialog set` 6393, `Content: completing mission` 568 and `Mission completed directly` (player 105). No accept lines anywhere in the packet. | not viewed | results/issues/SP-DTO4-p2-1.md |

Both rows are BLOCKED per the brief: `/gmmissionclear 568` had no effect, so the abandon and re-accept clauses never ran. The "Core to 1966" half of U5b did hold on the server (step 1966, 4959 swapped for 6393), but the row's pass criteria all depend on the abandon.

## Not run

- U5a: abandon from step 1965, the 2656 repaint, the offer mark, the 2153 re-offer, and the re-accept to step 1964 with `repeats` 0. Reason: `gmMissionClear` received an empty DesignID.
- U5b: abandon from step 1966, 6393 removal, the 2656 repaint, the second re-accept to step 1964. Same reason.
- Looker questions for `DTO4-U5a-after-abandon` and `DTO4-U5b-after-abandon-1966`: not run (nothing to see; the driver also reported Copplemann off screen, on_screen false, in both shots).
- Journal Abandon button (cell method 52): untested, as the brief expects. Put it on the sign-off sheet as untested.
- Known gaps from the brief (not findings): double-click and journal (SP-DTO4-p1), turn-in and silent completed state (SP-DTO4-p3), first-offer dialog text, prop dialogs, relog rows (SP-DTO3-p1 to p4), phasing rows U-PH1 to U-PH5.

## Resume hint

Mission 568 on Fxdcatwo (player 105) is now COMPLETED (08:07:39Z, side effect of the driver closing the turn-in dialog 2154 at step 1966). A re-run needs a fresh fixture (`server_stage_fixture` mission 568 step 1965) on a character that is offline, or a different character. Do not use `/gmmissionclear` as the abandon route until the empty-argument defect is fixed; an abandon needs another route (the journal Abandon button, cell method 52, which no brief has traced, or a server-side or DB-level abandon staged by the coordinator).

## Recipe

- Window: `Select-String` on `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\logs\server.log` for `2026-11-09T08:0[4-8]`, filtered on `SPU SP-DTO4|gmMission|GM mission|accepting mission|Mission accepted|completing mission|advancing step|cache miss|Content: (adding|removing) dialog`. Timestamps are UTC. The bookmark text is in the `note` field.
- No `dialog_set_maps cache miss` WARN appeared. The only WARNs in the window are the two `GM mission cmd` rejections.

## Findings

1. U5a and U5b: `/gmmissionclear 568` typed through the chat accept handler reaches the server as an empty WSTRING (`mission_arg=""`) both times (08:06:07Z, 08:07:03Z), so `parse_mission_id` rejects it and the handler replies `gmMissionClear: DesignID must be a positive numeric id`. Same defect as SP-DHA1-p1-1 (`mission_arg=""` three times on 2026-11-03). Cause (code read, `crates/cell-console/src/cell/console/gm/missions.rs`): `parse_mission_id` calls `read_wstring(args, 0)` and requires `trim().parse::<i32>()` to give an id above 0. The WSTRING decodes fine (no "malformed" WARN) but is empty, so the failure is upstream of the parser: the client's single-String slash command sends the method with an empty DesignID instead of "568". The parser is behaving as written. Multi-parameter commands such as `/gmmissionassign 686 1` do deliver their arguments, so the single-String shape is the suspect.
2. U5b step 5b.5: the driver's DIALOG step closed the turn-in dialog 2154 and completed mission 568 (`Content: completing mission`, 08:07:39Z). That is the brief's own hazard, not a server defect, but it consumed the fixture.
3. U5a step 5a.4 and 5a.5: with no abandon, the click on Copplemann showed the in-progress dialog (4959), not 2153, as the server state implies. Consistent with the log; not a defect.

## Sign-off sheet

| Row | What to look at | Screenshot path | Driver's words | Looker pre-screen | Your read of the numbers |
|---|---|---|---|---|---|
| U5a | Offer mark back above Copplemann after abandon | lab_screenshot-20261109-020615.636.png (not viewed) | "Mark not observable", on_screen false | not run | Abandon never ran, so no mark change is expected; nothing to sign off |
| U5b | Offer mark back, no turn-in mark, after abandon from 1966 | lab_screenshot-20261109-020714.389.png (not viewed) | Copplemann on_screen false | not run | Abandon never ran; nothing to sign off |
| U5a/U5b | Journal Abandon button (cell method 52) | none | not driven | not run | Untested |
