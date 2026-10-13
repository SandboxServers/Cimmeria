# HP-M2 probe: take the paid COD item and the payment-mail cash (row 4)

Run by the coordinator on instance `default` once the other driver is done. Lab-driver tools only, 8 tool calls. L = the lease id. Attempt 2 ran with the GateMailCreateWin ("Create Message", subject HPUM4r3b, left open by an earlier step) on top of the inbox; its screenshot lab_screenshot-20261013-025439.214.png shows that window, not the inbox. No mail.* server row exists after 07:53:19Z, so the Take clicks hit the compose window. This probe closes it first.

1. `client_batch {"lease_id":L,"steps":[
 {"op":"lua","id":"close","chunk":"GateMailCreateWin:hide() GateMailReadWin:hide() if not GateMailInboxWin:isVisible() then GateMailMod.onMailboxVisibilityChange(nil, true) end GateMailInboxWin:moveToFront() return GateMailCreateWin:isVisible()..'/'..GateMailReadWin:isVisible()..'/'..GateMailInboxWin:isVisible()"},
 {"op":"wait","ms":1500},
 HDR, CASH, UTC]}`
 (HDR, CASH, UTC as defined in briefs/HP-M2.md.) Expect `false/false/true`. If CreateWin is still true, stop and report.
2. `lab_screenshot {"lease_id":L,"instance":"default"}`: report the path. Expect the inbox with Take attachments button visible and no Create Message window.
3. `client_batch {"lease_id":L,"steps":[{"op":"lua","id":"sel","chunk":"mailSelectNone() mailAddToSelection(5) local n=mailGetHeaderCount() local s={} for i=0,n-1 do local h=mailGetHeaderInfo(i) if h.Selected then s[#s+1]=tostring(h.Id) end end return table.concat(s,',')"}]}` expect `5`.
4. `client_window_click {"lease_id":L,"target":"GateMailInbox_TakeAttachmentsButton"}`. Record native_pass and the level.
5. `client_batch`: wait 2500, HDR, CASH, UTC.
6. `client_inventory {"lease_id":L}`: record whether item type 2893 count rose by one.
7. Repeat 3 and 4 with `mailAddToSelection(6)` (payment mail, cash 200), then `client_batch`: wait 2500, HDR, CASH, UTC.
8. `lab_screenshot` once more; report path.

Grade (coordinator, server side): mail.item_taken type_id 2893 right after the step-4 click and mail.cash_taken cash 200 after the step-7 click, naquadah +200 (CASH 775 to 975), DB: escrow for mail 5 gone. If the clicks reach the server and nothing is taken, or a refusal row appears, the bug is in the server: re-open the issue plan below. If no mail.* row appears with the compose window verifiably closed, the lab click path is at fault (see the lab issue).
