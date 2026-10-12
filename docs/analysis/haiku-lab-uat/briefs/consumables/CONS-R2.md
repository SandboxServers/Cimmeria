<!-- markdownlint-disable MD029 -- step numbers run on across sections and are cited by number -->

# CONS-R2: consumables I3, I4, I5, I9, I10, I10b (lab-driver briefs)

Shared (put at the top of every driver brief)

- Instance p2, character Labtwo, lease <LEASE_ID> on every call. Touch only Labtwo.
- Driver tools only: client_batch (lua steps), client_ui_sequence, client_chat_log, client_inventory, client_player_state, client_entity_find, lab_screenshot, lab_ensure_in_world, lab_logout.
- USE AN ITEM: client_batch lua step  `useItem(1, <slot>)`  where <slot> is the slot number client_inventory prints for the item (it prints 1-based; the PROVEN call was `useItem(1, 2)` for the Health Slappack printed as slot 2 in Main). Orchestrator's note says `useItem(Container.Main, <0-based>)`; I did not test that form. If `useItem(1, <printed slot>)` consumes nothing and sends nothing, report it and try the other form once. Find the item by exact name in client_inventory {containers:["Main"]}; never assume the slot.
- client_item_action does NOT reliably send the use: at HP 209 it reported native_pass and nothing reached the server. Do not use it as evidence either way.
- Read HP/focus: client_player_state (fields ["stats"] came back empty for me; use verbose or read the HUD with lab_screenshot). Orchestrator can read exact values with server stats (health/focus cur/max) between steps.
- Chat text: client_chat_log with a fresh cursor name per brief; set the cursor first, act, then read.
- Orchestrator pre-stages items (/gmgiveitem needs chat typing, which the driver lacks). The driver only verifies the staged state and reports raw values.

LOWERING HP (what was observed, not what is assumed)

- `.dummy caster 1354` (typed in chat as `.dummy caster 1354`): places a dummy that casts Disabling Shot (1354) at you every 8 s default. An interval of 4 is REFUSED: ".dummy caster: Disabling Shot (1354) has a 5 s cooldown and a 4 s warmup; use an interval of at least 5 s". In my run it did not change Labtwo's health in ~10 s (760/760 stayed); Disabling Shot is a debuff. So verify HP actually drops before relying on it, or the orchestrator uses the fallback below.
- Fallback that did work (orchestrator, server_console_exec entity 4): `gotoxyz 178 -8.4 9` (next to the Ba'al's Jaffa in SGC_W1). About 4 s later FOCUS starts draining (1570 to 0 in about 8 s), then HEALTH falls about 100 per second once focus is 0. Poll `stats`; at health 150-300 send `gotoxyz 201.5 1.4 49.7` to return. Overshoot kills (first try did, 12 s); death respawns at 760/760.
- Slappack heals +500 capped at max; any HP at or below 260 shows the full +500, so exact 100 is not needed.

I3 full-HP use

1. State: HP 760/760, slappacks in Main (2). client_inventory snapshot "i3a".
2. Set chat cursor "i3". client_batch: lua `useItem(1, <slot>)`.
3. client_chat_log cursor "i3": expect SYSTEM line "You are already at full health." (seen verbatim at full HP, followed by a line "CONDITION_FEEDBACK_StatValueGreaterThanOrEqual ").
4. client_inventory diff_against "i3a": stack unchanged (2). Server: `consumable_use` "item use refused: it would do nothing; nothing consumed", stat_cur 760 stat_max 760.

Report: the exact chat lines, stack before/after.

I4 double use guard (needs exactly ONE slappack and HP 100-300)

1. Orchestrator drops the stack to 1 (use one while hurt, as I2), and lowers HP to 100-300 via the methods above.
2. Driver confirms 1 slappack (client_inventory) and HP in range, snapshot "i4a".
3. ONE client_batch with two lua steps back to back (no wait step between): `useItem(1, <slot>)` then `useItem(1, <slot>)`. stop_on_error false.
4. Wait 3 s, client_inventory diff_against "i4a": item gone; HP rose by one heal only (about +500 capped); client_chat_log for any second message.

Expect ONE `consumable_used` row and no second (a second attempt either never sends, is refused, or finds no item). Report HP before/after, exactly how many heal rows (orchestrator reads ClickHouse).

I5 Focus Heal Consumable (item type 6106, orchestrator gives 1 via /gmgiveitem 6106 1)

- At full focus (client_player_state or HUD): useItem once. Chat should show "You are already at full focus."; item stays. Do this part now.
- Lowering focus is BLOCKED for the driver: the only working method seen is standing near the Ba'al's Jaffa (orchestrator `gotoxyz 178 -8.4 9`, focus drains within about 4 to 8 s, health follows) and it needs the server console or typed chat. Orchestrator: teleport in, poll stats, when focus is 100-600 and health still above 400 teleport out (`gotoxyz 201.5 1.4 49.7`; focus stops draining but does not regen), then tell the driver to useItem. Expect focus +384 (or to max) and the item gone; consumable_used row has focus_before/focus_after. If you cannot time it, mark the second half BLOCKED(no safe focus-lowering for the driver).

I9 die, respawn, relog (BLOCK for the lite driver)

- Needs a death (client_die_and_respawn, which the driver lacks) and a relog (lab_logout + lab_ensure_in_world are available). Give I9 to a general-purpose driver that has client_die_and_respawn. Recipe that works: orchestrator `gotoxyz 178 -8.4 9`, wait about 12 s until dead, then client_die_and_respawn (Release; respawn returns at 760/760 at the respawner). Precondition: a stimpack buff already on (I6/I7 item), attribute value read before. Check attributes after respawn (buffs expected still on) and after lab_logout + lab_ensure_in_world (expected back at base, no icon).

I10 mission item must not heal (orchestrator gives item 1893)

1. Orchestrator lowers HP/focus (not full). Driver snapshots "i10a", records HP/focus.
2. Driver: useItem(1, <slot of the item named Opheltes's Injection>).
3. After 3 s: client_inventory diff_against "i10a": item still there; HP/focus unchanged; client_chat_log shows no heal feedback.

I10b Stealth Boost Consumable (item 6206)

1. Snapshot "i10b". Set chat cursor. useItem(1, <slot>).
2. Chat shows "This item has no effect yet."; item stays (diff empty). Expected K21 behavior; server row reason consumable_not_implemented.

Open items / caveats

- client_item_action 'use' vs Lua useItem: only Lua is proven to send the message (08:09:40 consumable_used, health_before 210, health_after 710, instance 10299, ability 648).
- gmsethealth from the client can never target self (client sends target -1), so I1 as literally written cannot run.
