---
name: reference_lab_driving_recipes_2026_10_12
description: Lab driving recipes and traps proven in the 2026-10-12 Haiku UAT round - teleport-then-click at 3.5-4.5 m, pitch_counts -650, held keys, native tools first (updated 2026-10-12), Lua moveItem/useItem/setVendorSelectedQuantity as N3 fallbacks, HP staging, chat capture, ClickHouse grading, build-lane-idle start
metadata:
  type: reference
---

From the Haiku-driven colo UAT round 1 (2026-10-12; bank, consumables, ability trees on three parallel clients). Full write-up and evidence: `docs/analysis/haiku-lab-uat/README.md` (section "Proven driving recipes"); lab bugs are tracked in issue #1660.

- **Start clients only with the build lane idle.** Under heavy cargo/sccache load on C: the client hung before its window 3/3; idle, it logged in in about 2 s. Kill `Ribbons.scr` first after a long idle (the D3D preflight refuses).
- **Teleport, then click; never `client_move_to` in the stasis-room hub** (gets stuck, drifts 15-35 m). Stand 3-4 m from the NPC: 6.4 m opens nothing, about 3.4 m puts your own avatar in the camera line.
- **Camera pinned at the pitch clamp** after failed face attempts: `client_camera {"pitch_counts": -650}`, then face again. `client_input_mouse` ignores x/y: `client_cursor_move` first.
- **Walk with held keys** (`{"do":"key","key":"s","hold_ms":2500}`), N1; `client_move_to` now steers with turn keys but still avoid it in the hub.
- **Native tools now (updated 2026-10-12, LH-01..04, LH-19, LH-20; the original round-1 notes said the opposite):** `client_item_action` use/equip/unequip right-clicks the slot at N1 and reports `native_pass` only when the inventory changed (`expect_change:false` is a negative check and reports its single click's N1 tier); `client_vendor_buy` sets the spinner with clicks and presses Buy at N1; `client_drag_drop` split and drag start natively and land at N3 by design; `client_move_to` steers with turn keys; covered windows are raised automatically before a click. Try these first.
- **Lua fallbacks, only if a native tool fails:** `moveItem(srcC, srcSlot, dstC, dstSlot, qty)` (1-based; Main 1, Crafting 15, Vault 17); `useItem(1, <slot>)`; `setVendorSelectedQuantity(row0, n)` then click `Vendor_BuyButton`; `getUnitStat(Unit.Player, Stat.X)` (`client_player_state` stats is empty); `buyTrainable(id)`. All N3. Main-slot drags failed while another inventory tab was active in round 1.
- **Lowering your own HP:** `/gmsethealth` cannot target self (client sends -1). Proven: X-tier `gotoxyz 178 -8.4 9` beside Ba'al's Jaffa in SGC_W1, out at 150-300 HP. `.dummy caster 1354` is unconfirmed (one run saw no HP drop).
- **Chat:** type into `Inst1Chat_Input`, check `getText()`, Enter; read only with `client_chat_log` and a named cursor; repeat once if no reply (capture can miss one).
- **Grading:** `server_log_tail` is flooded by client telemetry during a lab run; grade from ClickHouse (`toInt64(attributes_number['player_id'])`) when the SigNoz MCP is down. GM reply text exists only in the client chat log.
- **Agents:** prefer the `lab-driver` agent (no memory) over a general-purpose Haiku, which refuses a lent lease under the user's ask-before-lab memory. `lab-driver` lacks `client_item_action`, `client_chat_log`, `client_die_and_respawn`, `client_type_text` (#1660). New SGU characters spawn in SGC_W1, not the hub.

Related: [[reference_lab_client_controls]], [[reference_lab_parallel_clients_2026_10_10]], [[reference_lab_mcp_token_cost_2026_10_10]].
