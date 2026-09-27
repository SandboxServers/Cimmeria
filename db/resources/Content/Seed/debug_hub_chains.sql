-- NEW CONTENT (debug hub): content chains for the Castle_CellBlock stasis-room
-- debug hub (templates 300-304, spawnlist 400-404). docs/content/debug-hub.md
-- describes what each NPC tests.
--
-- Two of the five NPCs are chain-driven; the vendor, the trainer and the loot
-- crate go through the built-in interaction paths and need no chain.
--
--   Dialog NPC  (tag DebugHub_DialogNpc)        chains 7001-7003
--   Livewire    (tag DebugHub_LivewireTerminal) chains 7004-7005
--   Mail clerk  (tag DebugHub_MailClerk)        chains 7010-7011 (SS-U3)
--
-- Chain ID range: 7001-7099 is reserved for the debug hub. The highest id in
-- any other chain file is 6528.
--
-- Neither tag has a `set_interaction_type` anywhere: the cursor bit is the
-- template default (302 INT_NonAStoryMissionAvaliable, 303
-- INT_MinigameLivewire) and never changes, because both NPCs are reusable
-- and belong to no mission. Both interact chains are therefore allowlisted in
-- crates/content-engine/tests/it/interact_tag_linter.rs.
--
-- No chain carries a condition. The hub is for testing, so every click works
-- every time, for every character, at any point in the Cellblock missions.
--
-- Visible feedback. `system_message` is a log-only stub (no wire format), so
-- the "it worked" lines use `npc_bark`, which speaks one `dialog_screens` line
-- into the player's chat through onPlayerCommunication. Screens 200003 and
-- 200004 exist only to hold those lines.

SET search_path = resources, pg_catalog;

-- ============================================================
-- Dialog round trip: click -> dialog A -> button -> dialog B -> close
-- ============================================================
--
-- Exercises both ways the client answers a dialog (client contract F8):
--   * 60100 has one button, on its final screen. Clicking it sends
--     dialogButtonChoice(60100, 8) and fires chain 7002. Closing it with X
--     sends nothing, so nothing happens, and a second click on the NPC starts
--     over.
--   * 60101 has no buttons, so closing it sends dialogButtonChoice(60101,
--     -1), which fires chain 7003.
-- A `dialog_choice` chain matches on the dialog id only; no condition can
-- read the button id (F9). One button per dialog is therefore all the branching
-- a chain can see.
--
-- Chain 7002 displays 60101 through the player's `last_interaction_target`
-- pin: a dialog_choice event carries no NPC id, and 60101's screens are
-- spoken by speaker 754, so it is not a monologue. That makes this dialog a
-- live check of the pin written before chain dispatch.

-- Chain 7001: click Airman Lance -> dialog 60100.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7001, 'Debug Hub - click the dialog NPC: display dialog 60100', 'space', 12, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7001, 'interact_tag', 'DebugHub_DialogNpc', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7001, 'display_dialog', 60100, NULL, '{}', 0, 0);

-- Chain 7002: 60100's button -> dialog 60101.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7002, 'Debug Hub - dialog 60100 button: display dialog 60101', 'space', 12, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7002, 'dialog_choice', '60100', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7002, 'display_dialog', 60101, NULL, '{}', 0, 0);

-- Chain 7003: 60101 closed (-1) -> confirmation line in chat.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7003, 'Debug Hub - dialog 60101 closed: confirm in chat', 'space', 12, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7003, 'dialog_choice', '60101', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7003, 'npc_bark', NULL, NULL, '{"screen_id": 200003, "speaker": "Airman Lance", "channel": "say"}', 0, 0);

-- ============================================================
-- Livewire round trip: click -> minigame -> win -> chat line
-- ============================================================
--
-- Same shape as the Cellblock Livewire pairs (1016/1017, 1041/1042,
-- 1060/1061), minus the mission gate and the interaction-bit clear: the
-- terminal can be hacked again and again. Default difficulty (1).

-- Chain 7004: click the terminal -> start Livewire; a win fires chain 7005.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7004, 'Debug Hub - click the Livewire terminal: start Livewire', 'space', 12, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7004, 'interact_tag', 'DebugHub_LivewireTerminal', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7004, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [7005]}', 0, 0);

-- Chain 7005: Livewire won -> confirmation line in chat.
-- No trigger row: the minigame callback invokes it directly by id.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7005, 'Debug Hub - Livewire victory: confirm in chat', 'space', 12, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7005, 'npc_bark', NULL, NULL, '{"screen_id": 200004, "speaker": "Terminal", "channel": "say"}', 0, 0);

-- ============================================================
-- Gate Mail Clerk (social-systems SS-U3): click -> dialog -> button -> mail
-- ============================================================
--
-- Template 390 (tag DebugHub_MailClerk), spawn 490. Lets a tester without GM
-- rights exercise take-cash and take-item: the button mails the clicking
-- player a stack of 5 Health Slappack TC1 (item 2893, max stack 10) and 50
-- naquadah, from "Gate Mail Clerk". COD needs a player sender, so it stays
-- with the GM `.mail ... cod <n>` command and the two-player UAT.
--
-- The limit is at most one mail per player per 10 minutes (`cooldown_secs`
-- 600). The base keeps it in sgw_player_content_cooldown, keyed
-- `send_system_mail/7011`, and claims it in the mail's own transaction, so it
-- holds across a relog, a restart and the mail's deletion. Every press gets a
-- chat line from the base: the mail id and what it carries, or how long to
-- wait.
--
-- Same shape as the dialog round trip: 7010's cursor bit is template 390's
-- default (INT_NonAStoryMissionAvaliable), so it is allowlisted in
-- interact_tag_linter.rs. Closing 60104 with X sends nothing (it has a
-- button), so nothing is mailed.

-- Chain 7010: click the clerk -> dialog 60104.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7010, 'Debug Hub - click the Gate Mail Clerk: display dialog 60104', 'space', 12, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7010, 'interact_tag', 'DebugHub_MailClerk', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7010, 'display_dialog', 60104, NULL, '{}', 0, 0);

-- Chain 7011: 60104's button -> one system mail to the clicking player.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (7011, 'Debug Hub - Gate Mail Clerk button: send the test mail', 'space', 12, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (7011, 'dialog_choice', '60104', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (7011, 'send_system_mail', NULL, NULL, '{"sender": "Gate Mail Clerk", "subject": "Gate Mail test delivery", "body": "A test mail from the stasis-room debug hub. Take the naquadah and the Health Slappacks from this mail. The Gate Mail Clerk can send you another one in 10 minutes.", "cash": 50, "item_id": 2893, "qty": 5, "cooldown_secs": 600}', 0, 0);
