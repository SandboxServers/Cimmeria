-- Dakara_E1 (world 61) space chains: arrival, travel and the notice.
-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet prefix DK-).
--
-- Chain ID range: 8001-8039 of the campaign's 8001-8499 block
-- (work-packets.md, "Worker Input And Ownership"). The mission chains live
-- in dakara_e1_arrival_chains.sql (8040-8159), dakara_e1_betrayal_chains.sql
-- (8160-8339) and dakara_e1_climax_chains.sql (8340-8489).
--
--   DK-01  8001  Free Jaffa arriving on Dakara_E1 learns the Omega Site address
--   DK-01  8002  Free Jaffa arriving on Dakara_E1 gets the arrival notice, once
--
-- Evidence labels are the ledger's (README.md, "Evidence Labels"). Nothing
-- in this file restates recovered data: the client holds no script, offer
-- rule or trigger for world 61 (audit.md), so every row here is
-- PROJECT_FINAL_RECONSTRUCTION or NEW CONTENT and says which.
--
-- Both DK-01 chains share one trigger and one gate on purpose:
--
--   * `player_loaded` with event_key 'Dakara_E1'. The key is the world
--     filter: `fire_player_loaded` takes the world name from the base's own
--     world-entry state (never from a client payload) and the trigger
--     matches it exactly. It fires once per world entry: a login on
--     Dakara_E1 or an arrival there from another world.
--   * `archetype eq 7`. 7 is ARCHETYPE_Sholva in EArchetype
--     (entities/defs/enumerations.xml): the SGU Jaffa of char_defs 8 and 18,
--     which the ledgers call the Free Jaffa. It is not ARCHETYPE_Jaffa (8),
--     the Praxis Jaffa. The archetype is stamped on the cell entity from the
--     character's database row. `eq` fails closed when it is missing (the
--     evaluator reads a missing archetype as -1). A Human (1 to 4), an
--     Asgard (5), a Goa'uld (6) or a Praxis Jaffa (8) who visits Dakara_E1
--     gets nothing from either chain.
--
-- Neither chain has a mission gate, so each is safe to fire on every entry
-- only because its action is idempotent by itself:
--
--   * `grant_stargate_address` does nothing for an address the player
--     already holds (no write, no client method, no base round trip).
--   * `send_system_mail` claims a persisted per-player cooldown in the
--     mail's own transaction; see chain 8002.
--
-- The two chains are siblings on one event, and the engine runs every
-- matching chain. That is intended here: their gates are identical and their
-- actions do not depend on each other.

SET search_path = resources, pg_catalog;

-- ============================================================
-- DK-01: the way out
-- ============================================================
--
-- PROJECT_FINAL_RECONSTRUCTION. A new character's address book is empty
-- (`base::character_create` does not name `known_stargates`), and the only
-- grant paths are a committed gate arrival and this verb. Class Start v6
-- (CS-02, PR #1273) starts the Free Jaffa on Dakara_E1 with nothing to dial,
-- so this chain is what lets them leave. In the client's own story the Omega
-- Site address is handed out in mission 1655; granting it on arrival is this
-- project's decision.
--
-- OD-DK02 (PROPOSED, recommended default): Omega Site only. `target_id` is
-- `resources.stargates.stargate_id`: 5 = Omega Site (world 18), which has a
-- DHD (spawnlist 41), so the trip is two-way: the arrival unlock teaches
-- gate 25 (Dakara E1) on landing at Omega Site, and Omega's DHD then lists
-- it. To change the decision, change this one row's target_id (or add a
-- second action row for "both"): gate 27 (SGC W1) is the other candidate and
-- is a one-way trip today (no DHD on world 58, B-DK4).
--
-- Guards: `chain_replay_tests/dakara_e1_space.rs` (resolution per archetype
-- and world, the executed grant and its idempotence) and
-- `gate_dakara_e1_tests.rs` in cimmeria-cell-world (every address a
-- `player_loaded` chain grants has a way back).

-- Chain 8001: player_loaded Dakara_E1, archetype 7 -> grant Omega Site.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8001, 'Dakara_E1 - Free Jaffa arrival: learn the Omega Site address (DK-01)', 'space', 61, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8001, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (8001, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (8001, 'grant_stargate_address', 5, NULL, '{}', 0, 0);

-- ============================================================
-- DK-01: the arrival notice
-- ============================================================
--
-- NEW CONTENT. The sender, the subject and the body are written by this
-- project and have no recovered counterpart. World 61 has no mission, no
-- NPC and no dialog for a level-1 Free Jaffa yet (OD-DK01, option A), so the
-- notice says so and says how to leave and come back. No cash, no item.
--
-- Once per character: `cooldown_secs` is 2147483647 (`i32::MAX`, about 68
-- years, the longest window the claim honours), and the claim is a row in
-- `sgw_player_content_cooldown` under key `send_system_mail/8002`, written
-- in the mail's own transaction. It survives a relog, a server restart and
-- the player deleting the mail.
--
-- `quiet_cooldown`: without it the base answers every firing inside the
-- window with "<sender> has already sent you mail. You can ask again in
-- ...", which is right for a button and wrong here, where the chain fires at
-- every world entry and the player pressed nothing. With it, a firing inside
-- the window sends the player nothing and logs at DEBUG.
--
-- The first firing is announced like any system mail: the line
-- "Dakara Gate Watch sent you mail <id>. Open your mail to read it." and the
-- new-mail notification.

-- Chain 8002: player_loaded Dakara_E1, archetype 7 -> the arrival notice.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8002, 'Dakara_E1 - Free Jaffa arrival: one-time arrival notice by mail (DK-01)', 'space', 61, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8002, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (8002, 'archetype', NULL, NULL, 'eq', '7', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (8002, 'send_system_mail', NULL, NULL, '{"sender": "Dakara Gate Watch", "subject": "Dakara: no orders yet", "body": "The Free Jaffa command on Dakara is not staffed yet, so there are no orders for you here. The DHD in front of the Stargate dials Omega Site. The DHD at Omega Site brings you back to Dakara.", "cooldown_secs": 2147483647, "quiet_cooldown": true}', 0, 0);
