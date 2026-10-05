-- NEW CONTENT (Debug Area, DA-02): content chains for the services plaza of
-- world 1300 `DebugArea` (spawnlist_debug_area_plaza.sql). docs/content/
-- debug-area.md describes what each NPC tests.
--
--   Ability granter  (tag DebugArea_AbilityGranter)   chain 13000
--   Ability reset    (tag DebugArea_AbilityReset)     chain 13001
--   Dialog NPC       (tag DebugArea_DialogNpc)        chains 13002-13003
--   Livewire         (tag DebugArea_LivewireTerminal) chains 13004-13005
--   Loot crate       (tag DebugArea_LootCrate)        chain 13006
--   Mail clerk       (tag DebugArea_MailClerk)        chain 13007
--   Auctioneer       (tag DebugArea_Auctioneer)       chain 13008
--
-- Chain ID range: 13000-13099 is DA-02's, the same numbers as its spawn
-- block. The vendors, trainers, Bankers, registrars and crafting stations go
-- through the built-in interaction paths and need no chain.
--
-- Tags fire chains in any world (`scope_id` is a label), which is why every
-- plaza NPC has its own `DebugArea_*` tag rather than the hub's.
--
-- No tag has a `set_interaction_type` anywhere: each cursor bit is its
-- template's default and never changes, because these NPCs are reusable and
-- belong to no mission. The interact chains are allowlisted in
-- crates/content-engine/tests/it/interact_tag_linter.rs. No chain carries a
-- condition: every click works every time. The ability chains' GM gate is in
-- the `gm_ability_bulk` action itself, so a non-GM's click is answered with a
-- refusal line rather than dropped by a condition.

SET search_path = resources, pg_catalog;

-- ============================================================
-- Ability granter and ability reset (GM only)
-- ============================================================
--
-- `gm_ability_bulk` does from an NPC what /gmgiveallabilities (grant_all) or
-- /gmresetabilities (reset) does: the same plan, the same base write, the
-- same onKnownAbilitiesUpdate burst (the Abilities window refreshes) and
-- result line, after clearing every running cooldown. A non-GM gets
-- "Only a GM can use this." and nothing changes.

-- Chain 13000: click the ability granter -> every ability of your tree.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13000, 'Debug Area - click the ability granter: every ability of your archetype tree (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13000, 'interact_tag', 'DebugArea_AbilityGranter', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13000, 'gm_ability_bulk', NULL, NULL, '{"change": "grant_all"}', 0, 0);

-- Chain 13001: click the ability reset NPC -> back to the starters.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13001, 'Debug Area - click the ability reset NPC: back to your starter abilities (GM)', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13001, 'interact_tag', 'DebugArea_AbilityReset', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13001, 'gm_ability_bulk', NULL, NULL, '{"change": "reset"}', 0, 0);

-- ============================================================
-- Dialog round trip: click -> shipped dialog 5738 -> button -> chat line
-- ============================================================
--
-- The hub's own dialogs (60100, 60101) are quarantined (#943): a client that
-- received them as cooked overrides crashed on map load, so they are not
-- served and the hub's dialog NPC shows nothing. This NPC uses a dialog the
-- client ships instead: 5738, two screens (paging) and one Generic button
-- ("Kill NID Operative", ButtonID 198). No mission, dialog set or chain
-- uses 5738, so its button can fire nothing but chain 13003. It is a
-- mission line out of context; the point is the dialog UI, not the words.
--
-- Clicking the button sends dialogButtonChoice(5738, 198), the server checks
-- 5738 was offered, and chain 13003 barks the hub's confirmation line as
-- Airman Lance. Closing with X sends nothing (a dialog with a button), so
-- nothing happens and a second click starts over.

-- Chain 13002: click Airman Lance -> dialog 5738.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13002, 'Debug Area - click the dialog NPC: display shipped dialog 5738', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13002, 'interact_tag', 'DebugArea_DialogNpc', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13002, 'display_dialog', 5738, NULL, '{}', 0, 0);

-- Chain 13003: 5738's button -> confirmation line in chat.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13003, 'Debug Area - dialog 5738 button: confirm in chat', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13003, 'dialog_choice', '5738', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13003, 'npc_bark', NULL, NULL, '{"screen_id": 200003, "speaker": "Airman Lance", "channel": "say"}', 0, 0);

-- ============================================================
-- Livewire round trip: click -> minigame -> win -> chat line
-- ============================================================

-- Chain 13004: click the terminal -> start Livewire; a win fires chain 13005.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13004, 'Debug Area - click the Livewire terminal: start Livewire', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13004, 'interact_tag', 'DebugArea_LivewireTerminal', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13004, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [13005]}', 0, 0);

-- Chain 13005: Livewire won -> confirmation line in chat.
-- No trigger row: the minigame callback invokes it directly by id.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13005, 'Debug Area - Livewire victory: confirm in chat', 'space', 1300, true, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13005, 'npc_bark', NULL, NULL, '{"screen_id": 200004, "speaker": "Terminal", "channel": "say"}', 0, 0);

-- ============================================================
-- Loot crate: click -> loot window on table 3 (no kill)
-- ============================================================

-- Chain 13006: the hub crate's chain 7020, for this crate's tag.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13006, 'Debug Area - click the loot crate: open a loot window on table 3', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13006, 'interact_tag', 'DebugArea_LootCrate', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13006, 'open_loot', 3, NULL, '{}', 0, 0);

-- ============================================================
-- Gate Mail Clerk: click -> one test mail (no dialog)
-- ============================================================
--
-- The hub clerk mails from his dialog's button (60104), and that dialog is
-- quarantined, so he cannot mail at all. This one mails on the click itself:
-- the same `send_system_mail` contents and 10-minute per-character window,
-- with the base's chat line naming the mail or the wait. The window is keyed
-- by this chain, so it is separate from the hub clerk's.

-- Chain 13007: click the clerk -> one system mail to the clicking player.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13007, 'Debug Area - click the Gate Mail Clerk: send the test mail', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13007, 'interact_tag', 'DebugArea_MailClerk', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13007, 'send_system_mail', NULL, NULL, '{"sender": "Gate Mail Clerk", "subject": "Gate Mail test delivery", "body": "A test mail from the Debug Area services plaza. Take the naquadah and the Health Slappacks from this mail. The Gate Mail Clerk can send you another one in 10 minutes.", "cash": 50, "item_id": 2893, "qty": 5, "cooldown_secs": 600}', 0, 0);

-- ============================================================
-- Black Market auctioneer: click -> open the Black Market
-- ============================================================
--
-- The hub auctioneer's chain 5030 binds the tag `BlackMarket_Auctioneer`;
-- this one binds the plaza auctioneer's own tag. Same template (305), so the
-- same seeded INT_Auction role that `open_black_market` checks.

-- Chain 13008: click the auctioneer -> open the Black Market.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (13008, 'Debug Area - click the Black Market auctioneer: open the Black Market', 'space', 1300, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (13008, 'interact_tag', 'DebugArea_Auctioneer', 'player', false, 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (13008, 'open_black_market', NULL, NULL, '{}', 0, 0);
