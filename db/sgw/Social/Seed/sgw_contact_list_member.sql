--
-- Members for the seeded dev-character contact lists.
--
-- Each dev character (player_ids 62–68) gets:
--   - 'Friendly' in their Friends list
--   - 'Annoying' in their Ignore list
--
-- list_ids 1–7 = Friends, 8–14 = Ignore (matching sgw_contact_list.sql order).
--

-- Friends list members
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (1,  'Friendly');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (2,  'Friendly');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (3,  'Friendly');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (4,  'Friendly');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (5,  'Friendly');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (6,  'Friendly');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (7,  'Friendly');

-- Ignore list members
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (8,  'Annoying');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (9,  'Annoying');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (10, 'Annoying');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (11, 'Annoying');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (12, 'Annoying');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (13, 'Annoying');
INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES (14, 'Annoying');
