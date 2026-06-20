--
-- Seed contact lists for all dev characters (player_ids 62–68).
--
-- Every dev character gets:
--   - A "Friends" list (flags=300, per EMoniker 300) containing 'Friendly'
--   - An "Ignore" list (flags=301, per EMoniker 301) containing 'Annoying'
--
-- The tester account characters (Friendly / Annoying, player_ids 69–70) are
-- intentionally NOT seeded here — their lists are created server-side on first
-- login like any new character.
--
-- list_id assignment: 1–7 Friends lists (one per player 62–68),
--                     8–14 Ignore lists.
-- Members immediately follow in sgw_contact_list_member.sql.
--

-- Friends lists
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (1,  62, 'Friends', 300);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (2,  63, 'Friends', 300);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (3,  64, 'Friends', 300);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (4,  65, 'Friends', 300);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (5,  66, 'Friends', 300);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (6,  67, 'Friends', 300);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (7,  68, 'Friends', 300);

-- Ignore lists
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (8,  62, 'Ignore', 301);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (9,  63, 'Ignore', 301);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (10, 64, 'Ignore', 301);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (11, 65, 'Ignore', 301);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (12, 66, 'Ignore', 301);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (13, 67, 'Ignore', 301);
INSERT INTO sgw_contact_list (list_id, player_id, name, flags) VALUES (14, 68, 'Ignore', 301);

--
-- Advance sequence past the seeded rows.
--

SELECT setval('sgw_contact_list_list_id_seq', (SELECT COALESCE(MAX(list_id), 1) FROM sgw_contact_list), true);
